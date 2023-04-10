//! A piece-wise linear voltage source.

use std::fmt::Write;
use std::sync::Arc;

use crate::component::Component;
use crate::deps::arcstr::ArcStr;
use crate::schematic::circuit::Direction;
use crate::verification::simulation::waveform::{TimeWaveform, Waveform};

/// A piece-wise linear voltage source.
pub struct Vpwl(Arc<Waveform>);

impl Component for Vpwl {
    type Params = Arc<Waveform>;

    fn new(params: &Self::Params, _ctx: &crate::data::SubstrateCtx) -> crate::error::Result<Self> {
        assert!(params.len() > 0);
        Ok(Self(params.clone()))
    }

    fn name(&self) -> ArcStr {
        arcstr::format!("vpwl")
    }

    fn schematic(
        &self,
        ctx: &mut crate::schematic::context::SchematicCtx,
    ) -> crate::error::Result<()> {
        let _p = ctx.port("p", Direction::InOut);
        let _n = ctx.port("n", Direction::InOut);

        let mut spice = String::from("V1 p n PWL(");
        for pt in self.0.values() {
            write!(&mut spice, " {} {}", pt.t(), pt.x()).unwrap();
        }
        write!(&mut spice, " )").unwrap();
        ctx.set_spice(spice);
        Ok(())
    }
}
