//! Trait definitions and types for netlisters.

use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};
use slotmap::SlotMap;
use thiserror::Error;

use crate::deps::arcstr::ArcStr;
use crate::fmt::signal::BusFmt;
use crate::schematic::circuit::{Param, Port, Value};
use crate::schematic::signal::{Signal, SignalInfo, SignalKey};

/// Options describing the output of a nestlister.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct NetlistOpts {
    pub netlist_format: NetlistFormat,
    pub bus_format: BusFmt,
    pub global_ground_net: ArcStr,
}

/// An enumeration of supported netlist formats.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum NetlistFormat {
    /// Spectre netlist format.
    Spectre,
    /// Spectre-compatible SPICE netlist format.
    SpectreSpice,
    /// SPICE netlist format.
    Spice,
    /// NgSpice-compatible SPICE netlist format.
    NgSpice,
    /// A custom netlist format.
    Other(String),
}

impl Default for NetlistFormat {
    fn default() -> Self {
        Self::Spice
    }
}

impl Display for NetlistFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Spectre => write!(f, "spectre"),
            Self::SpectreSpice => write!(f, "spectre-spice"),
            Self::Spice => write!(f, "spice"),
            Self::NgSpice => write!(f, "ngspice"),
            Self::Other(ref s) => write!(f, "other-{s}"),
        }
    }
}

/// A trait representing the expected functionality of a netlister.
pub trait Netlister {
    /// Returns [`NetlistOpts`] describing the output of the netlister.
    fn opts(&self) -> NetlistOpts {
        NetlistOpts::default()
    }

    /// Emits a comment to the provided output stream.
    #[allow(unused_variables)]
    fn emit_comment(&self, out: &mut dyn Write, comment: &str) -> Result<()>;

    /// Emits a directive to begin a subcircuit to the provided output stream.
    fn emit_begin_subcircuit(&self, out: &mut dyn Write, info: SubcircuitInfo) -> Result<()>;

    /// Emits a directive to end a subcircuit to the provided output stream.
    fn emit_end_subcircuit(&self, out: &mut dyn Write, name: &str) -> Result<()>;

    /// Emits a raw SPICE directive to the provided output stream.
    fn emit_raw_spice(&self, out: &mut dyn Write, spice: &str) -> Result<()>;

    /// Emits an instance to the provided output stream.
    fn emit_instance(&self, out: &mut dyn Write, instance: InstanceInfo) -> Result<()>;

    /// Emits an include directive to the provided output stream.
    fn emit_include(&self, out: &mut dyn Write, include: &Path) -> Result<()>;

    /// Emits an library include directive to the provided output stream.
    fn emit_lib_include(&self, out: &mut dyn Write, lib: &Path, section: &str) -> Result<()>;

    /// Emits a prologue to the provided output stream.
    ///
    /// Called after `pdk.pre_netlist(...)`.
    #[allow(unused_variables)]
    fn emit_begin(&self, out: &mut dyn Write) -> Result<()> {
        Ok(())
    }

    /// Emits an epilogue to the provided output stream.
    #[allow(unused_variables)]
    fn emit_end(&self, out: &mut dyn Write) -> Result<()> {
        Ok(())
    }
}

/// A description of a schematic instance.
pub struct InstanceInfo<'a> {
    /// The instance name.
    pub name: &'a str,
    /// A list of instance ports.
    pub ports: &'a [&'a Signal],
    /// An unstructured map of parameters.
    pub params: &'a HashMap<ArcStr, Value>,
    /// A map of signals associated with the instance.
    pub signals: &'a SlotMap<SignalKey, SignalInfo>,
    /// The name of the subcircuit that the instance is associated with.
    pub subcircuit_name: &'a str,
}

/// A description of a schematic subcircuit.
pub struct SubcircuitInfo<'a> {
    /// The name of the subcircuit.
    pub name: &'a str,
    /// A list of ports associated with the subcircuit.
    pub ports: &'a [Port],
    /// An unstructured map of parameters.
    pub params: &'a HashMap<ArcStr, Param>,
    /// A map of signals associated with the subcircuit.
    pub signals: &'a SlotMap<SignalKey, SignalInfo>,
}

/// An enumeration of netlisting errors.
#[derive(Debug, Error)]
pub enum NetlistError {
    /// General I/O errors.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Unexpected errors.
    #[error("unexpected error: {0}")]
    Other(String),
}

/// The netlisting `Result` type.
pub type Result<T> = std::result::Result<T, NetlistError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::signal::{parse_bus, ParsedBus};

    #[test]
    fn test_parse_bus() {
        let format = BusFmt::DoubleDelimiter('[', ']');
        let parsed = parse_bus("input[1]", format).unwrap();
        assert_eq!(
            parsed,
            ParsedBus {
                name: "input",
                idx: 1
            }
        );

        let format = BusFmt::SingleDelimiter('_');
        let parsed = parse_bus("input_1", format).unwrap();
        assert_eq!(
            parsed,
            ParsedBus {
                name: "input",
                idx: 1
            }
        );
    }
}
