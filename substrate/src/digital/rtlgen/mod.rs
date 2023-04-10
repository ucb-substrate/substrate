//! The interface for RTL generation from Substrate digital modules.
use thiserror::Error;

/// An enumeration of RTL generation errors.
#[derive(Debug, Error)]
pub enum RtlGenError {
    /// General I/O errors.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Unexpected errors.
    #[error("unexpected error: {0}")]
    Other(String),
}

/// The RTL generation `Result` type.
pub type Result<T> = std::result::Result<T, RtlGenError>;

/// The trait implemented by RTL generation plugins.
pub trait RtlGenerator {
    fn write_module() -> Result<()>;
}
