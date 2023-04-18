use std::collections::HashMap;
use std::fs::File;
use std::os::unix::prelude::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Result};
use lazy_static::lazy_static;
use psf_ascii::parser::ac::AcData as PsfAcData;
use psf_ascii::parser::transient::TransientData;
use serde::Serialize;
use substrate::verification::simulation::{
    AcData, Analysis, AnalysisData, AnalysisType, ComplexSignal, OutputFormat, Quantity,
    RealSignal, Save, SimInput, SimOutput, Simulator, SimulatorOpts, SweepMode, TranData,
};
use templates::{render_netlist, NetlistCtx};
use tera::{Context, Tera};

pub const TOP_NETLIST_NAME: &str = "sim.top.spice";

lazy_static! {
    pub static ref TEMPLATES: Tera =
        match Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*")) {
            Ok(t) => t,
            Err(e) => panic!("Error parsing templates: {e}"),
        };
}

fn tran_conv(data: TransientData) -> TranData {
    let mut data = HashMap::from_iter(data.signals.into_iter().map(|(k, v)| {
        (
            k,
            RealSignal {
                values: v,
                quantity: Quantity::Unknown,
            },
        )
    }));
    let time = data.remove("time").unwrap();
    TranData { data, time }
}

fn ac_conv(parsed_data: PsfAcData) -> AcData {
    let data = HashMap::from_iter(parsed_data.signals.into_iter().map(|(k, v)| {
        let (real, imag) = v.iter().copied().unzip();
        (
            k,
            ComplexSignal {
                real,
                imag,
                quantity: Quantity::Unknown,
            },
        )
    }));
    AcData {
        data,
        freq: RealSignal {
            values: parsed_data.freq,
            quantity: Quantity::Unknown,
        },
    }
}

pub(crate) mod templates;
#[cfg(test)]
mod tests;

pub struct Spectre {}

struct SpectreOutputParser<'a> {
    raw_output_dir: &'a Path,
}

impl<'a> SpectreOutputParser<'a> {
    fn new(raw_output_dir: &'a Path) -> Self {
        Self { raw_output_dir }
    }

    fn parse_analysis(&mut self, num: usize, input: &SimInput) -> Result<AnalysisData> {
        let analyses = &input.analyses;
        let analysis = &analyses[num];

        // Spectre chooses this file name by default
        let file_name = match analysis.analysis_type() {
            AnalysisType::Ac => {
                format!("analysis{num}.ac")
            }
            AnalysisType::Tran => {
                format!("analysis{num}.tran.tran")
            }
            _ => {
                bail!("spectre plugin only supports ac and transient simulations");
            }
        };
        let psf_path = self.raw_output_dir.join(file_name);
        let psf = substrate::io::read_to_string(psf_path)?;
        let ast = psf_ascii::parser::frontend::parse(&psf)?;
        Ok(match analysis.analysis_type() {
            AnalysisType::Ac => ac_conv(PsfAcData::from_ast(&ast)).into(),
            AnalysisType::Tran => tran_conv(TransientData::from_ast(&ast)).into(),
            _ => bail!("spectre plugin only supports ac and transient simulations"),
        })
    }

    fn parse_analyses(mut self, input: &SimInput) -> Result<Vec<AnalysisData>> {
        let mut analyses = Vec::new();
        if output_format_name(&input.output_format) == "psfascii" {
            for i in 0..input.analyses.len() {
                let analysis = self.parse_analysis(i, input)?;
                analyses.push(analysis);
            }
        }
        Ok(analyses)
    }
}

pub struct Paths {
    pub raw_output_dir: PathBuf,
    pub log_path: PathBuf,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
    pub run_script_path: PathBuf,
    pub top_netlist_path: PathBuf,
}

fn generate_paths(work_dir: impl AsRef<Path>) -> Paths {
    let path = work_dir.as_ref();
    Paths {
        raw_output_dir: path.join("psf/"),
        log_path: path.join("spectre.log"),
        stdout_path: path.join("spectre.out"),
        stderr_path: path.join("spectre.err"),
        run_script_path: path.join("run_sim.sh"),
        top_netlist_path: path.join(TOP_NETLIST_NAME),
    }
}

fn save_directives(input: &SimInput, directives: &mut Vec<String>) {
    match &input.save {
        Save::Signals(s) => {
            directives.reserve(s.len());
            for s in s {
                directives.push(format!("save \"{s}\""));
            }
        }
        Save::All => directives.push("opsaveall options save=all".to_string()),
        Save::None => directives.push("opsavenone options save=none".to_string()),
    }
}

fn temp_directives(input: &SimInput, directives: &mut Vec<String>) {
    if let Some(t) = input.opts.temp {
        directives.push(format!("settemp alter param=temp value={t}"));
    }
    if let Some(t) = input.opts.tnom {
        directives.push(format!("settnom alter param=tnom value={t}"));
    }
}

fn ic_directives(input: &SimInput, directives: &mut Vec<String>) {
    use std::fmt::Write;
    if input.ic.is_empty() {
        return;
    }

    let mut ic = String::from(".ic\n");
    for (name, value) in input.ic.iter() {
        writeln!(&mut ic, "+ {}={}", name, value)
            .expect("out of memory: failed to write initial condition directive");
    }

    directives.push(ic);
}

