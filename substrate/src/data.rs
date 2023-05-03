use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use serde::{Deserialize, Serialize};
use tempdir::TempDir;

use crate::component::{Component, View};
use crate::deps::arcstr::ArcStr;
use crate::digital::context::{DigitalCtx, DigitalData};
use crate::digital::module::{DigitalModule, DigitalModuleKey, Instance as DigitalInstance};
use crate::digital::{DigitalComponent, Interface};
use crate::error::{with_err_context, ErrorContext, ErrorSource, Result, SubstrateError};
use crate::generation::GeneratedCheck;
use crate::io::create_dir_all;
use crate::layout::cell::{Cell, CellKey, Instance as LayoutInstance};
use crate::layout::context::{LayoutCtx, LayoutData};
use crate::layout::layers::{Layers, LayersRef};
use crate::layout::LayoutFormat;
use crate::log::{self, Log};
use crate::pdk::corner::error::ProcessCornerError;
use crate::pdk::corner::{CornerDb, CornerEntry, Pvt};
use crate::pdk::mos::db::MosDb;
use crate::pdk::stdcell::StdCellDb;
use crate::pdk::Pdk;
use crate::schematic::circuit::{Instance as SchematicInstance, Reference};
use crate::schematic::context::{ModuleKey, SchematicCtx, SchematicData};
use crate::schematic::module::{AbstractModule, ExternalModule, Module, RawSource};
use crate::schematic::netlist::interface::{InstanceInfo, Netlister, SubcircuitInfo};
use crate::schematic::netlist::preprocess::{preprocess_netlist, PreprocessedNetlist};
use crate::schematic::netlist::NetlistPurpose;
use crate::schematic::validation::connectivity::validate_connectivity;
use crate::schematic::validation::drivers::validate_drivers;
use crate::schematic::validation::naming::validate_naming;
use crate::script::map::ScriptMap;
use crate::script::Script;
use crate::verification::drc::{DrcInput, DrcOutput, DrcTool};
use crate::verification::lvs::{LvsInput, LvsOutput, LvsTool};
use crate::verification::pex::{PexInput, PexOutput, PexTool};
use crate::verification::simulation::context::{PostSimCtx, PreSimCtx};
use crate::verification::simulation::testbench::Testbench;
use crate::verification::simulation::{SimInput, SimOpts, Simulator};
use crate::verification::timing::context::TimingCtx;
use crate::verification::timing::{generate_timing_report, TimingConfig};

pub(crate) struct SubstrateData {
    schematics: SchematicData,
    layouts: LayoutData,
    digital: DigitalData,
    netlister: Option<Arc<dyn Netlister>>,
    pdk: Arc<dyn Pdk>,
    mos_db: Arc<MosDb>,
    std_cell_db: Arc<StdCellDb>,
    layers: Arc<RwLock<Layers>>,
    simulator: Option<Arc<dyn Simulator>>,
    drc_tool: Option<Arc<dyn DrcTool>>,
    lvs_tool: Option<Arc<dyn LvsTool>>,
    pex_tool: Option<Arc<dyn PexTool>>,
    script_map: ScriptMap,
    corner_db: Arc<CornerDb>,
    simulation_bashrc: Option<PathBuf>,
    timing_config: Option<Arc<TimingConfig>>,
}

pub struct SubstrateConfig {
    pub netlister: Option<Arc<dyn Netlister>>,
    pub pdk: Arc<dyn Pdk>,
    pub simulator: Option<Arc<dyn Simulator>>,
    pub drc_tool: Option<Arc<dyn DrcTool>>,
    pub lvs_tool: Option<Arc<dyn LvsTool>>,
    pub pex_tool: Option<Arc<dyn PexTool>>,
    pub simulation_bashrc: Option<PathBuf>,
    pub timing_config: Option<Arc<TimingConfig>>,
}

