use arcstr::ArcStr;
use subgeom::{Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::layout::context::LayoutCtx;
use substrate::layout::layers::selector::Selector;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::resistor::Resistor;
use substrate::units::{SiPrefix, SiValue};

pub mod array;
pub mod tb;

pub struct VDivider;

impl Component for VDivider {
    type Params = NoParams;

    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let out = ctx.port("out", Direction::Output);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);

        ctx.instantiate::<Resistor>(&SiValue::new(2, SiPrefix::Kilo))?
            .with_connections([("p", &vdd), ("n", &out)])
            .named("R1")
            .add_to(ctx);

        ctx.instantiate::<Resistor>(&SiValue::new(1, SiPrefix::Kilo))?
            .with_connections([("p", &out), ("n", &vss)])
            .named("R2")
            .add_to(ctx);
        Ok(())
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Routing(1)).unwrap();
        ctx.draw_rect(layer, Rect::new(Point::new(0, 0), Point::new(250, 500)));
        Ok(())
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("vdivider")
    }
}
