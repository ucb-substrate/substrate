//! Conversion error types.

use crate::deps::arcstr::ArcStr;

/// A helper trait for conversion tree-walkers.
///
/// Each implementer will generally have some internal state to report upon failure,
/// which it can inject in the implementation-required `err` method.
/// The `fail` method, provided by default, simply returns the `err` value.
pub trait ErrorHelper {
    type Error;

    /// Creates and returns a [Self::Error] value.
    fn err(&self, msg: impl Into<String>) -> Self::Error;
    /// Returns the given failure message.
    fn fail<T>(&self, msg: impl Into<String>) -> Result<T, Self::Error> {
        Err(self.err(msg))
    }
    /// Unwraps the [`Option`] `opt` if it is [`Some`] and returns an error if not.
    fn unwrap<T>(&self, opt: Option<T>, msg: impl Into<String>) -> Result<T, Self::Error> {
        match opt {
            Some(val) => Ok(val),
            None => self.fail(msg),
        }
    }
    /// Asserts boolean condition `b`. Returns through `self.fail` if not.
    fn assert(&self, b: bool, msg: impl Into<String>) -> Result<(), Self::Error> {
        match b {
            true => Ok(()),
            false => self.fail(msg),
        }
    }
    /// Unwraps the [`Result`] `res`. Returns through our failure method if it is [`Err`].
    /// Optional method, but must be implemented to be (usefully) called.
    /// The default implementation simply returns an error via `self.fail`.
    fn ok<T, E>(&self, _res: Result<T, E>, msg: impl Into<String>) -> Result<T, Self::Error>
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        self.fail(msg) // Default version always fails.
    }
}
/// A conversion context.
///
/// This enumeration is generally used for error reporting.
#[derive(Debug, Clone)]
pub enum ErrorContext {
    Library,
    Cell(ArcStr),
    Abstract,
    Impl,
    Instance(ArcStr),
    Array(ArcStr),
    Units,
    Geometry,
    Annotations,
    Ports,
    Unknown,
}
