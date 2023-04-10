//! A primitive AC current source `Component`.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::units::SiValue;

/// A primitive AC current source parametrized by AC amplitude.
pub struct Iac(SiValue);

impl Component for Iac {
    type Params = SiValue;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(*params))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("iac_{}", self.0)
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        ctx.set_spice(format!("I1 p n ac {}", self.0));
        Ok(())
    }
}
