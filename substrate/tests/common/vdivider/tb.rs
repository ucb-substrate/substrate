use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::deps::arcstr::ArcStr;
use substrate::error::Result;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::vdc::Vdc;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::{Analysis, OpAnalysis, Save};

use super::VDivider;

pub struct VDividerTb;

pub struct Output {
    /// The voltage divider ratio
    pub ratio: f64,
}

impl Component for VDividerTb {
    type Params = NoParams;

    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> Result<Self> {
        Ok(Self)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let vin = ctx.signal("vin");
        let vout = ctx.signal("vout");

        let mut dut = ctx.instantiate::<VDivider>(&NoParams)?;
        dut.connect_all([("out", &vout), ("vdd", &vin), ("vss", &vss)]);
        dut.set_name("DUT");
        ctx.add_instance(dut);

        let mut src = ctx.instantiate::<Vdc>(&SiValue::new(1, SiPrefix::None))?;
        src.connect_all([("p", &vin), ("n", &vss)]);
        src.set_name("src");
        ctx.add_instance(src);

        Ok(())
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("vdivider_dc_tb")
    }
}

impl Testbench for VDividerTb {
    type Output = Output;

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        ctx.add_analysis(Analysis::Op(OpAnalysis::new()))
            .save(Save::All);
        Ok(())
    }

    fn measure(
        &mut self,
        ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        let ratio = ctx.output().data[0].op().data["v(vout)"].value;
        Ok(Output { ratio })
    }
}
