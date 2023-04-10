use std::collections::HashMap;
use std::fmt::Display;
use std::num::ParseIntError;
use std::sync::{Arc, RwLock};

use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::deps::arcstr::ArcStr;

/// An enumeration of bus formatting styles.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum BusFmt {
    /// Delimits the bus index using two characters, eg. `data[3]`.
    DoubleDelimiter(char, char),

    /// Delimits the bus index using one character, eg. `data_3`.
    SingleDelimiter(char),
}

impl Default for BusFmt {
    fn default() -> Self {
        Self::DoubleDelimiter('[', ']')
    }
}

struct Escape(char);

impl Display for Escape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = self.0;
        if c == '\\'
            || c == '.'
            || c == '+'
            || c == '*'
            || c == '?'
            || c == '('
            || c == ')'
            || c == '|'
            || c == '['
            || c == ']'
            || c == '{'
            || c == '}'
            || c == '^'
            || c == '$'
        {
            write!(f, "\\{c}")
        } else {
            write!(f, "{c}")
        }
    }
}

impl BusFmt {
    pub fn regex(&self) -> Regex {
        use BusFmt::*;
        let regex = match *self {
            DoubleDelimiter(a, b) => {
                format!("^(?P<name>.+){}(?P<idx>\\d+){}$", Escape(a), Escape(b))
            }
            SingleDelimiter(d) => format!("^(?P<name>.+){}(?P<idx>\\d+)$", Escape(d)),
        };
        println!("regex = {regex}");
        Regex::new(&regex).expect("failed to compile bus parsing regex")
    }
}

pub fn format_signal(name: impl Into<ArcStr>, idx: usize, width: usize, format: BusFmt) -> ArcStr {
    let name = name.into();
    if width == 1 {
        name
    } else {
        format_bus(&name, idx, format)
    }
}

pub fn format_bus(name: &str, idx: usize, format: BusFmt) -> ArcStr {
    use BusFmt::*;
    match format {
        DoubleDelimiter(a, b) => arcstr::format!("{name}{a}{idx}{b}"),
        SingleDelimiter(d) => arcstr::format!("{name}{d}{idx}"),
    }
}

#[derive(Debug, Error)]
pub enum ParseBusError {
    #[error("invalid bus syntax")]
    InvalidSyntax,

    #[error("error parsing integer: {0}")]
    ParseInt(#[from] ParseIntError),
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug, Serialize)]
pub struct ParsedBus<'a> {
    pub(crate) name: &'a str,
    pub(crate) idx: usize,
}

lazy_static! {
    static ref BUS_PARSING_REGEXES: Arc<RwLock<HashMap<BusFmt, Regex>>> =
        Arc::new(RwLock::new(HashMap::new()));
}

fn get_regex(format: BusFmt) -> Regex {
    let map = BUS_PARSING_REGEXES.read().unwrap();
    if let Some(regex) = map.get(&format) {
        return regex.clone();
    }
    drop(map);
    let regex = format.regex();
    let mut map = BUS_PARSING_REGEXES.write().unwrap();
    map.insert(format, regex.clone());
    drop(map);
    regex
}

pub fn parse_bus(text: &str, format: BusFmt) -> std::result::Result<ParsedBus, ParseBusError> {
    let re = get_regex(format);
    let caps = re
        .captures(text.trim())
        .ok_or(ParseBusError::InvalidSyntax)?;
    let idx = caps.name("idx").ok_or(ParseBusError::InvalidSyntax)?;
    let idx = idx.as_str().parse::<usize>()?;
    let name = caps
        .name("name")
        .ok_or(ParseBusError::InvalidSyntax)?
        .as_str();

    Ok(ParsedBus { name, idx })
}
