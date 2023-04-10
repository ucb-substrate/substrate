use common::{out_path, setup_ctx};
use subgeom::{Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::layout::layers::selector::Selector;

mod common;

pub struct TrimBasic;

impl Component for TrimBasic {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("trim_basic")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let r = Rect::new(Point::new(-200, -200), Point::new(200, 200));

        let l = ctx.layers();
        let m1 = l.get(Selector::Metal(1))?;

        ctx.draw_rect(m1, r);

        let trim = Rect::new(Point::zero(), Point::new(400, 400));
        ctx.trim(&trim);

        let bounds = ctx.bbox().into_rect();
        assert_eq!(
            bounds,
            Rect::new(Point::zero(), Point::new(200, 200)),
            "cell bbox after trimming did not match expected bbox"
        );
        Ok(())
    }
}

#[test]
fn trim_basic() {
    let ctx = setup_ctx();
    ctx.write_layout::<TrimBasic>(&NoParams, out_path("trim_basic", "layout.gds"))
        .expect("failed to write layout");
}
