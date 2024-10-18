use std::fmt::{Display, Write};
use std::sync::Arc;

use db::MosDb;
use error::MosError;
use serde::{Deserialize, Serialize};
use spec::MosId;

use self::spec::MosKind;

pub mod db;
pub mod error;
pub mod query;
pub mod spec;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MosParams {
    pub w: i64,
    pub l: i64,
    pub m: u64,
    pub nf: u64,
    pub id: MosId,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LayoutMosParams {
    pub devices: Vec<MosParams>,
    /// A vector of finger indices to skip metallization on for each device.
    pub skip_sd_metal: Vec<Vec<usize>>,
    pub deep_nwell: bool,
    pub contact_strategy: GateContactStrategy,
}

/// Specifies the geometric arrangement of contacts for transistor gates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateContactStrategy {
    /// Attempt to place all contacts on one side (usually the left)
    SingleSide,
    /// Attempt to place contacts on both sides of the transistor.
    BothSides,
    /// Alternate contact placement
    Alternate,
    /// ABBA placement
    Abba,
    /// Other; effect depends on layout generator
    Other(String),
}

impl<'a> MosParams {
    pub fn kind(&self, mos_db: &Arc<MosDb>) -> MosKind {
        mos_db.get_spec(self.id).unwrap().kind
    }

    pub fn name(&self, mos_db: &'a Arc<MosDb>) -> &'a str {
        &mos_db.get_spec(self.id).unwrap().name
    }
}

impl Display for MosParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "w{}_l{}_m{}_nf{}_id{}",
            self.w, self.l, self.m, self.nf, self.id
        )
    }
}

impl LayoutMosParams {
    pub fn validate(&self) -> Result<(), MosError> {
        if self.devices.is_empty() {
            return Err(MosError::NoDevices);
        }

        let start = &self.devices[0];
        if start.nf as i64 <= 0 {
            return Err(MosError::InvalidNumFingers(start.nf));
        }

        for device in self.devices.iter().skip(1) {
            if device.l != start.l {
                return Err(MosError::MismatchedLengths);
            } else if device.nf != start.nf {
                return Err(MosError::MismatchedFingers);
            }
        }

        Ok(())
    }

    pub fn name(&self, mos_db: &Arc<MosDb>) -> String {
        let mut name = String::new();
        write!(&mut name, "ptx").unwrap();

        for device in self.devices.iter() {
            write!(&mut name, "__{}", device.name(&Arc::clone(mos_db))).unwrap();
        }

        name
    }

    pub fn fingers(&self) -> u64 {
        self.devices[0].nf
    }

    pub fn length(&self) -> i64 {
        self.devices[0].l
    }
}