pub fn run_spectre(input: &SimInput) -> Result<Vec<AnalysisData>> {
    let work_dir = &input.work_dir;
    let paths = generate_paths(work_dir);

    std::fs::create_dir_all(&input.work_dir)?;
    let analyses = get_analyses(&input.analyses);

    let mut spectre_directives = vec!["oppreserveall options preserve_inst=all".to_string()];
    save_directives(input, &mut spectre_directives);
    temp_directives(input, &mut spectre_directives);

    let mut spice_directives = Vec::new();
    ic_directives(input, &mut spice_directives);

    let ctx = NetlistCtx {
        libs: &input.libs,
        includes: &input.includes,
        spectre_directives: &spectre_directives,
        spice_directives: &spice_directives,
        analyses: &analyses,
    };
    render_netlist(ctx, &paths.top_netlist_path)?;

    write_run_script(&paths, input)?;
    let mut perms = std::fs::metadata(&paths.run_script_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&paths.run_script_path, perms)?;

    let out_file = std::fs::File::create(paths.stdout_path)?;
    let err_file = std::fs::File::create(paths.stderr_path)?;

    let status = Command::new("/bin/bash")
        .arg(&paths.run_script_path)
        .stdout(out_file)
        .stderr(err_file)
        .current_dir(work_dir)
        .status()?;

    if !status.success() {
        bail!("Spectre exited unsuccessfully");
    }

    SpectreOutputParser::new(&paths.raw_output_dir).parse_analyses(input)
}

fn output_format_name(format: &OutputFormat) -> &str {
    match format {
        OutputFormat::DefaultReadable => "psfascii",
        OutputFormat::DefaultViewable => "fsdb",
        OutputFormat::Custom(s) => s,
    }
}

#[derive(Debug, Copy, Clone, Serialize)]
struct RunScriptContext<'a> {
    spice_path: &'a PathBuf,
    raw_output_dir: &'a PathBuf,
    log_path: &'a PathBuf,
    bashrc: Option<&'a PathBuf>,
    format: &'a str,
    flags: &'a str,
}

fn flags(input: &SimInput) -> String {
    if let Some(ref flags) = input.opts.flags {
        flags.clone()
    } else if let Ok(flags) = std::env::var("SPECTRE_FLAGS") {
        flags
    } else {
        // Use default flags.
        "-64 +multithread +spice ++aps +error +warn +note".to_string()
    }
}

fn write_run_script(paths: &Paths, input: &SimInput) -> Result<()> {
    let ctx = RunScriptContext {
        spice_path: &paths.top_netlist_path,
        raw_output_dir: &paths.raw_output_dir,
        log_path: &paths.log_path,
        bashrc: input.opts.bashrc.as_ref(),
        format: output_format_name(&input.output_format),
        flags: &flags(input),
    };
    let ctx = Context::from_serialize(ctx)?;

    let mut f = File::create(&paths.run_script_path)?;
    TEMPLATES.render_to("run_sim.sh", &ctx, &mut f)?;

    Ok(())
}

impl Simulator for Spectre {
    fn new(_opts: SimulatorOpts) -> substrate::error::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }

    fn simulate(&self, input: SimInput) -> substrate::error::Result<SimOutput> {
        if input.analyses.is_empty() {
            return Ok(SimOutput { data: Vec::new() });
        }
        let data = run_spectre(&input)?;
        Ok(SimOutput { data })
    }
}

fn get_analyses(input: &[Analysis]) -> Vec<String> {
    input
        .iter()
        .enumerate()
        .map(|(i, analysis)| analysis_line(analysis, i))
        .collect()
}

fn analysis_line(input: &Analysis, num: usize) -> String {
    match input {
        Analysis::Op(_) => format!("analysis{num} dc"),
        Analysis::Tran(a) => {
            let strobe = if let Some(strobe) = a.strobe_period {
                format!(" strobeperiod={strobe}")
            } else {
                String::new()
            };
            format!(
                "analysis{num} tran step={} stop={} start={}{}",
                a.step, a.stop, a.start, strobe
            )
        }
        Analysis::Ac(a) => format!(
            "analysis{num} ac start={} stop={} {}",
            a.fstart,
            a.fstop,
            fmt_sweep_mode(a.sweep, a.points),
        ),
        Analysis::Dc(a) => format!(
            "analysis{num} dc {} start={} stop={} step={}",
            a.sweep, a.start, a.stop, a.step
        ),
    }
}

fn fmt_sweep_mode(mode: SweepMode, points: usize) -> String {
    match mode {
        SweepMode::Dec => format!("dec={points}"),
        SweepMode::Oct => {
            // Oct isn't directly supported by Spectre; use a log sweep instead.
            log::warn!(
                "Unsupported sweep mode `{:?}`; using a log sweep instead",
                mode
            );
            format!("log={points}")
        }
        SweepMode::Lin => format!("lin={points}"),
    }
}
