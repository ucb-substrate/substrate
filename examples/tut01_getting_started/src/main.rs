use arcstr::ArcStr;
use empty_pdk::EmptyPdk;
use substrate::component::{Component, NoParams};
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::resistor::Resistor;
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::units::{SiPrefix, SiValue};

struct VDivider;

impl Component for VDivider {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        "vdivider".into()
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let out = ctx.port("out", Direction::Output);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);

        ctx.instantiate::<Resistor>(&SiValue::new(2, SiPrefix::Kilo))?
            .with_connections([("p", vdd), ("n", out)])
            .named("R1")
            .add_to(ctx);

        ctx.instantiate::<Resistor>(&SiValue::new(1, SiPrefix::Kilo))?
            .with_connections([("p", out), ("n", vss)])
            .named("R2")
            .add_to(ctx);

        Ok(())
    }
}

pub fn setup_ctx() -> SubstrateCtx {
    let pdk = EmptyPdk::new();
    let cfg = SubstrateConfig::builder()
        .netlister(SpiceNetlister::new())
        .pdk(pdk)
        .build();
    SubstrateCtx::from_config(cfg).unwrap()
}

fn main() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<VDivider>(&NoParams, "build/vdivider.spice")
        .expect("failed to write schematic");
}

#[cfg(test)]
#[test]
fn test_tut01_getting_started() {
    main();
}
