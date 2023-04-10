//! DRC plugin API.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::deps::arcstr::ArcStr;
use crate::error::Result;
use crate::layout::LayoutFormat;

/// Inputs passed to a [`DrcTool`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DrcInput {
    /// The name of the cell to run DRC on.
    pub cell_name: ArcStr,
    /// The directory to place intermediate and output files.
    pub work_dir: PathBuf,
    /// The path to the layout file containing the cell of interest.
    pub layout_path: PathBuf,
    /// The format of the layout file.
    pub layout_format: LayoutFormat,
    /// Unstructured options.
    pub opts: HashMap<ArcStr, ArcStr>,
}

/// An enumeration describing the high-level result of a DRC run.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DrcSummary {
    /// DRC run passed.
    Pass,
    /// DRC run passed with warnings.
    Warn,
    /// DRC run failed.
    Fail,
}

impl DrcSummary {
    /// Checks if a [`DrcSummary`] describes a passing DRC run.
    pub fn is_ok(&self) -> bool {
        match self {
            Self::Pass | Self::Warn => true,
            Self::Fail => false,
        }
    }
}

/// A DRC error.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DrcError {
    /// The name of the error.
    pub name: ArcStr,
    /// An optional description of the error.
    pub desc: Option<ArcStr>,
    /// The Cartesian coordinates of the error.
    pub location: Option<(i64, i64)>,
}

/// Outputs emitted by a [`DrcTool`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DrcOutput {
    /// A summary of the DRC run.
    pub summary: DrcSummary,
    /// A list of errors encountered during the DRC run.
    pub errors: Vec<DrcError>,
}

/// The trait that DRC plugins must implement.
pub trait DrcTool {
    /// Runs the DRC tool on the provided input files.
    fn run_drc(&self, input: DrcInput) -> Result<DrcOutput>;
}
