//! Verifies that module names are SPICE compatible and that there are no duplicate names.

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;

use crate::deps::arcstr::ArcStr;
use crate::log::Log;
use crate::schematic::context::ModuleKey;
use crate::schematic::module::{ExternalModule, Module};
use crate::schematic::netlist::preprocess::PreprocessedNetlist;
use crate::validation::{Empty, ValidatorOutput};

/// Validates the names of modules and module signals.
pub(crate) fn validate_naming(
    netlist: &PreprocessedNetlist,
    _ext_modules: &HashMap<ArcStr, Arc<ExternalModule>>,
) -> NamingValidatorOutput {
    NamingValidator {
        netlist,
        output: ValidatorOutput::default(),
    }
    .validate()
}

/// Verifies that all module and signal names are unique and reasonable.
pub struct NamingValidator<'a> {
    netlist: &'a PreprocessedNetlist,
    output: NamingValidatorOutput,
}

#[derive(Default)]
pub struct NamingValidatorData {
    // Empty for now.
}

impl Log for NamingValidatorData {
    fn log(&self) {
        // Empty for now.
    }
}

pub type NamingValidatorOutput = ValidatorOutput<Empty, Empty, Error, NamingValidatorData>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    module: ModuleKey,
    module_name: ArcStr,
}

/// Data for an error.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Error {
    loc: Location,
    cause: ErrorCause,
}

/// An enumeration of causes for an error.
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ErrorCause {
    /// The given name is invalid.
    ///
    /// This may be because it is not SPICE or GDS friendly.
    /// For example, SPICE identifiers cannot contain spaces.
    /// The rules for name validation are subject to change.
    InvalidName { name: ArcStr },
    /// Multiple modules have the same name.
    DuplicateModuleName { name: ArcStr },
    /// Multiple instances have the same name.
    DuplicateInstanceName { name: ArcStr },
    /// Multiple signals or ports within a module have the same name.
    DuplicateSignalName { name: ArcStr },
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "module {}", self.module_name)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.cause {
            ErrorCause::InvalidName { name } => {
                write!(f, "invalid name: `{}` in {}", name, self.loc)
            }
            ErrorCause::DuplicateModuleName { name } => {
                write!(f, "duplicate module name: `{}`", name)
            }
            ErrorCause::DuplicateInstanceName { name } => {
                write!(f, "duplicate instance name: `{}` in {}", name, self.loc)
            }
            ErrorCause::DuplicateSignalName { name } => {
                write!(f, "duplicate signal name: `{}` in {}", name, self.loc)
            }
        }
    }
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
    pub fn new(loc: Location, cause: ErrorCause) -> Self {
        Self { loc, cause }
    }
}

/// Checks if the given string `name` is a valid identifier.
fn is_valid_name(name: &str) -> bool {
    name.is_ascii() && !name.is_empty() && !name.contains(char::is_whitespace)
}

impl<'a> NamingValidator<'a> {
    fn validate(mut self) -> NamingValidatorOutput {
        let mut module_names = HashSet::with_capacity(self.netlist.modules.len());
        for module in self.netlist.modules.values() {
            self.validate_module(module, &mut module_names);
        }
        self.output
    }

    fn validate_name(
        &mut self,
        name: &ArcStr,
        set: &mut HashSet<ArcStr>,
        mut loc: impl FnMut() -> Location,
        mut duplicate_error: impl FnMut(ArcStr) -> ErrorCause,
    ) {
        if !is_valid_name(name) {
            self.output.errors.push(Error::new(
                loc(),
                ErrorCause::InvalidName { name: name.clone() },
            ));
        }

        if set.contains(name) {
            self.output
                .errors
                .push(Error::new(loc(), duplicate_error(name.clone())));
        }

        set.insert(name.clone());
    }

    /// Validates a single module.
    fn validate_module(&mut self, module: &Module, module_names: &mut HashSet<ArcStr>) {
        let loc = || Location {
            module: module.id(),
            module_name: module.name().clone(),
        };
        self.validate_name(module.name(), module_names, loc, |name| {
            ErrorCause::DuplicateModuleName { name }
        });

        let mut inst_names = HashSet::with_capacity(module.instances().len());
        for instance in module.instances() {
            self.validate_name(instance.name(), &mut inst_names, loc, |name| {
                ErrorCause::DuplicateInstanceName { name }
            });
        }

        let mut signal_names = HashSet::with_capacity(module.signals().len());
        for (_, signal) in module.signals() {
            self.validate_name(signal.name(), &mut signal_names, loc, |name| {
                ErrorCause::DuplicateSignalName { name }
            });
        }
    }
}
