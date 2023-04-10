//! Power straps.
//!
//! # Structure
//!
//! Power straps in Substrate are composed of repeating arrays of **groups**.
//! Each group generally contains 2 or more power straps.
//!
//! Straps within a group are usually closely-spaced, whereas there is
//! usually a larger spacing between groups.
//!
//! The most common group configuration is `[Vdd, Vss]`. This means that each group
//! has one `Vdd` strap followed by one `Vss` strap. The first net in this ordering
//! is the leftmost/bottommost strap.
//!
//! More complex configurations are also possible, such as `[Vdd, Vdd, Vss, Vss, Vss]`.
//! This configuration produces groups with 5 straps. The leftmost/bottommost 2 straps
//! supply power; the remaining 3 are grounded.
//!
//! # Hammer Integration
//!
//! Substrate can import power straps from Hammer.
//! See the Hammer documentation for the `par.power_straps_abutment` flag for more information.
//!
//! When this flag is set, Hammer will export power strap information to a JSON file that Substrate
//! can read using [`StrapConfig::from_hammer_json_file`].
//!
//! Substrate interprets Hammer power strap data as follows:
//! * The first strap of the first group starts at `offset`. That is, it's leftmost/bottommost edge
//!   is positioned at the coordinate `offset`.
//! * The center-to-center (or equivalently, edge-to-edge) spacing between groups is `group_pitch`.
//! * Nets in `net_order` are written in left-to-right or bottom-to-top order.
//! * The fields `inst_paths` and `inst_orientations` are subject to change.
//! * Layer names in the Hammer JSON file must match layer names in the PDK library used
//!   by the Substrate context.
//!
//! # Specifying Net Ordering
//!
//! The types in this module ([`StrapConfig`] in particular) are generic over
//! parameter `N`, which indicates the net to which each strap is connected.
//!
//! The default net type is `SingleSupplyNet`, which supports power straps where each
//! net is either `Vdd` or `Vss`.
//!
//! If you need to define custom nets, such as if you would like to support having multiple
//! supplies, you can implement your own net enumeration. It should implement [`Clone`].
//!
//! Note that you do not need to define a custom net type if you want different **quantities**
//! of nets. You only need to do this if you need different **types** of nets.
//!
//! So a custom definition is necessary if your net order contains `[VDDA, VDDB, VSS]`,
//! but not if your net order is `[VSS, VDD, VDD, VSS, VDD, VSS]`.
//!
//! Custom net types should implement [`FromStr`] so that they can be parsed from strings
//! (such as the strings specified in Hammer-generated JSON).
//!
use std::cmp::Ordering;
use std::error::Error;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use subgeom::{Dir, DirParseError, Rect, Span};
use thiserror::Error;

use super::cell::Element;
use super::group::elements::ElementGroup;
use super::layers::selector::Selector;
use super::layers::{LayerKey, LayerSpec};
use crate::data::SubstrateCtx;
use crate::error::{Result, SubstrateError};

mod hammer;
mod parse;

/// Net type enumeration for power straps with a single supply.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
pub enum SingleSupplyNet {
    Vdd,
    Vss,
}

/// Error from parsing VDD or VSS from a string.
#[derive(Debug, Error)]
#[error("error parsing net name `{0}`; expected VDD or VSS")]
pub struct SingleSupplyNetParseError(String);

impl FromStr for SingleSupplyNet {
    type Err = SingleSupplyNetParseError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let lowercase = s.to_lowercase();
        match lowercase.trim() {
            "vdd" | "vpwr" | "pwr" | "vcc" => Ok(Self::Vdd),
            "vss" | "vgnd" | "gnd" => Ok(Self::Vss),
            _ => Err(SingleSupplyNetParseError(s.to_string())),
        }
    }
}

/// Layer strap configuration for a single layer.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct LayerStraps<N = SingleSupplyNet> {
    layer: LayerKey,
    /// Index of the metal layer.
    index: usize,
    dir: Dir,
    nets: Vec<N>,
    width: i64,
    spacing: i64,
    group_pitch: i64,
    offset: i64,
}

/// A single power strap, with a known net (usually either Vdd or Vss).
#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Strap<N = SingleSupplyNet> {
    span: Span,
    net: N,
}

/// A single group of straps, corresponding to the nets specified in `net_order`.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct StrapGroup<N = SingleSupplyNet> {
    straps: Vec<Strap<N>>,
}

