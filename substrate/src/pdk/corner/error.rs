//! Standard cell and standard cell library error handling.

use thiserror::Error;

use super::CornerKey;

/// An error type for the corner API.
#[derive(Debug, Error, Clone)]
pub enum ProcessCornerError {
    #[error("no default process corner specified")]
    NoDefaultCorner,

    #[error("no process corner with the given ID was found")]
    CornerIdNotFound(CornerKey),

    #[error("no process corner named `{0}` was found")]
    CornerNameNotFound(String),
}
