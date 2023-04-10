//! Verifies that signal widths are matched and that all ports are connected correctly.

use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;

use crate::deps::arcstr::ArcStr;
use crate::log::Log;
use crate::schematic::circuit::{Instance, Reference};
use crate::schematic::context::ModuleKey;
use crate::schematic::module::{AbstractModule, ExternalModule, Module};
use crate::schematic::netlist::preprocess::PreprocessedNetlist;
use crate::validation::{Empty, ValidatorOutput};

/// Validates the connectivity of instances and ports.
///
/// Verifies that all instances have all ports connected,
/// and that all connections have the correct width.
pub(crate) fn validate_connectivity(
    netlist: &PreprocessedNetlist,
    ext_modules: &HashMap<ArcStr, Arc<ExternalModule>>,
) -> ConnectivityValidatorOutput {
    ConnectivityValidator {
        netlist,
        ext_modules,
        output: ValidatorOutput::default(),
    }
    .validate()
}

/// Verifies that all instances have all ports connected,
/// and that all connections have the correct width.
pub struct ConnectivityValidator<'a> {
    netlist: &'a PreprocessedNetlist,
    ext_modules: &'a HashMap<ArcStr, Arc<ExternalModule>>,
    output: ConnectivityValidatorOutput,
}

#[derive(Default)]
pub struct ConnectivityValidatorData {
    /// Modules that were skipped.
    ///
    /// Modules will be skipped if they contain raw spice literals.
    /// The [`ConnectivityValidator`] makes no attempt to parse raw spice
    /// and verify if it connects properly.
    skipped: Vec<(ModuleKey, ArcStr)>,
}

impl Log for ConnectivityValidatorData {
    fn log(&self) {
        use crate::log::info;

        for (_, name) in self.skipped.iter() {
            info!(
                "connectivity validation skipped module {name} (it may have a raw spice literal)"
            );
        }
    }
}

pub type ConnectivityValidatorOutput =
    ValidatorOutput<Empty, Empty, Error, ConnectivityValidatorData>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    module: ModuleKey,
    module_name: ArcStr,
    port_name: ArcStr,
    instance_name: ArcStr,
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
    /// Attempted to connect signals with different widths.
    WidthMismatch {
        conn_width: usize,
        port_width: usize,
    },
    /// An instance does not specify a connection for a particular port of a module.
    UnconnectedPort,
    /// Attempted to connect to a port that does not exist.
    NoSuchPort,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "module {}, instance {}, port {}",
            self.module_name, self.instance_name, self.port_name
        )
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause {
            ErrorCause::WidthMismatch {
                conn_width,
                port_width,
            } => write!(
                f,
                "mismatched widths at {}: connection has width {}, but port expects width {}",
                self.loc, conn_width, port_width
            ),
            ErrorCause::UnconnectedPort => {
                write!(f, "port is not connected: {}", self.loc)
            }
            ErrorCause::NoSuchPort => {
                write!(f, "attempted to connect to nonexistent port: {}", self.loc)
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

impl<'a> ConnectivityValidator<'a> {
    fn validate(mut self) -> ConnectivityValidatorOutput {
        for module in self.netlist.modules.values() {
            self.validate_module(module);
        }
        self.output
    }

    /// Validates a single module.
    fn validate_module(&mut self, module: &Module) {
        // Skip modules with raw spice literals.
        if module.raw_spice().is_some() {
            self.output
                .data
                .skipped
                .push((module.id, module.name().clone()));
            return;
        }

        for instance in module.instances() {
            match instance.module() {
                Reference::Local(key) => {
                    let submod = &self.netlist.modules[key.id()];
                    self.validate_instance(module, instance, submod);
                }
                Reference::External(name) => {
                    let submod = &self.ext_modules[&name];
                    self.validate_instance(module, instance, &**submod);
                }
            }
        }
    }

    /// Validates a single instance.
    fn validate_instance<T>(&mut self, module: &Module, instance: &Instance, submod: &T)
    where
        T: AbstractModule,
    {
        // The submodule specifies the ports that are available;
        // the instance must specify the connections to those ports.
        let mut map = HashSet::with_capacity(submod.raw_ports().len());

        // Check that ALL submodule ports are connected.
        for port in submod.raw_ports() {
            let info = submod.port_info(port);
            let port_name = info.name();

            // Defined as a closure to avoid needless cloning.
            // If we need the location, the closure will clone the `ArcStr`s.
            // If we don't need the location, the closure will never be called,
            // and we won't have had to issue any atomic instructions.
            let loc = || Location {
                module: module.id(),
                module_name: module.name().clone(),
                port_name: port_name.clone(),
                instance_name: instance.name().clone(),
            };

            let connected_to = instance.connections().get(port_name);

            if connected_to.is_none() {
                self.output.errors.push(Error {
                    loc: loc(),
                    cause: ErrorCause::UnconnectedPort,
                });
                continue;
            }

            let connected_to = connected_to.unwrap();
            let conn_width = connected_to.width();
            let port_width = info.width();
            if port_width != conn_width {
                self.output.errors.push(Error {
                    loc: loc(),
                    cause: ErrorCause::WidthMismatch {
                        conn_width,
                        port_width,
                    },
                });
            }

            map.insert(port_name);
        }

        // Check that ONLY the ports declared in the submodule have connections.
        for conn in instance.connections().keys() {
            if !map.contains(conn) {
                let loc = Location {
                    module: module.id(),
                    module_name: module.name().clone(),
                    port_name: conn.clone(),
                    instance_name: instance.name().clone(),
                };
                self.output.errors.push(Error {
                    loc,
                    cause: ErrorCause::NoSuchPort,
                })
            }
        }
    }
}
