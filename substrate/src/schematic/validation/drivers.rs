//! Verifies that nets are driven appropriately.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use slotmap::SecondaryMap;

use super::super::circuit::{Direction, Instance, Reference};
use super::super::context::ModuleKey;
use super::super::module::{AbstractModule, ExternalModule, Module};
use super::super::netlist::preprocess::PreprocessedNetlist;
use super::super::signal::SignalKey;
use crate::deps::arcstr::ArcStr;
use crate::log::Log;
use crate::validation::ValidatorOutput;

/// Validates the number of drivers on each net.
pub(crate) fn validate_drivers(
    netlist: &PreprocessedNetlist,
    ext_modules: &HashMap<ArcStr, Arc<ExternalModule>>,
) -> DriverValidatorOutput {
    DriverValidator {
        netlist,
        ext_modules,
    }
    .validate()
}

/// Validates that nets have exactly one output driver
/// or have possibly multiple inout drivers.
pub struct DriverValidator<'a> {
    netlist: &'a PreprocessedNetlist,
    ext_modules: &'a HashMap<ArcStr, Arc<ExternalModule>>,
}

#[derive(Default)]
pub struct DriverValidatorData {
    /// Modules that were skipped.
    ///
    /// Modules will be skipped if they contain raw spice literals.
    /// The [`DriverValidator`] makes no attempt to parse raw spice
    /// and verify if it connects properly.
    skipped: Vec<(ModuleKey, ArcStr)>,
}

impl Log for DriverValidatorData {
    fn log(&self) {
        use crate::log::info;

        for (_, name) in self.skipped.iter() {
            info!("validation skipped module {name} (it may have a raw spice literal)");
        }
    }
}

pub type DriverValidatorOutput = ValidatorOutput<Info, Warning, Error, DriverValidatorData>;

/// An error location or net.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    module: ModuleKey,
    module_name: ArcStr,
    signal: SignalKey,
    signal_name: ArcStr,
    idx: usize,
}

impl Location {
    /// Creates a new [`Location`].
    pub fn new(
        module: ModuleKey,
        module_name: impl Into<ArcStr>,
        signal: SignalKey,
        signal_name: impl Into<ArcStr>,
        idx: usize,
    ) -> Self {
        Self {
            module,
            module_name: module_name.into(),
            signal,
            signal_name: signal_name.into(),
            idx,
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "module {}, signal {}, idx {}",
            self.module_name, self.signal_name, self.idx
        )
    }
}

/// Data for an info-level debug message.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Info {
    loc: Location,
    cause: InfoCause,
}

/// An enumeration of causes for an info message.
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum InfoCause {
    /// A net that is driven, but is not tapped.
    NotConnected,
}

impl Log for Info {
    fn log(&self) {
        use crate::log::info;
        info!("{self}");
    }
}

/// Data for a warning.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Warning {
    loc: Location,
    cause: WarningCause,
}

/// An enumeration of causes for a warning.
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WarningCause {
    /// A net that is declared but not connected to anything.
    Floating,
    /// A net that has multiple drivers.
    MultipleDrivers,
}

impl Log for Warning {
    fn log(&self) {
        use crate::log::warn;
        warn!("{self}");
    }
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
    /// A net that is tapped but has no drivers.
    NoDriver,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause {
            ErrorCause::NoDriver => {
                write!(
                    f,
                    "net is used as an input, but has no drivers: {}",
                    self.loc
                )
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

impl Warning {
    /// Creates a new [`Warning`].
    pub fn new(loc: Location, cause: WarningCause) -> Self {
        Self { loc, cause }
    }
}

impl Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause {
            WarningCause::MultipleDrivers => write!(f, "net has multiple drivers: {}", self.loc),
            WarningCause::Floating => write!(
                f,
                "net is declared but not connected to anything: {}",
                self.loc
            ),
        }
    }
}

impl Info {
    /// Creates a new [`Info`].
    pub fn new(loc: Location, cause: InfoCause) -> Self {
        Self { loc, cause }
    }
}

impl Display for Info {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause {
            InfoCause::NotConnected => write!(
                f,
                "net is driven but not connected to anything else: {}",
                self.loc
            ),
        }
    }
}