/// Power strap configuration for a single macro.
#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct StrapConfig<N = SingleSupplyNet> {
    /// The configuration for the power straps on this macro's top layer.
    top: LayerStraps<N>,
    /// The configuration for the power straps on the layer above this
    /// macro's top layer.
    ///
    /// If no top layer is available, this will be [`None`]. Otherwise,
    /// it should be [`Some`].
    above_top: Option<LayerStraps<N>>,
}

impl<N: FromStr> LayerStraps<N>
where
    N::Err: Error + Send + Sync + 'static,
{
    pub fn from_hammer_straps(straps: &hammer::StrapInfo, ctx: &SubstrateCtx) -> Result<Self> {
        let layers = ctx.layers();
        let layer = layers.get(Selector::Name(straps.layer))?;
        let info = layers.info(layer)?;
        let index = info
            .metal_idx
            .ok_or_else(|| PowerStrapError::NotMetalLayer(straps.layer.to_string()))?;

        let nets = straps
            .net_order
            .iter()
            .map(|n| n.parse::<N>())
            .collect::<std::result::Result<Vec<_>, N::Err>>()
            .map_err(|e| PowerStrapError::InvalidNet(Box::new(e)))?;

        let dir = straps
            .direction
            .parse()
            .map_err(PowerStrapError::InvalidDirection)?;

        Ok(Self {
            layer,
            index,
            dir,
            nets,
            width: straps.width,
            spacing: straps.spacing,
            group_pitch: straps.group_pitch,
            offset: straps.offset,
        })
    }
}

fn usize_as_i64(x: usize) -> i64 {
    i64::try_from(x).unwrap()
}

impl<N> Strap<N> {
    #[inline]
    pub fn new(net: N, span: Span) -> Self {
        Self { span, net }
    }
    #[inline]
    pub fn net(&self) -> &N {
        &self.net
    }

    #[inline]
    pub fn span(&self) -> Span {
        self.span
    }
}

impl<N: Clone> LayerStraps<N> {
    /// The i-th power strap.
    pub fn strap(&self, i: usize) -> Strap<N> {
        let (group_idx, net_idx) = self.to_group_indices(i);
        let net = self.nets[net_idx].clone();
        let group_start = self.group_start(group_idx);
        let start = group_start + usize_as_i64(net_idx) * self.strap_pitch();
        let span = Span::with_start_and_length(start, self.width);

        Strap { net, span }
    }

    pub fn group(&self, n: usize) -> StrapGroup<N> {
        let start = n * self.nets.len();
        let end = (n + 1) * self.nets.len();
        let straps = (start..end).map(|i| self.strap(i)).collect();
        StrapGroup { straps }
    }

    /// Iterate over all straps that intersect the interval `[0, x]`.
    pub fn straps_until(&self, x: i64) -> impl Iterator<Item = Strap<N>> + '_ {
        (0..)
            .map(|i| self.strap(i))
            .take_while(move |s| s.span.start() < x)
    }

    /// Draws all power straps intersecting the interval `[0, x]`, with the given span.
    ///
    /// The span is in the direction parallel to the power straps.
    pub fn draw_until(&self, x: i64, span: Span) -> ElementGroup {
        let mut group = ElementGroup::new();
        for strap in self.straps_until(x) {
            let rect = Rect::span_builder()
                .with(self.dir, span)
                .with(!self.dir, strap.span)
                .build();
            let spec = LayerSpec::drawing(self.layer);
            group.add(Element::new(spec, rect));
        }
        group
    }
}

impl<N> LayerStraps<N> {
    /// Returns the group and net indices for a given strap index.
    fn to_group_indices(&self, n: usize) -> (usize, usize) {
        (n / self.nets.len(), n % self.nets.len())
    }

    /// The starting coordinate of the n-th group.
    fn group_start(&self, n: usize) -> i64 {
        let n = usize_as_i64(n);
        self.offset + n * self.group_pitch
    }

    /// The line plus space of straps within a single net group.
    pub fn strap_pitch(&self) -> i64 {
        self.width + self.spacing
    }

    #[inline]
    pub fn layer(&self) -> LayerKey {
        self.layer
    }

    #[inline]
    pub fn dir(&self) -> Dir {
        self.dir
    }

