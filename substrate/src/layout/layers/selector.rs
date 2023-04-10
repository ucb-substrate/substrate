//! The `Selector` type for PDK layer selection.

use serde::Serialize;

use super::GdsLayerSpec;

/// An enumeration for selecting layers in a PDK.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize)]
pub enum Selector<'a> {
    /// The n'th metal layer.
    Metal(usize),
    /// The n'th routing layer.
    ///
    /// This may be different from `Metal(n)`.
    /// For example, some processes may consider poly a routing layer
    /// but not a metal layer.
    Routing(usize),
    /// The via layer connecting metal `N + 1` to metal `N`.
    ///
    /// For example, `Via(2)` should connect metal 3 to metal 2.
    Via(usize),
    /// The layer with the given name.
    Name(&'a str),
    /// The layer containing the given GDS spec.
    ///
    /// Note that "layers" in Substrate consist of multiple GDS
    /// specs. For example, "m1 pin", "m1 drawing", and "m1 label"
    /// may all be grouped under the layer "m1".
    Gds(GdsLayerSpec),
}
