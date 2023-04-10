//! Queries to select MOS devices.

use derive_builder::Builder;

use super::spec::{MosFlavor, MosId, MosKind, MosSpec};
use crate::pdk::SupplyId;

/// A query for selecting a MOS device from the ones available in a PDK.
#[derive(Debug, Clone, Eq, PartialEq, Builder)]
pub struct Query {
    #[builder(default)]
    pub(crate) kind: MosKind,
    #[builder(default)]
    pub(crate) flavor: MosFlavor,
    #[builder(default)]
    pub(crate) supply: SupplyId,
    /// Whether or not to require an exact match.
    ///
    /// If set to `false`, queries may return alternative
    /// devices that still match the requested conditions.
    ///
    /// For example, if `exact` is `false` and a high-Vt device
    /// is requested, a standard-Vt device may be returned if
    /// no high-Vt device is available.
    #[builder(default)]
    pub(crate) exact: bool,
}

pub struct QueryResult<'a> {
    /// The ID of the MOSFET selected.
    pub(crate) id: MosId,
    /// Information about the selected MOSFET.
    pub(crate) spec: &'a MosSpec,
    /// Indicates if query conditions were relaxed.
    pub(crate) alternate: bool,
}

impl Query {
    /// Creates a [`QueryBuilder`].
    #[inline]
    pub fn builder() -> QueryBuilder {
        QueryBuilder::default()
    }
}

impl<'a> QueryResult<'a> {
    #[inline]
    pub fn spec(&self) -> &MosSpec {
        self.spec
    }

    #[inline]
    pub fn id(&self) -> MosId {
        self.id
    }

    #[inline]
    pub fn alternate(&self) -> bool {
        self.alternate
    }
}