/// The state of a net.
#[derive(Debug, Clone, Default)]
struct NetState {
    /// The number of drivers on this net.
    ///
    /// A module input port counts as a driver,
    /// since an input port is presumably driven
    /// by some other module.
    drivers: usize,
    /// The number of "readers" on this net.
    ///
    /// A module output port counts as a tap,
    /// since an output port is presumably read
    /// by some other module.
    taps: usize,
    /// The number of inouts connected to this net.
    ///
    /// A module inout port counts as a tap.
    inouts: usize,
}

impl NetState {
    /// Creates a new [`NetState`].
    #[inline]
    fn new() -> Self {
        Self::default()
    }

    /// Returns the number of connections to the net.
    fn degree(&self) -> usize {
        self.drivers + self.taps + self.inouts
    }

    /// Returns the effective number of drivers of the net.
    ///
    /// Counts inouts as drivers.
    fn eff_drivers(&self) -> usize {
        self.inouts + self.drivers
    }

    /// Validates the number of drivers, taps, and inouts on the net.
    fn validate(&self, loc: Location, output: &mut DriverValidatorOutput) {
        if self.drivers > 1 {
            output
                .warnings
                .push(Warning::new(loc.clone(), WarningCause::MultipleDrivers));
        }

        if self.taps > 0 && self.inouts + self.drivers == 0 {
            output
                .errors
                .push(Error::new(loc.clone(), ErrorCause::NoDriver));
        }

        if self.degree() == 0 {
            output
                .warnings
                .push(Warning::new(loc.clone(), WarningCause::Floating));
        }

        if self.taps == 0 && self.eff_drivers() == 1 {
            output.infos.push(Info::new(loc, InfoCause::NotConnected));
        }
    }
}

impl<'a> DriverValidator<'a> {
    /// Validates all of the modules in the provided netlist.
    fn validate(&self) -> DriverValidatorOutput {
        let mut output = DriverValidatorOutput::default();
        for module in self.netlist.modules.values() {
            self.validate_module(module, &mut output);
        }
        output
    }

    /// Validates a single module.
    fn validate_module(&self, module: &Module, output: &mut DriverValidatorOutput) {
        // Skip modules with raw spice literals.
        if module.raw_spice().is_some() {
            output.data.skipped.push((module.id, module.name().clone()));
            return;
        }

        let mut net_states: SecondaryMap<SignalKey, Vec<NetState>> =
            SecondaryMap::with_capacity(module.signals().capacity());
        for (key, info) in module.signals().iter() {
            net_states.insert(key, vec![NetState::new(); info.width()]);
        }

        for port in module.ports() {
            for net in net_states[port.signal].iter_mut() {
                match port.direction {
                    Direction::Input => net.drivers += 1,
                    Direction::Output => net.taps += 1,
                    Direction::InOut => net.inouts += 1,
                };
            }
        }

        for instance in module.instances() {
            match instance.module() {
                Reference::Local(key) => {
                    let submod = &self.netlist.modules[key.id()];
                    self.validate_instance(&mut net_states, instance, submod);
                }
                Reference::External(name) => {
                    let submod = &self.ext_modules[&name];
                    self.validate_instance(&mut net_states, instance, &**submod);
                }
            }
        }

        for (sig, states) in net_states {
            for (i, state) in states.iter().enumerate() {
                let loc = Location::new(
                    module.id,
                    module.name(),
                    sig,
                    module.signals()[sig].name(),
                    i,
                );
                state.validate(loc, output);
            }
        }
    }

    /// Validates a single instance.
    fn validate_instance<T>(
        &self,
        net_states: &mut SecondaryMap<SignalKey, Vec<NetState>>,
        instance: &Instance,
        submod: &T,
    ) where
        T: AbstractModule,
    {
        for port in submod.raw_ports() {
            let info = submod.port_info(port);
            let name = info.name();
            let connected_to = instance.connections().get(name).unwrap_or_else(|| {
                panic!(
                    "No connection found for port {name} on instance {}",
                    instance.name()
                )
            });
            for part in connected_to.parts() {
                for idx in part.range() {
                    let state = &mut net_states[part.signal()][idx];
                    match port.direction {
                        Direction::Input => state.taps += 1,
                        Direction::Output => state.drivers += 1,
                        Direction::InOut => state.inouts += 1,
                    };
                }
            }
        }
    }
}