#[derive(Default)]
pub struct SubstrateConfigBuilder {
    pub netlister: Option<Arc<dyn Netlister>>,
    pub pdk: Option<Arc<dyn Pdk>>,
    pub simulator: Option<Arc<dyn Simulator>>,
    pub drc_tool: Option<Arc<dyn DrcTool>>,
    pub lvs_tool: Option<Arc<dyn LvsTool>>,
    pub pex_tool: Option<Arc<dyn PexTool>>,
    pub simulation_bashrc: Option<PathBuf>,
    pub timing_config: Option<Arc<TimingConfig>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
enum FlattenTop {
    #[default]
    No,
    Yes {
        rename_ground: RenameNet,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
struct RenameNet {
    from: ArcStr,
    to: ArcStr,
}

pub struct WriteSchematicArgs<'a, P, W> {
    params: &'a P,
    out: W,
    purpose: NetlistPurpose,
    flatten_top: FlattenTop,
}

pub(crate) struct InnerWriteSchematicArgs<W> {
    top: ModuleKey,
    flatten_top: FlattenTop,
    purpose: NetlistPurpose,
    out: W,
}

/// Whether or not to verify timing constraints for transient simulations.
pub enum VerifyTiming {
    /// Do **not** verify timing constraints.
    No,
    /// Verify timing constraints in the given [`Pvt`] corner.
    Yes(Pvt),
}

impl SubstrateData {
    #[inline]
    pub(crate) fn from_config(cfg: SubstrateConfig) -> Result<Self> {
        let pdk = cfg.pdk.clone();
        Ok(Self {
            schematics: SchematicData::new(cfg.pdk.clone()),
            layouts: LayoutData::new(cfg.pdk.clone()),
            digital: DigitalData::new(),
            netlister: cfg.netlister,
            pdk: cfg.pdk.clone(),
            mos_db: Arc::new(MosDb::new(cfg.pdk.clone()).unwrap()),
            std_cell_db: Arc::new(pdk.standard_cells()?),
            corner_db: Arc::new(pdk.corners()?),
            layers: Arc::new(RwLock::new(cfg.pdk.layers())),
            simulator: cfg.simulator,
            drc_tool: cfg.drc_tool,
            lvs_tool: cfg.lvs_tool,
            pex_tool: cfg.pex_tool,
            script_map: ScriptMap::new(),
            simulation_bashrc: cfg.simulation_bashrc,
            timing_config: cfg.timing_config,
        })
    }
}

#[derive(Clone)]
pub struct SubstrateCtx {
    inner: Arc<RwLock<SubstrateData>>,
}

impl SubstrateConfig {
    #[inline]
    pub fn builder() -> SubstrateConfigBuilder {
        SubstrateConfigBuilder::default()
    }
}

impl SubstrateConfigBuilder {
    pub fn netlister<N>(&mut self, netlister: N) -> &mut Self
    where
        N: Netlister + 'static,
    {
        self.netlister = Some(Arc::new(netlister));
        self
    }

    pub fn pdk<T>(&mut self, pdk: T) -> &mut Self
    where
        T: Pdk + 'static,
    {
        self.pdk = Some(Arc::new(pdk));
        self
    }

    pub fn simulator<T>(&mut self, simulator: T) -> &mut Self
    where
        T: Simulator + 'static,
    {
        self.simulator = Some(Arc::new(simulator));
        self
    }

    pub fn drc_tool<T>(&mut self, drc_tool: T) -> &mut Self
    where
        T: DrcTool + 'static,
    {
        self.drc_tool = Some(Arc::new(drc_tool));
        self
    }

    pub fn lvs_tool<T>(&mut self, lvs_tool: T) -> &mut Self
    where
        T: LvsTool + 'static,
    {
        self.lvs_tool = Some(Arc::new(lvs_tool));
        self
    }

    pub fn pex_tool<T>(&mut self, pex_tool: T) -> &mut Self
    where
        T: PexTool + 'static,
    {
        self.pex_tool = Some(Arc::new(pex_tool));
        self
    }

    pub fn simulation_bashrc<P>(&mut self, path: P) -> &mut Self
    where
        P: Into<PathBuf>,
    {
        self.simulation_bashrc = Some(path.into());
        self
    }

    pub fn timing_config(&mut self, config: TimingConfig) -> &mut Self {
        self.timing_config = Some(Arc::new(config));
        self
    }

    pub fn build(&self) -> SubstrateConfig {
        SubstrateConfig {
            netlister: self.netlister.clone(),
            pdk: self.pdk.clone().unwrap(),
            simulator: self.simulator.clone(),
            drc_tool: self.drc_tool.clone(),
            lvs_tool: self.lvs_tool.clone(),
            pex_tool: self.pex_tool.clone(),
            simulation_bashrc: self.simulation_bashrc.clone(),
            timing_config: self.timing_config.clone(),
        }
    }
}

impl SubstrateCtx {
    #[inline]
    pub(crate) fn read(&self) -> RwLockReadGuard<SubstrateData> {
        self.inner.read().unwrap()
    }

    #[inline]
    pub(crate) fn write(&self) -> RwLockWriteGuard<SubstrateData> {
        self.inner.write().unwrap()
    }

