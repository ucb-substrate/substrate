use arcstr::ArcStr;
use subgeom::{Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaParams};
use substrate::layout::layers::selector::Selector;

mod common;
use common::{out_path, setup_ctx};

pub struct MyRouting;

impl Component for MyRouting {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> ArcStr {
        arcstr::literal!("my_routing")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let m0 = ctx.layers().get(Selector::Metal(0))?;
        let m1 = ctx.layers().get(Selector::Metal(1))?;

        let r0 = Rect::new(Point::zero(), Point::new(2_000, 200));
        let r1 = Rect::new(Point::new(800, -1_000), Point::new(1_000, 1_000));
        ctx.draw_rect(m0, r0);
        ctx.draw_rect(m1, r1);

        let via =
            ctx.instantiate::<Via>(&ViaParams::builder().layers(m0, m1).geometry(r0, r1).build())?;
        ctx.draw(via)?;

        let r0 = Rect::new(Point::new(0, 2_000), Point::new(2_000, 3_000));
        let r1 = Rect::new(Point::new(400, 1_500), Point::new(1_600, 4_000));
        ctx.draw_rect(m0, r0);
        ctx.draw_rect(m1, r1);

        let via =
            ctx.instantiate::<Via>(&ViaParams::builder().layers(m0, m1).geometry(r0, r1).build())?;
        ctx.draw(via)?;

        Ok(())
    }
}

#[test]
fn via_basic() {
    let ctx = setup_ctx();
    ctx.write_layout::<MyRouting>(&NoParams, out_path("via_basic", "layout.gds"))
        .expect("failed to write layout");
}
