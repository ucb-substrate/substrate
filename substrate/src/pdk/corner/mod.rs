use std::sync::Arc;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use self::error::ProcessCornerError;
use crate::deps::arcstr::ArcStr;

pub mod error;

new_key_type! {
    /// A unique identifier for [process corners](StdCellInfo).
    pub struct CornerKey;
}

///
#[derive(Debug, Clone, PartialEq, Builder, Serialize, Deserialize)]
pub struct Pvt {
    /// The process corner.
    corner: CornerEntry,
    /// Supply voltage, in volts.
    voltage: f64,
    /// Temperature, in degrees Celsius.
    temp: f64,
}

#[derive(Debug, Clone, Eq, PartialEq, Builder, Serialize, Deserialize)]
pub struct CornerEntry {
    id: CornerKey,
    data: Arc<CornerData>,
}

#[derive(Debug, Clone, Eq, PartialEq, Builder, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CornerData {
    #[builder(setter(into))]
    name: ArcStr,
    #[builder(default, setter(strip_option))]
    nmos: Option<CornerSkew>,
    #[builder(default, setter(strip_option))]
    pmos: Option<CornerSkew>,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
pub enum CornerSkew {
    Slow,
    #[default]
    Typical,
    Fast,
}

impl Pvt {
    #[inline]
    pub fn new(corner: CornerEntry, voltage: f64, temp: f64) -> Self {
        Self {
            corner,
            voltage,
            temp,
        }
    }
}

#[derive(Debug)]
pub struct CornerDb {
    corners: SlotMap<CornerKey, CornerEntry>,
    default_corner: Option<CornerKey>,
}

impl CornerData {
    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    #[inline]
    pub fn builder() -> CornerDataBuilder {
        CornerDataBuilder::default()
    }

    #[inline]
    pub fn nmos(&self) -> Option<CornerSkew> {
        self.nmos
    }

    #[inline]
    pub fn pmos(&self) -> Option<CornerSkew> {
        self.pmos
    }
}

impl CornerEntry {
    #[inline]
    pub fn name(&self) -> &ArcStr {
        self.data.name()
    }

    #[inline]
    pub fn id(&self) -> CornerKey {
        self.id
    }

    #[inline]
    pub fn nmos(&self) -> Option<CornerSkew> {
        self.data.nmos()
    }

    #[inline]
    pub fn pmos(&self) -> Option<CornerSkew> {
        self.data.pmos()
    }
}

impl Default for CornerDb {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl CornerDb {
    pub fn new() -> Self {
        Self {
            corners: SlotMap::with_key(),
            default_corner: None,
        }
    }

    #[inline]
    pub fn add_corner(&mut self, corner: CornerData) -> CornerKey {
        self.corners.insert_with_key(move |k| CornerEntry {
            id: k,
            data: Arc::new(corner),
        })
    }

    #[inline]
    pub fn set_default_corner(&mut self, id: CornerKey) {
        self.default_corner = Some(id);
    }

    #[inline]
    pub fn default_corner(&self) -> Option<&CornerEntry> {
        self.corner(self.default_corner?)
    }

    #[inline]
    pub fn try_default_corner(&self) -> crate::error::Result<&CornerEntry> {
        self.default_corner()
            .ok_or_else(|| ProcessCornerError::NoDefaultCorner.into())
    }

    pub fn corner_named(&self, name: &str) -> Option<&CornerEntry> {
        self.corners.values().find(|l| l.name() == name)
    }

    pub fn try_corner_named(&self, name: &str) -> crate::error::Result<&CornerEntry> {
        self.corner_named(name)
            .ok_or_else(|| ProcessCornerError::CornerNameNotFound(name.to_string()).into())
    }

    #[inline]
    pub fn corner(&self, id: CornerKey) -> Option<&CornerEntry> {
        self.corners.get(id)
    }

    pub fn try_corner(&self, id: CornerKey) -> crate::error::Result<&CornerEntry> {
        self.corner(id)
            .ok_or_else(|| ProcessCornerError::CornerIdNotFound(id).into())
    }

    #[inline]
    pub fn corners(&self) -> impl Iterator<Item = &CornerEntry> + '_ {
        self.corners.values()
    }
}
