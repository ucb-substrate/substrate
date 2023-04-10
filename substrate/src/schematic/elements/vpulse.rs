//! A pulse voltage source.

use serde::{Deserialize, Serialize};

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::units::SiValue;

/// A pulse voltage source.
#[derive(Copy, Clone, Eq, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vpulse {
    /// Initial value (volts).
    pub v1: SiValue,
    /// Pulsed value (volts).
    pub v2: SiValue,
    /// Delay time (seconds).
    pub td: SiValue,
    /// Rise time (seconds).
    pub tr: SiValue,
    /// Fall time (seconds).
    pub tf: SiValue,
    /// Pulse width (seconds).
    pub pw: SiValue,
    /// Period (seconds).
    pub period: SiValue,
}

impl Component for Vpulse {
    type Params = Vpulse;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        Ok(*params)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("vpulse")
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        ctx.set_spice(format!(
            "V1 p n PULSE({} {} {} {} {} {} {})",
            self.v1, self.v2, self.td, self.tr, self.tf, self.pw, self.period
        ));
        Ok(())
    }
}
