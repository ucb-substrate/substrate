use std::fmt::Display;

use super::cell::{BusPort, Cell, CellKey, PortId};
use crate::deps::arcstr::ArcStr;
use crate::log::Log;
use crate::validation::{Empty, ValidatorOutput};

/// Validates a layout cell.
pub fn validate_cell(cell: &Cell) -> LayoutValidatorOutput {
    LayoutValidator { cell }.validate()
}

pub struct LayoutValidator<'a> {
    cell: &'a Cell,
}

pub type LayoutValidatorOutput = ValidatorOutput<Empty, Empty, Error, Empty>;

/// An error location or net.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    cell: CellKey,
    cell_name: ArcStr,
    port_id: PortId,
}

impl Location {
    /// Creates a new [`Location`].
    pub fn new(cell: CellKey, cell_name: impl Into<ArcStr>, port_id: impl Into<PortId>) -> Self {
        Self {
            cell,
            cell_name: cell_name.into(),
            port_id: port_id.into(),
        }
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cell {}, port {}", self.cell_name, self.port_id)
    }
}

/// Data for an error.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Error {
    loc: Location,
    cause: ErrorCause,
}

impl Log for Error {
    fn log(&self) {
        use crate::log::error;
        error!("{self}");
    }
}

/// An enumeration of causes for an error.
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ErrorCause {
    /// Missing port in bus.
    MissingPort,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.cause {
            ErrorCause::MissingPort => {
                write!(f, "bus is missing a port: {}", self.loc)
            }
        }
    }
}

impl Error {
    /// Creates a new [`Error`].
    pub fn new(loc: Location, cause: ErrorCause) -> Self {
        Self { loc, cause }
    }
}

impl<'a> LayoutValidator<'a> {
    fn validate(&self) -> LayoutValidatorOutput {
        let mut output = LayoutValidatorOutput::default();
        self.validate_bus_ports(&mut output);
        output
    }

    /// Validates all bus ports at the top level of the cell.
    fn validate_bus_ports(&self, output: &mut LayoutValidatorOutput) {
        for (name, bus_port) in self.cell.bus_ports() {
            self.validate_bus_port(name, bus_port, output);
        }
    }

    /// Validates that a bus port consists of consecutive indices in the range 0..width.
    fn validate_bus_port(
        &self,
        name: &ArcStr,
        bus_port: &BusPort,
        output: &mut LayoutValidatorOutput,
    ) {
        let width = bus_port.len();

        for i in 0..width {
            if !bus_port.contains_key(&i) {
                output.errors.push(Error::new(
                    Location::new(self.cell.id(), self.cell.name(), PortId::new(name, i)),
                    ErrorCause::MissingPort,
                ));
            }
        }
    }
}
