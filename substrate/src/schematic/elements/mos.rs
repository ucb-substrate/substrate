//! A primitive four-port MOSFET schematic `Component`.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::pdk::mos::MosParams;
use crate::schematic::circuit::Direction;
use crate::schematic::context::SchematicCtx;

/// A primitive MOSFET parametrized by [`MosParams`].
pub struct SchematicMos(MosParams);

impl Component for SchematicMos {
    type Params = MosParams;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(params.to_owned()))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("mos_{}", self.0)
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> crate::error::Result<()> {
        let _d = ctx.port("d", Direction::InOut);
        let _g = ctx.port("g", Direction::InOut);
        let _s = ctx.port("s", Direction::InOut);
        let _b = ctx.port("b", Direction::InOut);

        let pdk = ctx.pdk();
        pdk.mos_schematic(ctx, &self.0)?;

        Ok(())
    }
}
