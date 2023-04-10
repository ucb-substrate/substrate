//! Standard cell and standard cell library error handling.

use thiserror::Error;

use super::{StdCellKey, StdCellLibKey};

/// An error type for the standard cell API.
#[derive(Debug, Error, Clone)]
pub enum StdCellError {
    #[error("no default library specified")]
    NoDefaultLibrary,

    #[error("no standard cell library with the given ID was found")]
    LibIdNotFound(StdCellLibKey),

    #[error("no standard cell library named `{0}` was found")]
    LibNameNotFound(String),

    #[error("no standard cell with the given ID was found in library `{lib}`")]
    CellIdNotFound { cell: StdCellKey, lib: String },

    #[error("no standard cell named `{cell}` was found in library `{lib}`")]
    CellNameNotFound { cell: String, lib: String },
}
