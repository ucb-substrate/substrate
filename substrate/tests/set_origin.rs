use common::{out_path, setup_ctx};
use subgeom::{Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::layout::layers::selector::Selector;

mod common;

pub struct SetOriginBasic;

impl Component for SetOriginBasic {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("set_origin_basic")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let r = Rect::new(Point::new(-200, -200), Point::new(200, 200));

        let l = ctx.layers();
        let m1 = l.get(Selector::Metal(1))?;

        ctx.draw_rect(m1, r);

        ctx.set_origin(Point::new(-200, 200));

        let bounds = ctx.bbox().into_rect();
        assert_eq!(
            bounds,
            Rect::new(Point::new(0, -400), Point::new(400, 0)),
            "cell bbox after updating origin did not match expected bbox"
        );
        Ok(())
    }
}

#[test]
fn set_origin_basic() {
    let ctx = setup_ctx();
    ctx.write_layout::<SetOriginBasic>(&NoParams, out_path("set_origin_basic", "layout.gds"))
        .expect("failed to write layout");
}
