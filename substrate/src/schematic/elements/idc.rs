//! A primitive DC current supply.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::units::SiValue;

/// A primitive DC current supply parametrized by DC current.
pub struct Idc(SiValue);

impl Component for Idc {
    type Params = SiValue;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(*params))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("idc_{}", self.0)
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        ctx.set_spice(format!("I1 p n {}", self.0));
        Ok(())
    }
}
