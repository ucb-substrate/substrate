use codegen::Interface;
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::digital::context::DigitalCtx;
use substrate::digital::types::UInt;
use substrate::digital::{DigitalComponent, Interface};
use substrate::error::Result;

mod common;

pub struct Alu;

impl Component for Alu {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("alu")
    }
}

#[derive(Interface)]
pub struct AluIntf {
    #[input]
    op: UInt,
    #[input]
    a: UInt,
    #[input]
    b: UInt,
    #[output]
    out: UInt,
}

impl DigitalComponent for Alu {
    type Interface = AluIntf;

    fn digital(
        &self,
        _ctx: &mut DigitalCtx,
        input: <Self::Interface as Interface>::Input,
    ) -> substrate::digital::Result<<Self::Interface as Interface>::Output> {
        Ok(AluIntfOutputs::new(input.a + input.b))
    }

    fn interface(&self) -> Self::Interface {
        AluIntf {
            op: UInt::new(4),
            a: UInt::new(32),
            b: UInt::new(32),
            out: UInt::new(32),
        }
    }
}
