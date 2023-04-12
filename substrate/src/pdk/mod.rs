use std::collections::HashMap;
use std::fmt::Display;
use std::path::PathBuf;

use self::corner::CornerDb;
use self::mos::spec::MosSpec;
use self::mos::{LayoutMosParams, MosParams};
use self::stdcell::StdCellDb;
use crate::error::Result;
use crate::layout::context::LayoutCtx;
use crate::layout::elements::via::ViaParams;
use crate::layout::layers::Layers;
use crate::schematic::context::SchematicCtx;
use crate::schematic::netlist::{IncludeBundle, NetlistPurpose};
use crate::units::SiPrefix;
use crate::verification::simulation::context::PreSimCtx;

pub mod corner;
pub mod mos;
pub mod stdcell;

#[derive(Debug, Clone)]
pub struct PdkParams {
    /// The path at which PDK files are stored
    pub pdk_root: PathBuf,
}

#[derive(Clone, Default, Debug)]
pub struct Supplies {
    pub values: HashMap<SupplyId, Supply>,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Supply {
    pub typ: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Hash)]
pub enum SupplyId {
    #[default]
    Core,
    Named(String),
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DeviceClass {
    Mos,
    Res,
    Cap,
    Ind,
    Diode,
    Other,
}

impl Display for DeviceClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Mos => write!(f, "mos"),
            Self::Res => write!(f, "res"),
            Self::Cap => write!(f, "cap"),
            Self::Ind => write!(f, "ind"),
            Self::Diode => write!(f, "diode"),
            Self::Other => write!(f, "other"),
        }
    }
}

pub struct Units {
    pub(crate) schematic: SiPrefix,
    pub(crate) layout: SiPrefix,
}

impl Units {
    pub fn new(schematic: SiPrefix, layout: SiPrefix) -> Self {
        Self { schematic, layout }
    }
}

pub trait Pdk {
    fn name(&self) -> &'static str;

    fn process(&self) -> &'static str;

    fn lengths(&self) -> Units;

    fn voltages(&self) -> SiPrefix;

    fn layers(&self) -> Layers;

    fn supplies(&self) -> Supplies;

    /// Retrieves the list of MOSFETs available in this PDK.
    fn mos_devices(&self) -> Vec<MosSpec>;

    /// Provide the SPICE netlist for a MOSFET with the given parameters.
    ///
    /// The drain, gate, source, and body ports are named
    /// `d`, `g`, `s`, and `b`, respectively.
    fn mos_schematic(&self, ctx: &mut SchematicCtx, params: &MosParams) -> Result<()>;

    /// Draws MOSFETs with the given parameters
    // TODO: define layout type
    fn mos_layout(&self, ctx: &mut LayoutCtx, params: &LayoutMosParams) -> Result<()>;

    /// Draws a via with the given params in the given context.
    fn via_layout(&self, ctx: &mut LayoutCtx, params: &ViaParams) -> Result<()>;

    /// The grid on which all layout geometry must lie.
    fn layout_grid(&self) -> i64;

    /// Called before running simulations.
    ///
    /// Allows the PDK to include model libraries, configure simulation
    /// options, and/or write relevant files.
    fn pre_sim(&self, _ctx: &mut PreSimCtx) -> Result<()> {
        Ok(())
    }

    /// Returns data that should be prepended to generated netlists,
    /// depending on their [purpose](NetlistPurpose) and the [process corner](crate::pdk::corner::CornerEntry).
    #[allow(unused)]
    fn includes(&self, purpose: NetlistPurpose) -> Result<IncludeBundle> {
        Ok(Default::default())
    }

    /// Returns a database of the standard cell libraries available in the PDK.
    fn standard_cells(&self) -> Result<StdCellDb> {
        Ok(StdCellDb::new())
    }

    /// Returns a database of the available process corners.
    fn corners(&self) -> Result<CornerDb> {
        Ok(CornerDb::new())
    }
}
