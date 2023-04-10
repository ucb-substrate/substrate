use error::{Error, Result};
use parser::{SpiceLine, SubcktLine};
use serde::Serialize;

pub mod error;
pub mod parser;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ParsedSpice<'a> {
    pub lines: Vec<SpiceLine<'a>>,
}

/// Parse the given rawfile data.
pub fn parse<T>(input: &T) -> Result<ParsedSpice<'_>>
where
    T: AsRef<str>,
{
    match parser::parse_spice(input.as_ref()) {
        Ok((_, lines)) => Ok(ParsedSpice { lines }),
        Err(_) => Err(Error::Parse),
    }
}

impl<'a> ParsedSpice<'a> {
    /// Return an iterator over the lines in the parsed SPICE netlist.
    pub fn lines(&self) -> impl Iterator<Item = &SpiceLine> {
        self.lines.iter()
    }

    /// Return an iterator over the subcircuit definitions in the netlist.
    pub fn subcircuits(&self) -> impl Iterator<Item = &SubcktLine> {
        self.lines.iter().filter_map(|line| line.subckt())
    }

    /// Return the subcircuit definition with the given name.
    ///
    /// Note that this operation takes `O(N)` time, where `N`
    /// is the number of lines in the parsed netlist.
    ///
    /// If you need to query for multiple subcircuits, you may wish to
    /// collect the [`subcircuits`](ParsedSpice::subcircuits) into a
    /// [`HashMap`](std::collections::HashMap), and query the map instead.
    pub fn subcircuit_named(&self, name: impl AsRef<str>) -> Option<&SubcktLine> {
        let name = name.as_ref();
        self.subcircuits().find(|ckt| ckt.name == name)
    }
}
