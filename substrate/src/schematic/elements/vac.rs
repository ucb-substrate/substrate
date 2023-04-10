//! A primitive AC supply `Component`.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::units::SiValue;

/// A primitive AC supply parametrized by AC amplitude.
pub struct Vac(SiValue);

impl Component for Vac {
    type Params = SiValue;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(*params))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("vac_{}", self.0)
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        ctx.set_spice(format!("V1 p n ac {}", self.0));
        Ok(())
    }
}
