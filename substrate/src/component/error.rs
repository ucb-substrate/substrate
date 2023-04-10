//! Error types for creating SubComponents.

use thiserror::Error;

use super::View;

/// An error for the SubComponent API.
#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("unsupported view: {0:?}")]
    ViewUnsupported(View),

    #[error("invalid params")]
    InvalidParams,
}

/// A result for the SubComponent API.
pub type Result<T> = std::result::Result<T, Error>;
