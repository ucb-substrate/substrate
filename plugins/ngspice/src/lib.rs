use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use spice_rawfile::Rawfile;
use substrate::error::ErrorSource;
pub(crate) use substrate::error::Result;
use substrate::verification::simulation::{
    AcAnalysis, AcData, Analysis, AnalysisData, AnalysisType, DcAnalysis, DcData, OpAnalysis,
    OpData, Quantity, RealSignal, ScalarSignal, SimInput, SimOutput, Simulator, SimulatorOpts,
    SweepMode, TranAnalysis, TranData,
};
use templates::{render_netlist, NetlistCtx};

pub(crate) mod templates;
#[cfg(test)]
mod tests;

pub struct Ngspice {}

impl Simulator for Ngspice {
    fn new(_opts: SimulatorOpts) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }

    fn simulate(&self, input: SimInput) -> Result<SimOutput> {
        std::fs::create_dir_all(&input.work_dir)?;
        let analyses = get_analyses(&input.analyses);
        let directives = get_directives(&input);
        let ctx = NetlistCtx {
            libs: &input.libs,
            includes: &input.includes,
            directives: &directives,
            analyses: &analyses,
        };
        let path = render_netlist(ctx, &input.work_dir)?;
        let rawpath = input.work_dir.join("rawspice.raw");
        let status = Command::new("ngspice")
            .arg("-n")
            .arg("-b")
            .arg("-r")
            .arg(&rawpath)
            .current_dir(&input.work_dir)
            .arg(path)
            .status()?;

        if !status.success() {
            return Err(ErrorSource::Internal("simulator failed".to_string()).into());
        }

        let out = read_rawfile(&input, &rawpath)?;

        Ok(out)
    }

    fn node_voltage_string(
        &self,
        path: &substrate::schematic::signal::NamedSignalPathBuf,
    ) -> String {
        let mut s = String::new();
        s.push_str("v(");
        for inst in path.insts.iter() {
            s.push_str(&inst);
            s.push('.');
        }
        s.push_str(&path.signal);
        if let Some(idx) = path.idx {
            s.push_str(&format!("[{idx}]"));
        }
        s.push(')');
        s
    }
}

fn get_analyses(input: &[Analysis]) -> Vec<String> {
    input.iter().map(analysis_line).collect()
}

fn get_directives(input: &SimInput) -> Vec<String> {
    let mut directives = Vec::new();
    if let Some(t) = input.opts.temp {
        directives.push(format!(".temp {t}"));
    }
    if let Some(t) = input.opts.tnom {
        directives.push(format!(".options tnom={t}"));
    }
    directives
}

fn analysis_line(input: &Analysis) -> String {
    match input {
        Analysis::Op(_) => String::from(".op"),
        Analysis::Tran(a) => format!(".tran {} {} {}", a.step, a.stop, a.start),
        Analysis::Ac(a) => format!(
            ".ac {} {} {} {}",
            fmt_sweep_mode(a.sweep),
            a.points,
            a.fstart,
            a.fstop
        ),
        Analysis::Dc(a) => format!(".dc {} {} {} {}", a.sweep, a.start, a.stop, a.step),
    }
}

fn fmt_sweep_mode(mode: SweepMode) -> &'static str {
    match mode {
        SweepMode::Dec => "dec",
        SweepMode::Oct => "oct",
        SweepMode::Lin => "lin",
    }
}

fn read_rawfile(input: &SimInput, path: impl AsRef<Path>) -> Result<SimOutput> {
    let data = std::fs::read(path)?;
    let raw = spice_rawfile::parse(&data)
        .map_err(|e| ErrorSource::Internal(format!("failed to parse simulation output: {e}")))?;

    let out = arrange_rawfile(input, raw);

    Ok(SimOutput { data: out })
}

fn arrange_rawfile(input: &SimInput, raw: Rawfile) -> Vec<AnalysisData> {
    let mut out = vec![AnalysisData::Other; input.analyses.len()];
    for an in raw.analyses {
        let t = atype(&an);
        let (idx, ian) = input
            .analyses
            .iter()
            .enumerate()
            .find(|(_, a)| a.analysis_type() == t)
            .unwrap();
        out[idx] = parse_analysis(ian, an);
    }
    out
}

