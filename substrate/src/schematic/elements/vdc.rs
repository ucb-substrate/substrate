//! A primitive DC supply `Component`.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::units::SiValue;

/// A primitive DC supply parametrized by DC voltage.
pub struct Vdc(SiValue);

impl Component for Vdc {
    type Params = SiValue;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(*params))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("vdc_{}", self.0)
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        ctx.set_spice(format!("V1 p n {}", self.0));
        Ok(())
    }
}
