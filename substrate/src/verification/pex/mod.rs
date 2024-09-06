//! PEX plugin API.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::deps::arcstr::ArcStr;
use crate::error::Result;
use crate::layout::LayoutFormat;

/// Inputs passed to a [`PexTool`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PexInput {
    /// The directory to place intermediate and output files.
    pub work_dir: PathBuf,
    /// The path to the layout file containing the cell of interest.
    pub layout_path: PathBuf,
    /// The name of the layout cell to run PEX on.
    pub layout_cell_name: ArcStr,
    /// The format of the layout file.
    pub layout_format: LayoutFormat,
    /// A list of paths to netlist source files.
    pub source_paths: Vec<PathBuf>,
    /// The name of the schematic cell to run PEX on.
    pub source_cell_name: ArcStr,
    /// Output path for extracted netlist.
    pub pex_netlist_path: PathBuf,
    /// Unstructured options.
    pub opts: HashMap<ArcStr, ArcStr>,
    /// The name of the ground net.
    pub ground_net: String,
}

/// An enumeration describing the high-level result of a PEX run.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PexSummary {
    /// PEX run passed.
    Pass,
    /// PEX run passed with warnings.
    Warn,
    /// PEX run failed.
    Fail,
}

impl PexSummary {
    /// Checks if a [`PexSummary`] describes a passing PEX run.
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Pass | Self::Warn => true,
            Self::Fail => false,
        }
    }
}

/// A PEX error.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PexError {
    /// The name of the error.
    pub name: ArcStr,
    /// An optional description of the error.
    pub desc: Option<ArcStr>,
}

/// Outputs emitted by a [`PexTool`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PexOutput {
    /// A summary of the PEX run.
    pub summary: PexSummary,
    /// A list of errors encountered during the PEX run.
    pub errors: Vec<PexError>,
}

/// The trait that PEX plugins must implement.
pub trait PexTool {
    /// Runs the PEX tool on the provided input files.
    fn run_pex(&self, input: PexInput) -> Result<PexOutput>;
}
