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

        let mut r1 = ctx.instantiate::<Resistor>(&SiValue::new(2, SiPrefix::Kilo))?;
        r1.connect_all([("p", &vdd), ("n", &out)]);
        r1.set_name("R1");
        ctx.add_instance(r1);

        let mut r2 = ctx.instantiate::<Resistor>(&SiValue::new(1, SiPrefix::Kilo))?;
        r2.connect_all([("p", &out), ("n", &vss)]);
        r2.set_name("R2");
        ctx.add_instance(r2);
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
