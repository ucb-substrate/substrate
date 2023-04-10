use std::fmt::Display;

use crate::log::Log;

/// The output of a validator.
#[derive(Debug)]
pub struct ValidatorOutput<I, W, E, D> {
    pub(crate) infos: Vec<I>,
    pub(crate) warnings: Vec<W>,
    pub(crate) errors: Vec<E>,
    /// Additional validation data.
    pub(crate) data: D,
}

/// Dummy struct for infos, warnings, or errors that do not yet exist.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Empty;

impl Log for Empty {
    fn log(&self) {}
}

impl<I, W, E, D> Default for ValidatorOutput<I, W, E, D>
where
    D: Default,
{
    fn default() -> Self {
        Self {
            infos: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            data: D::default(),
        }
    }
}

impl<I, W, E, D> ValidatorOutput<I, W, E, D>
where
    I: Log,
    W: Log,
    E: Log + Display,
    D: Log + Default,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Logs all stored info, warning, and error messages.
    pub fn log(&self) {
        self.data.log();

        for info in self.infos.iter() {
            info.log();
        }
        for warning in self.warnings.iter() {
            warning.log();
        }
        for error in self.errors.iter() {
            error.log();
        }
    }

    /// Returns `true` is any errors were encountered.
    #[inline]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns the first encountered error as a [`String`].
    pub fn first_error(&self) -> String {
        format!("{}", self.errors[0])
    }
}
