//! Utilities and types for drawing vias.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use subgeom::{Dir, Rect};

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::layout::context::LayoutCtx;
use crate::layout::layers::LayerKey;

pub mod generators;

/// Via drawing parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViaParams {
    /// Identifies the type of via being drawn.
    ///
    /// This can specify, for example, that the desired via
    /// connects metal 2 to metal 1. See the [`ViaSelector`]
    /// documentation for more details.
    pub selector: ViaSelector,
    /// Constrains how much the via can expand beyond existing geometry.
    ///
    /// This is often needed when drawing minimum-size metal wires,
    /// as a via may require extra metal padding to meet DRC rules.
    ///
    /// This parameter determines how much extension beyond
    /// existing geometry is permissible. See the [`ViaExpansion`]
    /// documentation for more details.
    pub expand: ViaExpansion,

    /// Direction of the longer extension on the top layer.
    pub top_extension: Option<Dir>,

    /// Direction of the longer extension on the bottom layer.
    pub bot_extension: Option<Dir>,

    /// The geometry on the upper layer.
    pub top: Rect,

    /// The geometry on the lower layer.
    pub bot: Rect,

    /// Custom options passed directly to the PDK's via generator.
    pub opts: HashMap<String, String>,
}

pub enum Size {
    Geometry { top: Rect, bot: Rect },
    Count { nx: usize, ny: usize },
}

/// Identifies a type of via.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub enum ViaSelector {
    /// Identify a via by name.
    ///
    /// It is up to the PDK to provide via names.
    /// For process portability, use [`ViaSelector::Layers`] where possible.
    Name(ArcStr),

    /// Identify a via by the layers it connects.
    Layers { bot: LayerKey, top: LayerKey },
}

/// Constrains how much the via can expand beyond existing geometry.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Serialize, Deserialize)]
pub enum ViaExpansion {
    /// The via may not expand beyond the layer bounding boxes.
    ///
    /// Errors if the bounding boxes are too small to fully contain a via.
    None,

    /// Allow the via to expand beyond the layer bounding boxes,
    /// but only to allow one via to be created.
    ///
    /// When the bounding boxes are too small to contain one via,
    /// will typically generate a `1x1` via.
    #[default]
    Minimum,

    /// Alllow a via array to repeat in the longer overlap direction.
    ///
    /// When the bounding boxes are too small to contain one via,
    /// will typically generate `1xN` or `Nx1` arrays.
    ///
    /// Typically most useful when the overlap region is long and skinny.
    LongerDirection,
}

/// Builder for [`ViaParams`].
#[derive(Clone, Eq, PartialEq, Default)]
pub struct ViaParamsBuilder {
    selector: Option<ViaSelector>,
    expand: ViaExpansion,
    /// Direction of the longer extension on the top layer.
    pub top_extension: Option<Dir>,
    /// Direction of the longer extension on the bottom layer.
    pub bot_extension: Option<Dir>,
    /// Bottom and top rectangles, respectively.
    geometry: Option<(Rect, Rect)>,
    opts: HashMap<String, String>,
}

impl ViaParams {
    #[inline]
    pub fn builder() -> ViaParamsBuilder {
        ViaParamsBuilder::default()
    }

    /// Tells the via generator about the geometry on the top and bottom layers.
    pub fn set_geometry(&mut self, bot: impl Into<Rect>, top: impl Into<Rect>) -> &mut Self {
        self.bot = bot.into();
        self.top = top.into();
        self
    }
}

impl ViaParamsBuilder {
    /// Selects the via with the given name.
    ///
    /// [Named vias](ViaSelector::Name) must be supported by the PDK.
    pub fn name(&mut self, name: impl Into<ArcStr>) -> &mut Self {
        self.selector = Some(ViaSelector::Name(name.into()));
        self
    }

    /// Selects a via that connects the given top and bottom layers.
    pub fn layers(&mut self, bot: impl Into<LayerKey>, top: impl Into<LayerKey>) -> &mut Self {
        self.selector = Some(ViaSelector::Layers {
            bot: bot.into(),
            top: top.into(),
        });
        self
    }

    /// Tells the via generator about the geometry on the top and bottom layers.
    pub fn geometry(&mut self, bot: impl Into<Rect>, top: impl Into<Rect>) -> &mut Self {
        self.geometry = Some((bot.into(), top.into()));
        self
    }

    /// Sets a custom option that will be passed directly to the via generator.
    pub fn option(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.opts.insert(key.into(), value.into());
        self
    }

    /// Sets the (expansion mode)[ViaExpansion].
    pub fn expand(&mut self, expand: ViaExpansion) -> &mut Self {
        self.expand = expand;
        self
    }

    /// Sets the extension direction for the top layer.
    pub fn top_extension(&mut self, dir: Dir) -> &mut Self {
        self.top_extension = Some(dir);
        self
    }

    /// Sets the extension direction for the bottom layer.
    pub fn bot_extension(&mut self, dir: Dir) -> &mut Self {
        self.bot_extension = Some(dir);
        self
    }

    /// Consumes the builder, returning a [`ViaParams`] struct.
    pub fn build(&mut self) -> ViaParams {
        let (bot, top) = self.geometry.unwrap();
        ViaParams {
            selector: self.selector.clone().unwrap(),
            expand: self.expand,
            top_extension: self.top_extension,
            bot_extension: self.bot_extension,
            top,
            bot,
            opts: self.opts.clone(),
        }
    }
}

pub struct Via(ViaParams);

impl Component for Via {
    type Params = ViaParams;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Via(params.to_owned()))
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("via")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> crate::error::Result<()> {
        ctx.pdk().via_layout(ctx, &self.0)
    }
}
