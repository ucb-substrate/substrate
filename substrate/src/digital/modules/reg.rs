use codegen::Interface;

use crate::component::Component;
use crate::data::SubstrateCtx;
use crate::digital::context::DigitalCtx;
use crate::digital::types::{Bool, Clock, Hardware};
use crate::digital::wire::Literal;
use crate::digital::{DigitalComponent, Interface};
use crate::error::Result;

pub struct RegReset<T: Hardware> {
    reset_value: Literal<T>,
}

impl<T> Component for RegReset<T>
where
    T: Hardware,
{
    type Params = Literal<T>;
    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> Result<Self> {
        Ok(Self {
            reset_value: params.clone(),
        })
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("reg_reset")
    }
}

#[derive(Interface)]
pub struct RegResetIntf<T: Hardware> {
    #[input]
    d: T,
    #[output]
    q: T,
    #[input]
    clk: Clock,
    #[input]
    rst: Bool,
}

impl<T> DigitalComponent for RegReset<T>
where
    T: Hardware,
{
    type Interface = RegResetIntf<T>;

    fn digital(
        &self,
        _ctx: &mut DigitalCtx,
        input: <Self::Interface as Interface>::Input,
    ) -> crate::digital::Result<<Self::Interface as Interface>::Output> {
        let q = input.d.reg(input.clk);
        Ok(RegResetIntfOutputs::new(q))
    }

    fn interface(&self) -> Self::Interface {
        RegResetIntf {
            d: self.reset_value.t().clone(),
            q: self.reset_value.t().clone(),
            clk: Clock,
            rst: Bool,
        }
    }
}
