//! LVS plugin API.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::deps::arcstr::ArcStr;
use crate::error::Result;
use crate::layout::LayoutFormat;

/// Inputs passed to a [`LvsTool`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LvsInput {
    /// The directory to place intermediate and output files.
    pub work_dir: PathBuf,
    /// The path to the layout file containing the cell of interest.
    pub layout_path: PathBuf,
    /// The name of the layout cell to run LVS on.
    pub layout_cell_name: ArcStr,
    /// The format of the layout file.
    pub layout_format: LayoutFormat,
    /// A list of paths to netlist source files.
    pub source_paths: Vec<PathBuf>,
    /// The name of the schematic cell to run LVS on.
    pub source_cell_name: ArcStr,
    /// Unstructured options.
    pub opts: HashMap<ArcStr, ArcStr>,
}

/// An enumeration describing the high-level result of a LVS run.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum LvsSummary {
    /// LVS run passed.
    Pass,
    /// LVS run passed with warnings.
    Warn,
    /// LVS run failed.
    Fail,
}

impl LvsSummary {
    /// Checks if a [`LvsSummary`] describes a passing LVS run.
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Pass | Self::Warn => true,
            Self::Fail => false,
        }
    }
}

/// A LVS error.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct LvsError {
    /// The name of the error.
    pub name: ArcStr,
    /// An optional description of the error.
    pub desc: Option<ArcStr>,
}

/// Outputs emitted by a [`LvsTool`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct LvsOutput {
    /// A summary of the LVS run.
    pub summary: LvsSummary,
    /// A list of errors encountered during the LVS run.
    pub errors: Vec<LvsError>,
}

/// The trait that LVS plugins must implement.
pub trait LvsTool {
    /// Runs the LVS tool on the provided input files.
    fn run_lvs(&self, input: LvsInput) -> Result<LvsOutput>;
}
