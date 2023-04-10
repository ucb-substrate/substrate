//! A primitive four-port MOSFET layout `Component`.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::layout::context::LayoutCtx;
use crate::pdk::mos::LayoutMosParams;

/// A primitive MOSFET parametrized by [`LayoutMosParams`].
pub struct LayoutMos(LayoutMosParams);

impl Component for LayoutMos {
    type Params = LayoutMosParams;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(params.to_owned()))
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("mos")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> crate::error::Result<()> {
        ctx.pdk().mos_layout(ctx, &self.0)
    }
}
