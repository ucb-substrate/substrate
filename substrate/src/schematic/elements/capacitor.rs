//! A primitive capacitor `Component`.

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::schematic::context::SchematicCtx;
use crate::units::SiValue;

/// A primitive capacitor parametrized by capacitance.
pub struct Capacitor(SiValue);

impl Component for Capacitor {
    type Params = SiValue;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(Self(*params))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("capacitor_{}", self.0)
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        ctx.set_spice(format!("C1 p n {}", self.0));
        Ok(())
    }
}