fn atype(raw: &spice_rawfile::parser::Analysis) -> AnalysisType {
    let name = raw.plotname.to_lowercase();
    if name.contains("ac analysis") {
        AnalysisType::Ac
    } else if name.contains("transient") {
        AnalysisType::Tran
    } else if name.contains("operating") {
        AnalysisType::Op
    } else if name.contains("dc transfer") {
        AnalysisType::Dc
    } else {
        panic!("unknown analysis type {name}")
    }
}

use spice_rawfile::parser::Analysis as RawAnalysis;

fn parse_analysis(input: &Analysis, output: RawAnalysis) -> AnalysisData {
    match input {
        Analysis::Ac(ac) => AnalysisData::Ac(parse_ac(ac, output)),
        Analysis::Tran(tran) => AnalysisData::Tran(parse_tran(tran, output)),
        Analysis::Op(op) => AnalysisData::Op(parse_op(op, output)),
        Analysis::Dc(dc) => AnalysisData::Dc(parse_dc(dc, output)),
    }
}

use substrate::verification::simulation::ComplexSignal;

fn parse_ac(_input: &AcAnalysis, output: RawAnalysis) -> AcData {
    let data = output.data.unwrap_complex();
    let mut map = HashMap::with_capacity(output.variables.len() - 1);
    let mut freq = None;
    for (sig, var) in data.into_iter().zip(output.variables.iter()) {
        let sig = ComplexSignal {
            real: sig.real,
            imag: sig.imag,
            quantity: parse_qty(var.unit),
        };

        if var.name.trim() == "frequency" {
            assert_eq!(sig.quantity, Quantity::Frequency);
            freq = Some(RealSignal {
                values: sig.real,
                quantity: Quantity::Frequency,
            });
        } else {
            map.insert(var.name.trim().to_string(), sig);
        }
    }

    AcData {
        data: map,
        freq: freq.unwrap(),
    }
}

fn parse_tran(_input: &TranAnalysis, output: RawAnalysis) -> TranData {
    let data = output.data.unwrap_real();
    let mut map = HashMap::with_capacity(output.variables.len() - 1);
    let mut time = None;
    for (sig, var) in data.into_iter().zip(output.variables.iter()) {
        let sig = RealSignal {
            values: sig,
            quantity: parse_qty(var.unit),
        };

        if var.name.trim() == "time" {
            assert_eq!(sig.quantity, Quantity::Time);
            time = Some(sig);
        } else {
            map.insert(var.name.trim().to_string(), sig);
        }
    }

    TranData {
        data: map,
        time: time.unwrap(),
    }
}
fn parse_dc(_input: &DcAnalysis, output: RawAnalysis) -> DcData {
    let data = output.data.unwrap_real();
    let mut map = HashMap::with_capacity(output.variables.len());
    for (sig, var) in data.into_iter().zip(output.variables.iter()) {
        let sig = RealSignal {
            values: sig,
            quantity: parse_qty(var.unit),
        };

        map.insert(var.name.trim().to_string(), sig);
    }

    DcData { data: map }
}

fn parse_op(_input: &OpAnalysis, output: RawAnalysis) -> OpData {
    let data = output.data.unwrap_real();
    let mut map = HashMap::with_capacity(output.variables.len());
    for (sig, var) in data.into_iter().zip(output.variables.iter()) {
        assert_eq!(sig.len(), 1);
        let sig = ScalarSignal {
            value: sig[0],
            quantity: parse_qty(var.unit),
        };
        map.insert(var.name.trim().to_string(), sig);
    }

    OpData { data: map }
}

fn parse_qty(name: &str) -> Quantity {
    match name.trim() {
        "voltage" => Quantity::Voltage,
        "current" => Quantity::Current,
        "frequency" => Quantity::Frequency,
        "time" => Quantity::Time,
        "temp" | "temp-sweep" | "temperature" => Quantity::Temperature,
        _ => panic!("unknown quantity"),
    }
}