    #[inline]
    pub fn nets(&self) -> &[N] {
        &self.nets
    }

    /// The width of each strap.
    #[inline]
    pub fn width(&self) -> i64 {
        self.width
    }

    /// The spacing between adjacent straps within a group.
    #[inline]
    pub fn spacing(&self) -> i64 {
        self.spacing
    }

    /// The distance between the start of one group and the start of the next group.
    #[inline]
    pub fn group_pitch(&self) -> i64 {
        self.group_pitch
    }
    /// The offset of the power straps from the lower left corner of the macro.
    ///
    /// The first strap of the first group starts at this offset.
    #[inline]
    pub fn offset(&self) -> i64 {
        self.offset
    }
}

impl<N> IntoIterator for StrapGroup<N> {
    type Item = Strap<N>;
    type IntoIter = std::vec::IntoIter<Strap<N>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.straps.into_iter()
    }
}

impl<N: FromStr> StrapConfig<N>
where
    N::Err: Error + Send + Sync + 'static,
{
    /// Generate power strap configuration from raw Hammer strap configuration.
    pub fn from_hammer_straps(
        straps: hammer::HammerPowerStraps,
        macro_name: &str,
        ctx: &SubstrateCtx,
    ) -> Result<Self> {
        let straps = straps.get_macro(macro_name)?;

        let n = straps.len();
        // Must specify 1 or 2 layers.
        // If specifying 1 layer, it must be the top layer.
        // If specifying 2 layers, they must be the top layer
        // and the layer immediately above the top layer.
        // These 2 layers may be given in any order.
        if n != 1 && n != 2 {
            return Err(SubstrateError::new(
                PowerStrapError::HammerIncorrectLayerCount(straps.len()),
            ));
        }

        let l1 = LayerStraps::<N>::from_hammer_straps(&straps[0], ctx)?;
        let l2 = straps
            .get(1)
            .map(|info| LayerStraps::<N>::from_hammer_straps(info, ctx));

        if let Some(l2) = l2 {
            let l2 = l2?;

            // Figure out which layer is top, and which layer is the one above top.
            // Hammer should export layers in increasing order, but we double check
            // just to be sure.
            match l1.index.cmp(&l2.index) {
                Ordering::Less => Ok(Self {
                    top: l1,
                    above_top: Some(l2),
                }),
                Ordering::Equal => Err(SubstrateError::new(PowerStrapError::DuplicateMetalLayers(
                    l1.index,
                ))),
                Ordering::Greater => Ok(Self {
                    top: l2,
                    above_top: Some(l1),
                }),
            }
        } else {
            Ok(Self {
                top: l1,
                above_top: None,
            })
        }
    }

    /// The straps on the top layer of this macro.
    pub fn top(&self) -> &LayerStraps<N> {
        &self.top
    }

    /// Returns whether or not power straps can be drawn on the layer
    /// immediately above this macro's top layer.
    #[inline]
    pub fn above_top_exists(&self) -> bool {
        self.above_top.is_some()
    }

    /// Read Hammer power strap configuration from the given JSON string.
    pub fn from_hammer_json(json: &str, macro_name: &str, ctx: &SubstrateCtx) -> Result<Self> {
        let straps = hammer::HammerPowerStraps::from_json(json)?;
        Self::from_hammer_straps(straps, macro_name, ctx)
    }

    /// Read Hammer power strap configuration from the given JSON file.
    pub fn from_hammer_json_file(
        path: impl AsRef<Path>,
        macro_name: &str,
        ctx: &SubstrateCtx,
    ) -> Result<Self> {
        let json = crate::io::read_to_string(path)?;
        Self::from_hammer_json(&json, macro_name, ctx)
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum PowerStrapError {
    #[error("no such Hammer macro: {0}")]
    HammerMacroNotFound(String),

    #[error("expected 1 or 2 layers in Hammer power strap configuration, but found {0}")]
    HammerIncorrectLayerCount(usize),

    #[error("error parsing net names: {0}")]
    InvalidNet(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("layer {0} is not marked as a metal layer in the Substrate PDK")]
    NotMetalLayer(String),

    #[error("error parsing direction: {0}")]
    InvalidDirection(#[from] DirParseError),

    #[error("found the same metal index `{0}` multiple times in power strap configuration")]
    DuplicateMetalLayers(usize),
}