    #[inline]
    pub fn from_config(cfg: SubstrateConfig) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(RwLock::new(SubstrateData::from_config(cfg)?)),
        })
    }

    pub fn pdk(&self) -> Arc<dyn Pdk> {
        self.read().pdk()
    }

    pub fn simulator(&self) -> Option<Arc<dyn Simulator>> {
        self.read().simulator()
    }

    pub fn mos_db(&self) -> Arc<MosDb> {
        self.read().mos_db()
    }

    pub fn netlister(&self) -> Option<Arc<dyn Netlister>> {
        self.read().netlister()
    }

    pub fn try_netlister(&self) -> Result<Arc<dyn Netlister>> {
        self.read().try_netlister()
    }

    pub fn std_cell_db(&self) -> Arc<StdCellDb> {
        self.read().std_cell_db()
    }

    pub fn corner_db(&self) -> Arc<CornerDb> {
        self.read().corner_db()
    }

    pub fn raw_layers(&self) -> Arc<RwLock<Layers>> {
        self.read().layers()
    }

    pub fn layers(&self) -> LayersRef {
        LayersRef::new(self.raw_layers())
    }

    pub fn drc_tool(&self) -> Option<Arc<dyn DrcTool>> {
        self.read().drc_tool()
    }

    pub fn lvs_tool(&self) -> Option<Arc<dyn LvsTool>> {
        self.read().lvs_tool()
    }

    pub fn pex_tool(&self) -> Option<Arc<dyn PexTool>> {
        self.read().pex_tool()
    }

    pub fn timing_config(&self) -> Option<Arc<TimingConfig>> {
        self.read().timing_config()
    }

    pub fn try_timing_config(&self) -> Result<Arc<TimingConfig>> {
        Ok(self
            .timing_config()
            .ok_or(ErrorSource::TimingConfigNotSpecified)?)
    }

    pub fn run_script<T>(&self, params: &T::Params) -> Result<Arc<T::Output>>
    where
        T: Script,
    {
        let res = {
            let inner = self.read();
            inner.script_map.get::<T>(params)
        };
        if let Some(v) = res {
            return Ok(v);
        }

        let output = T::run(params, self)?;

        let v = {
            let mut inner = self.write();
            inner.script_map.set::<T>(params, output)
        };

        Ok(v)
    }

    pub(crate) fn instantiate_external_schematic<Q>(&self, name: &Q) -> Result<SchematicInstance>
    where
        Q: AsRef<str>,
    {
        let inner = self.read();
        let module = inner.schematics.get_external(name)?;
        Ok(SchematicInstance::new(Reference::External(
            module.name().clone(),
        )))
    }

    pub fn add_external_module(&self, module: ExternalModule) -> Result<()> {
        let mut inner = self.write();
        inner.schematics.add_external(module)
    }

    pub fn instantiate_schematic<T>(&self, params: &T::Params) -> Result<SchematicInstance>
    where
        T: Component,
    {
        let check = {
            let mut inner = self.write();
            inner.schematics.get_module::<T>(params)
        };

        Ok(match check {
            GeneratedCheck::Exists(module) => SchematicInstance::new(Reference::Local(module)),
            GeneratedCheck::MustGenerate(id) => {
                let module = self.generate_schematic::<T>(params, id)?;
                SchematicInstance::new(Reference::Local(module))
            }
        })
    }

    pub fn write_schematic_for_purpose<T, W: Write>(
        &self,
        args: WriteSchematicArgs<T::Params, W>,
    ) -> Result<()>
    where
        T: Component,
    {
        let inst = self.instantiate_schematic::<T>(args.params)?;
        let top = inst
            .module()
            .local_id()
            .ok_or(ErrorSource::NetlistExternalModule)?;
        let args = InnerWriteSchematicArgs {
            top,
            flatten_top: args.flatten_top,
            purpose: args.purpose,
            out: args.out,
        };
        let mut inner = self.write();
        inner.write_schematic(args)?;
        Ok(())
    }

    pub(crate) fn _write_schematic_for_purpose<T, W: Write>(
        &self,
        args: WriteSchematicArgs<T::Params, W>,
    ) -> Result<PreprocessedNetlist>
    where
        T: Component,
    {
        let inst = self.instantiate_schematic::<T>(args.params)?;
        let top = inst
            .module()
            .local_id()
            .ok_or(ErrorSource::NetlistExternalModule)?;
        let args = InnerWriteSchematicArgs {
            top,
            flatten_top: args.flatten_top,
            purpose: args.purpose,
            out: args.out,
        };
        let mut inner = self.write();
        let netlist = inner.write_schematic(args)?;
        Ok(netlist)
    }

    #[inline]
    pub fn write_schematic<T, W: Write>(&self, params: &T::Params, out: W) -> Result<()>
    where
        T: Component,
    {
        let args = WriteSchematicArgs {
            params,
            out,
            purpose: NetlistPurpose::default(),
            flatten_top: FlattenTop::No,
        };
        self.write_schematic_for_purpose::<T, W>(args)
    }

    pub fn write_schematic_to_file_for_purpose<T>(
        &self,
        params: &T::Params,
        out: impl AsRef<Path>,
        purpose: NetlistPurpose,
    ) -> Result<()>
    where
        T: Component,
    {
        let path = out.as_ref();
        let purp = purpose.clone();
        let inner = move || -> Result<()> {
            if let Some(prefix) = path.parent() {
                create_dir_all(prefix)?;
            }
            let mut f = crate::io::create_file(path)?;
            let args = WriteSchematicArgs {
                params,
                out: &mut f,
                purpose,
                flatten_top: FlattenTop::No,
            };
            self.write_schematic_for_purpose::<T, _>(args)
        };

        with_err_context(inner(), || {
            ErrorContext::Task(arcstr::format!(
                "writing schematic to file {:?} for purpose {}",
                path,
                purp,
            ))
        })
    }

    #[inline]
    pub fn write_schematic_to_file<T>(
        &self,
        params: &T::Params,
        out: impl AsRef<Path>,
    ) -> Result<()>
    where
        T: Component,
    {
        self.write_schematic_to_file_for_purpose::<T>(params, out, NetlistPurpose::default())
    }

    pub fn instantiate_layout<T>(&self, params: &T::Params) -> Result<LayoutInstance>
    where
        T: Component,
    {
        let check = {
            let mut inner = self.write();
            inner.layouts.get_generated_cell::<T>(params)
        };

        Ok(match check {
            GeneratedCheck::Exists(cell) => LayoutInstance::new(cell),
            GeneratedCheck::MustGenerate(id) => {
                let cell = self.generate_layout::<T>(params, id)?;
                LayoutInstance::new(cell)
            }
        })
    }

    pub fn write_layout<T>(&self, params: &T::Params, path: impl AsRef<Path>) -> Result<()>
    where
        T: Component,
    {
        let path = path.as_ref();

        let inner = || -> Result<()> {
            let inst = self.instantiate_layout::<T>(params)?;
            let top = inst.cell().clone();
            if let Some(parent) = path.parent() {
                create_dir_all(parent)?;
            }
            self.to_gds_with_top(top, path)?;
            Ok(())
        };

        with_err_context(inner(), || {
            ErrorContext::Task(arcstr::format!("writing layout to file {:?}", path))
        })
    }

    #[inline]
    pub fn instantiate_digital<T>(&self, params: &T::Params) -> Result<DigitalInstance>
    where
        T: DigitalComponent,
    {
        Ok(self._instantiate_digital::<T>(params)?.0)
    }

    pub(crate) fn _instantiate_digital<T>(
        &self,
        params: &T::Params,
    ) -> Result<(DigitalInstance, T::Interface)>
    where
        T: DigitalComponent,
    {
        let check = {
            let mut inner = self.write();
            inner.digital.get_generated_module::<T>(params)
        };

        let component = self.init_component::<T>(params)?;

        Ok(match check {
            GeneratedCheck::Exists(id) => (DigitalInstance::new(id), component.interface()),
            GeneratedCheck::MustGenerate(id) => {
                let (id, intf) = self.generate_digital::<T>(component, id)?;
                (DigitalInstance::new(id), intf)
            }
        })
    }

    pub fn run_drc(&self, input: DrcInput) -> Result<DrcOutput> {
        with_err_context(self._run_drc(input), || {
            ErrorContext::Task(arcstr::literal!("running DRC"))
        })
    }

    fn _run_drc(&self, input: DrcInput) -> Result<DrcOutput> {
        self.drc_tool()
            .ok_or(ErrorSource::ToolNotSpecified)?
            .run_drc(input)
    }

    pub fn write_drc<T>(&self, params: &T::Params, work_dir: impl AsRef<Path>) -> Result<DrcOutput>
    where
        T: Component,
    {
        let work_dir = work_dir.as_ref();
        with_err_context(self._write_drc::<T>(params, work_dir), || {
            ErrorContext::Task(arcstr::format!(
                "running DRC in working directory {:?}",
                work_dir
            ))
        })
    }

    fn _write_drc<T>(&self, params: &T::Params, work_dir: impl AsRef<Path>) -> Result<DrcOutput>
    where
        T: Component,
    {
        let work_dir = work_dir.as_ref();
        create_dir_all(work_dir)?;
        let layout_path = PathBuf::from(&work_dir).join("layout.gds");
        self.write_layout::<T>(params, &layout_path)?;
        self.run_drc(DrcInput {
            cell_name: T::new(params, self)?.name(),
            work_dir: PathBuf::from(&work_dir),
            layout_path,
            layout_format: LayoutFormat::Gds,
            opts: HashMap::new(),
        })
    }

    pub fn run_lvs(&self, input: LvsInput) -> Result<LvsOutput> {
        with_err_context(self._run_lvs(input), || {
            ErrorContext::Task(arcstr::literal!("running LVS"))
        })
    }

    fn _run_lvs(&self, input: LvsInput) -> Result<LvsOutput> {
        self.lvs_tool()
            .ok_or(ErrorSource::ToolNotSpecified)?
            .run_lvs(input)
    }

    pub fn write_lvs<T>(&self, params: &T::Params, work_dir: impl AsRef<Path>) -> Result<LvsOutput>
    where
        T: Component,
    {
        let work_dir = work_dir.as_ref();
        with_err_context(self._write_lvs::<T>(params, work_dir), || {
            ErrorContext::Task(arcstr::format!(
                "running LVS in working directory {:?}",
                work_dir
            ))
        })
    }

    fn _write_lvs<T>(&self, params: &T::Params, work_dir: impl AsRef<Path>) -> Result<LvsOutput>
    where
        T: Component,
    {
        let work_dir = work_dir.as_ref();
        create_dir_all(work_dir)?;
        let layout_path = PathBuf::from(&work_dir).join("layout.gds");
        self.write_layout::<T>(params, &layout_path)?;
        let schematic_path = PathBuf::from(&work_dir).join("netlist.spice");
        self.write_schematic_to_file_for_purpose::<T>(
            params,
            &schematic_path,
            NetlistPurpose::Lvs,
        )?;
        let cell_name = T::new(params, self)?.name();
        self.run_lvs(LvsInput {
            work_dir: PathBuf::from(&work_dir),
            layout_path,
            layout_cell_name: cell_name.clone(),
            layout_format: LayoutFormat::Gds,
            source_paths: vec![schematic_path],
            source_cell_name: cell_name,
            opts: HashMap::new(),
        })
    }

    pub fn run_pex(&self, input: PexInput) -> Result<PexOutput> {
        with_err_context(self._run_pex(input), || {
            ErrorContext::Task(arcstr::literal!("running PEX"))
        })
    }

    fn _run_pex(&self, input: PexInput) -> Result<PexOutput> {
        self.pex_tool()
            .ok_or(ErrorSource::ToolNotSpecified)?
            .run_pex(input)
    }

    pub fn write_pex<T>(
        &self,
        params: &T::Params,
        work_dir: impl AsRef<Path>,
        pex_netlist_path: impl Into<PathBuf>,
    ) -> Result<PexOutput>
    where
        T: Component,
    {
        let work_dir = work_dir.as_ref();

        with_err_context(
            self._write_pex::<T>(params, work_dir, pex_netlist_path),
            || {
                ErrorContext::Task(arcstr::format!(
                    "running PEX in working directory {:?}",
                    work_dir
                ))
            },
        )
    }

    fn _write_pex<T>(
        &self,
        params: &T::Params,
        work_dir: impl AsRef<Path>,
        pex_netlist_path: impl Into<PathBuf>,
    ) -> Result<PexOutput>
    where
        T: Component,
    {
        let work_dir = work_dir.as_ref();
        create_dir_all(work_dir)?;
        let layout_path = PathBuf::from(&work_dir).join("layout.gds");
        self.write_layout::<T>(params, &layout_path)?;
        let schematic_path = PathBuf::from(&work_dir).join("netlist.spice");
        self.write_schematic_to_file_for_purpose::<T>(
            params,
            &schematic_path,
            NetlistPurpose::Pex,
        )?;
        let cell_name = T::new(params, self)?.name();
        self.run_pex(PexInput {
            work_dir: PathBuf::from(&work_dir),
            layout_path,
            layout_cell_name: cell_name.clone(),
            layout_format: LayoutFormat::Gds,
            source_paths: vec![schematic_path],
            source_cell_name: cell_name,
            pex_netlist_path: pex_netlist_path.into(),
            opts: HashMap::new(),
        })
    }

    pub fn write_simulation<T>(
        &self,
        params: &T::Params,
        work_dir: impl AsRef<Path>,
    ) -> Result<T::Output>
    where
        T: Testbench,
    {
        let work_dir = work_dir.as_ref();
        with_err_context(
            self._write_simulation::<T>(params, work_dir, None, VerifyTiming::No),
            || {
                ErrorContext::Task(arcstr::format!(
                    "running simulation in working directory {:?}",
                    work_dir
                ))
            },
        )
    }

    pub fn write_simulation_with_corner<T>(
        &self,
        params: &T::Params,
        work_dir: impl AsRef<Path>,
        corner: CornerEntry,
    ) -> Result<T::Output>
    where
        T: Testbench,
    {
        let work_dir = work_dir.as_ref();
        with_err_context(
            self._write_simulation::<T>(params, work_dir, Some(corner), VerifyTiming::No),
            || {
                ErrorContext::Task(arcstr::format!(
                    "running simulation in working directory {:?}",
                    work_dir
                ))
            },
        )
    }

    pub fn _write_simulation<T>(
        &self,
        params: &T::Params,
        work_dir: impl AsRef<Path>,
        corner: Option<CornerEntry>,
        verify_timing: VerifyTiming,
    ) -> Result<T::Output>
    where
        T: Testbench,
    {
        let corner = if let Some(corner) = corner {
            corner
        } else {
            let corners = self.corner_db();
            corners
                .default_corner()
                .ok_or(ProcessCornerError::NoDefaultCorner)?
                .clone()
        };

        let work_dir = work_dir.as_ref();
        create_dir_all(work_dir)?;
        let path = work_dir.join("source.spice");
        let mut f = File::create(&path)?;

        let opts = self.try_netlister()?.opts();
        let mut tb = T::new(params, self)?;

        let args = WriteSchematicArgs {
            params,
            out: &mut f,
            purpose: NetlistPurpose::Simulation { corner },
            flatten_top: FlattenTop::Yes {
                rename_ground: RenameNet {
                    from: tb.ground_net(),
                    to: opts.global_ground_net,
                },
            },
        };

        let netlist: PreprocessedNetlist = self._write_schematic_for_purpose::<T, _>(args)?;

        f.flush()?;
        drop(f);

        let bashrc = {
            let inner = self.read();
            inner.simulation_bashrc()
        };

        let input = SimInput {
            work_dir: work_dir.to_owned(),
            includes: vec![path],
            opts: SimOpts {
                bashrc,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut ctx = PreSimCtx::new(input);

        tb.setup(&mut ctx)?;
        self.pdk().pre_sim(&mut ctx)?;
        let simulator = self.simulator().ok_or(ErrorSource::ToolNotSpecified)?;

        let output = if let VerifyTiming::Yes(ref pvt) = verify_timing {
            let timing_config = self.try_timing_config()?;
            let mut constraints = netlist.timing_constraint_db(pvt);

            for constraint in constraints.named_constraints(&netlist) {
                let port = simulator.node_voltage_string(&constraint.port);
                ctx.input.save.add(port);
                if let Some(ref related_port) = constraint.related_port {
                    let related_port = simulator.node_voltage_string(related_port);
                    ctx.input.save.add(related_port);
                }
            }

            let output = simulator.simulate(ctx.into_inner())?;

            let data = output.data[0].tran();
            let report = generate_timing_report(
                constraints.named_constraints(&netlist),
                data,
                &*simulator,
                &timing_config,
            );

            report.log();
            let path = work_dir.join("timing.rpt");
            report.save_to_file(path)?;
            if report.is_failure() {
                return Err(ErrorSource::TimingFailed(report).into());
            }

            output
        } else {
            simulator.simulate(ctx.into_inner())?
        };

        let mut ctx = PostSimCtx { output };
        tb.post_sim(&mut ctx)?;
        let output = tb.measure(&ctx)?;

        Ok(output)
    }

    pub fn simulate<T>(&self, params: &T::Params) -> Result<T::Output>
    where
        T: Testbench,
    {
        let work_dir = TempDir::new("subsim")?;
        let work_dir = work_dir.path();
        self.write_simulation::<T>(params, work_dir)
    }

    pub(crate) fn generate_schematic<T>(
        &self,
        params: &T::Params,
        id: ModuleKey,
    ) -> Result<Arc<Module>>
    where
        T: Component,
    {
        self._generate_schematic::<T>(params, id)
    }

    fn _generate_schematic<T>(&self, params: &T::Params, id: ModuleKey) -> Result<Arc<Module>>
    where
        T: Component,
    {
        let mut ctx = SchematicCtx {
            inner: self.clone(),
            module: Module::new(id),
        };
        let component = self.init_component::<T>(params)?;
        let name = component.name();
        ctx.module.set_name(component.name());
        with_err_context(component.schematic(&mut ctx), || {
            ErrorContext::GenComponent {
                name: name.clone(),
                type_name: std::any::type_name::<T>().into(),
                view: View::Schematic,
            }
        })?;

        let mut ctx = TimingCtx::new(ctx.module, self.clone());
        with_err_context(component.timing(&mut ctx), || ErrorContext::GenComponent {
            name,
            type_name: std::any::type_name::<T>().into(),
            view: View::Timing,
        })?;

        let module = {
            let mut inner = self.write();
            inner.schematics.set_module(ctx.into_module())
        };

        Ok(module)
    }

    fn init_component<T>(&self, params: &T::Params) -> Result<T>
    where
        T: Component,
    {
        let component = with_err_context(T::new(params, self), || ErrorContext::InitComponent {
            type_name: std::any::type_name::<T>().into(),
        })?;
        Ok(component)
    }

    pub(crate) fn generate_layout<T>(&self, params: &T::Params, id: CellKey) -> Result<Arc<Cell>>
    where
        T: Component,
    {
        let mut ctx = LayoutCtx {
            inner: self.clone(),
            cell: Cell::new(id),
        };
        let component = self.init_component::<T>(params)?;
        let name = component.name();
        ctx.cell.set_name(name.clone());
        with_err_context(component.layout(&mut ctx), || ErrorContext::GenComponent {
            name,
            type_name: std::any::type_name::<T>().into(),
            view: View::Layout,
        })?;
        ctx.cell.freeze();
        ctx.cell.validate()?;

        // Now that the cell is frozen, mark `ctx` as immutable
        // so we don't accidentally modify the cell in any way.
        let ctx = ctx;

        let cell = {
            let mut inner = self.write();
            inner.layouts.set_cell(ctx.cell)
        };

        Ok(cell)
    }

    pub(crate) fn generate_digital<T>(
        &self,
        component: T,
        id: DigitalModuleKey,
    ) -> Result<(Arc<DigitalModule>, T::Interface)>
    where
        T: DigitalComponent,
    {
        let mut ctx = DigitalCtx {
            inner: self.clone(),
            module: DigitalModule::new(id),
        };
        let name = component.name();
        ctx.module.set_name(name.clone());
        // TODO implement logic for creating interface

        let intf = component.interface();
        let input = intf.input(&mut ctx);
        let out = with_err_context(component.digital(&mut ctx, input), || {
            ErrorContext::GenComponent {
                name,
                type_name: std::any::type_name::<T>().into(),
                view: View::Digital,
            }
        })?;
        ctx.finish::<T>(out);
        // TODO validation logic?

        let module = {
            let mut inner = self.write();
            inner.digital.set_module(ctx.module)
        };

        Ok((module, intf))
    }

    #[allow(dead_code)]
    pub(crate) fn get_external_module<Q>(&self, name: &Q) -> Result<Arc<ExternalModule>>
    where
        Q: AsRef<str>,
    {
        let inner = self.read();
        inner.get_external_module(name)
    }
}

impl SubstrateData {
    #[inline]
    pub(crate) fn layouts(&self) -> &LayoutData {
        &self.layouts
    }

    #[inline]
    pub(crate) fn layouts_mut(&mut self) -> &mut LayoutData {
        &mut self.layouts
    }

    #[inline]
    pub(crate) fn pdk(&self) -> Arc<dyn Pdk> {
        self.pdk.clone()
    }

    pub(crate) fn netlister(&self) -> Option<Arc<dyn Netlister>> {
        self.netlister.clone()
    }

    pub(crate) fn try_netlister(&self) -> Result<Arc<dyn Netlister>> {
        let result = self.netlister().ok_or(ErrorSource::ToolNotSpecified)?;
        Ok(result)
    }

    #[inline]
    pub(crate) fn simulator(&self) -> Option<Arc<dyn Simulator>> {
        self.simulator.clone()
    }

    #[inline]
    pub(crate) fn mos_db(&self) -> Arc<MosDb> {
        self.mos_db.clone()
    }

    #[inline]
    pub(crate) fn std_cell_db(&self) -> Arc<StdCellDb> {
        self.std_cell_db.clone()
    }

    #[inline]
    pub(crate) fn corner_db(&self) -> Arc<CornerDb> {
        self.corner_db.clone()
    }

    #[inline]
    pub(crate) fn layers(&self) -> Arc<RwLock<Layers>> {
        self.layers.clone()
    }

    #[inline]
    pub(crate) fn drc_tool(&self) -> Option<Arc<dyn DrcTool>> {
        self.drc_tool.clone()
    }

    #[inline]
    pub(crate) fn simulation_bashrc(&self) -> Option<PathBuf> {
        self.simulation_bashrc.clone()
    }

    #[inline]
    pub(crate) fn timing_config(&self) -> Option<Arc<TimingConfig>> {
        self.timing_config.clone()
    }

    #[inline]
    pub(crate) fn lvs_tool(&self) -> Option<Arc<dyn LvsTool>> {
        self.lvs_tool.clone()
    }

    #[inline]
    pub(crate) fn pex_tool(&self) -> Option<Arc<dyn PexTool>> {
        self.pex_tool.clone()
    }

    pub(crate) fn write_schematic<W>(
        &mut self,
        args: InnerWriteSchematicArgs<W>,
    ) -> Result<PreprocessedNetlist>
    where
        W: Write,
    {
        let purp = args.purpose.clone();
        with_err_context(self._write_schematic(args), || {
            ErrorContext::Task(arcstr::format!("writing schematic for {}", purp))
        })
    }

    fn _write_schematic<W>(
        &mut self,
        args: InnerWriteSchematicArgs<W>,
    ) -> Result<PreprocessedNetlist>
    where
        W: Write,
    {
        let netlist = preprocess_netlist(&self.schematics, args.top)?;
        let validation = validate_naming(&netlist, &self.schematics.external_modules);
        validation.log();
        if validation.has_errors() {
            return Err(SubstrateError::from_context(
                ErrorSource::InvalidNetlist(validation.first_error()),
                ErrorContext::Task(arcstr::literal!("validating names in netlist")),
            ));
        }
        let validation = validate_connectivity(&netlist, &self.schematics.external_modules);
        validation.log();
        if validation.has_errors() {
            return Err(SubstrateError::from_context(
                ErrorSource::InvalidNetlist(validation.first_error()),
                ErrorContext::Task(arcstr::literal!("validating netlist connectivity")),
            ));
        }
        let validation = validate_drivers(&netlist, &self.schematics.external_modules);
        validation.log();
        if validation.has_errors() {
            return Err(SubstrateError::from_context(
                ErrorSource::InvalidNetlist(validation.first_error()),
                ErrorContext::Task(arcstr::literal!("validating net drivers")),
            ));
        }

        let mut out = Box::new(args.out);

        let top = &netlist.modules[netlist.top];

        let netlister = self.try_netlister()?;
        netlister.emit_begin(&mut out)?;
        netlister.emit_comment(&mut out, top.name())?;
        netlister.emit_comment(&mut out, "Schematic generated by Substrate")?;

        let mut include_paths = HashSet::new();

        let includes = self.pdk.includes(args.purpose)?;
        for path in includes.includes {
            let path = crate::io::canonicalize(path)?;
            if !include_paths.contains(&path) {
                netlister.emit_include(&mut out, &path)?;
                include_paths.insert(path);
            }
        }
        for (path, section) in includes.lib_includes {
            netlister.emit_lib_include(&mut out, &path, &section)?;
        }
        netlister.emit_raw_spice(&mut out, &includes.raw_spice)?;
        netlister.emit_raw_spice(&mut out, "\n")?;

        for key in netlist.netlist_order.iter().copied() {
            // If we're emitting the top module directly at the top level,
            // emit only its contents, and rename its ground net.
            if key == top.id() {
                if let FlattenTop::Yes { ref rename_ground } = args.flatten_top {
                    self.emit_inner_with_renamed_ground(
                        key,
                        &netlist,
                        &mut out,
                        rename_ground.clone(),
                    )?;
                    continue;
                }
            }

            // Otherwise, emit the module normally.
            self.emit_module(key, &netlist, &mut out)?;
        }

        for module in self.schematics.external_modules() {
            let source = module.source();
            match source {
                RawSource::File(path) => {
                    let path = crate::io::canonicalize(path)?;
                    if !include_paths.contains(&path) {
                        netlister.emit_include(&mut out, &path)?;
                        include_paths.insert(path);
                    }
                }
                RawSource::Literal(spice) => netlister.emit_raw_spice(&mut out, spice)?,
                RawSource::ManualInclude => (),
            }
        }

        netlister.emit_end(&mut out)?;
        out.flush()?;
        Ok(netlist)
    }

    fn get_external_module<Q>(&self, name: &Q) -> Result<Arc<ExternalModule>>
    where
        Q: AsRef<str>,
    {
        self.schematics.get_external(name).cloned()
    }

    fn emit_local_instance<W>(
        &mut self,
        module: &Module,
        inst: &SchematicInstance,
        netlist: &PreprocessedNetlist,
        out: &mut Box<W>,
    ) -> Result<()>
    where
        W: Write,
    {
        let key = inst.module().local_id().unwrap();
        let submodule = &netlist.modules[key];
        let conns = inst.connections();

        let mut ordered_conns = Vec::with_capacity(submodule.raw_ports().len());

        for port in submodule.ports() {
            let name = submodule.signals()[port.signal].name();
            ordered_conns.push(&conns[name]);
        }

        let info = InstanceInfo {
            name: inst.name(),
            ports: &ordered_conns,
            params: inst.params(),
            signals: module.signals(),
            subcircuit_name: submodule.name(),
        };

        self.try_netlister()?.emit_instance(out, info)?;
        Ok(())
    }

    fn emit_external_instance<W>(
        &mut self,
        module: &Module,
        inst: &SchematicInstance,
        out: &mut Box<W>,
    ) -> Result<()>
    where
        W: Write,
    {
        let key = inst.module().external().unwrap();
        let submodule = self.schematics.get_external(&key)?;
        let conns = inst.connections();

        let mut ordered_conns = Vec::with_capacity(submodule.raw_ports().len());

        for port in submodule.raw_ports() {
            let name = submodule.signals()[port.signal].name();
            ordered_conns.push(&conns[name]);
        }

        let info = InstanceInfo {
            name: inst.name(),
            ports: &ordered_conns,
            params: inst.params(),
            signals: module.signals(),
            subcircuit_name: submodule.name(),
        };

        self.try_netlister()?.emit_instance(out, info)?;
        Ok(())
    }

    fn emit_module<W: Write>(
        &mut self,
        key: ModuleKey,
        netlist: &PreprocessedNetlist,
        out: &mut Box<W>,
    ) -> Result<()> {
        let module = &netlist.modules[key];
        let info = SubcircuitInfo {
            name: module.name(),
            ports: module.raw_ports(),
            params: module.params(),
            signals: module.signals(),
        };
        let netlister = self.try_netlister()?;

        netlister.emit_begin_subcircuit(out, info)?;

        for inst in module.instances() {
            match inst.module() {
                Reference::Local(_) => self.emit_local_instance(module, inst, netlist, out)?,
                Reference::External(_) => self.emit_external_instance(module, inst, out)?,
            };
        }

        if let Some(spice) = module.raw_spice() {
            netlister.emit_raw_spice(out, spice)?;
        }

        netlister.emit_end_subcircuit(out, module.name())?;
        Ok(())
    }

    fn emit_inner_with_renamed_ground<W: Write>(
        &mut self,
        key: ModuleKey,
        netlist: &PreprocessedNetlist,
        out: &mut Box<W>,
        rename_ground: RenameNet,
    ) -> Result<()> {
        let mut module = netlist.modules[key].clone();
        for info in module.signals_mut().values_mut() {
            if *info.name() == rename_ground.from {
                info.set_name(rename_ground.to.clone());
            }
        }

        self.emit_inner(&module, netlist, out)?;

        Ok(())
    }

    /// Emits the contents of the given module.
    fn emit_inner<W: Write>(
        &mut self,
        module: &Module,
        netlist: &PreprocessedNetlist,
        out: &mut Box<W>,
    ) -> Result<()> {
        let netlister = self.try_netlister()?;

        for inst in module.instances() {
            match inst.module() {
                Reference::Local(_) => self.emit_local_instance(module, inst, netlist, out)?,
                Reference::External(_) => self.emit_external_instance(module, inst, out)?,
            };
        }

        if let Some(spice) = module.raw_spice() {
            log::warn!("Raw spice in flattened top level modules is unsupported. This may result in floating ground nets. If you need raw spice in a testbench, consider importing it as a hard macro or in a submodule.");
            netlister.emit_raw_spice(out, spice)?;
        }

        Ok(())
    }
}

impl FlattenTop {
    #[inline]
    #[allow(unused)]
    pub fn as_bool(&self) -> bool {
        match self {
            Self::No => false,
            Self::Yes { .. } => true,
        }
    }
}
