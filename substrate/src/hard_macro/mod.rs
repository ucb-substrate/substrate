use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::deps::arcstr::ArcStr;
use crate::error::Result;
use crate::fmt::signal::BusFmt;
use crate::schematic::circuit::Direction;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub ports: HashMap<ArcStr, Port>,
    #[serde(default)]
    pub bus_format: BusFmt,
    #[serde(default)]
    pub spice_subckt_name: Option<ArcStr>,
    #[serde(default)]
    pub spice_path: Option<PathBuf>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Port {
    #[serde(default = "default_port_width")]
    pub width: usize,
    #[serde(default)]
    pub direction: Direction,
}

const fn default_port_width() -> usize {
    1
}

impl Config {
    pub fn from_toml(input: &str) -> Result<Self> {
        let value = toml::from_str(input)?;
        Ok(value)
    }

    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let input = std::fs::read_to_string(path)?;
        let mut value = Self::from_toml(&input)?;
        value.resolve_paths(path);
        Ok(value)
    }

    fn resolve_paths(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        if let Some(ref p) = self.spice_path {
            if p.is_relative() {
                self.spice_path = Some(path.parent().unwrap().join(p));
            }
        }
        println!("{:?}", self.spice_path);
    }
}
