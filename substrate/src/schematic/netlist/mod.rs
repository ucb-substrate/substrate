//! Nestlisting utilities and implementations.

use std::fmt::Display;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::deps::arcstr::ArcStr;
use crate::pdk::corner::CornerEntry;

pub mod impls;
pub mod interface;
pub(crate) mod preprocess;

#[derive(Clone, Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum NetlistPurpose {
    /// A standalone netlist that can be included from other, higher-level, netlists.
    #[default]
    Library,
    /// A netlist to use for LVS.
    Lvs,
    /// A netlist to use for PEX.
    Pex,
    /// A netlist to use as the top-level schematic in simulation.
    Simulation { corner: CornerEntry },
}

impl Display for NetlistPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl NetlistPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            NetlistPurpose::Lvs => "LVS",
            NetlistPurpose::Pex => "PEX",
            NetlistPurpose::Simulation { .. } => "simulation",
            NetlistPurpose::Library => "library",
        }
    }

    pub fn is_simulation(&self) -> bool {
        matches!(self, NetlistPurpose::Simulation { .. })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct IncludeBundle {
    pub includes: Vec<PathBuf>,
    pub lib_includes: Vec<(PathBuf, ArcStr)>,
    pub raw_spice: ArcStr,
}
