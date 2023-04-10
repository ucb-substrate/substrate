use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::pdk::SupplyId;

#[derive(Default, Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MosId(u64);

impl MosId {
    #[inline]
    pub fn new(inner: u64) -> Self {
        Self(inner)
    }

    #[inline]
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Display for MosId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Default, Clone, Debug)]
pub struct MosSpec {
    pub id: MosId,
    pub name: String,
    pub lmin: i64,
    pub wmin: i64,
    pub lmax: Option<i64>,
    pub wmax: Option<i64>,
    pub kind: MosKind,
    pub flavor: MosFlavor,
    pub supply: SupplyId,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MosKind {
    #[default]
    Nmos,
    Pmos,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MosFlavor {
    /// Indicates that the user wants the lowest threshold
    /// MOSFET available, whether that is Svt, Lvt, Ulvt, etc.
    Lowest,
    Ulvt,
    Lvt,
    #[default]
    Svt,
    Hvt,
    Uhvt,
    /// Indicates that the user wants the highest threshold
    /// MOSFET available, whether that is Svt, Hvt, Uhvt, etc.
    Highest,
}
