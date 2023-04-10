use std::fmt::Display;

use super::types::TypeError;
use crate::log::Log;
use crate::validation::ValidatorOutput;

pub mod wire;

pub type TypeValidatorOutput = ValidatorOutput<Info, Warning, Error, TypeValidatorData>;

#[derive(Default)]
pub struct TypeValidatorData {}

impl Log for TypeValidatorData {
    fn log(&self) {}
}

/// Data for an info-level debug message.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Info {
    cause: InfoCause,
}

/// An enumeration of causes for an info message.
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum InfoCause {}

impl Log for Info {
    fn log(&self) {
        use crate::log::info;
        info!("{self}");
    }
}

impl Info {
    /// Creates a new [`Info`].
    pub fn new(cause: InfoCause) -> Self {
        Self { cause }
    }
}

impl Display for Info {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

/// Data for a warning.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Warning {
    cause: WarningCause,
}

/// An enumeration of causes for a warning.
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WarningCause {}

impl Log for Warning {
    fn log(&self) {
        use crate::log::warn;
        warn!("{self}");
    }
}

impl Warning {
    /// Creates a new [`Warning`].
    pub fn new(cause: WarningCause) -> Self {
        Self { cause }
    }
}

impl Display for Warning {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

/// Data for an error.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Error {
    cause: TypeError,
}

impl Log for Error {
    /// Logs the error to `stderr`.
    fn log(&self) {
        use crate::log::error;
        error!("{self}");
    }
}

impl Error {
    /// Creates a new [`Error`].
    pub fn new(cause: TypeError) -> Self {
        Self { cause }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
