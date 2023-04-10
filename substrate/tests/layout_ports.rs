use arcstr::ArcStr;
use subgeom::{Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::layout::cell::PortId;
use substrate::layout::context::LayoutCtx;
use substrate::layout::layers::selector::Selector;

mod common;
use common::{out_path, setup_ctx};
use substrate::layout::cell::CellPort;

struct FivePort;
struct InvalidFivePort;

impl Component for FivePort {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("five_port")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(2))?;
        let rect = Rect::new(Point::zero(), Point::new(1000, 200));
        ctx.draw_rect(layer, rect);

        for i in 0..5 {
            let rect = Rect::new(Point::new(200 * i, 0), Point::new(200 * (i + 1), 200));
            ctx.add_port(CellPort::with_shape(
                PortId::new("data", i as usize),
                layer,
                rect,
            ))
            .unwrap();
        }

        Ok(())
    }
}

impl Component for InvalidFivePort {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("invalid_five_port")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(2))?;
        let rect = Rect::new(Point::zero(), Point::new(1000, 200));
        ctx.draw_rect(layer, rect);

        for i in 0..5 {
            let rect = Rect::new(Point::new(200 * i, 0), Point::new(200 * (i + 1), 200));
            ctx.add_port(CellPort::with_shape(
                PortId::new("data", 2 * i as usize),
                layer,
                rect,
            ))
            .unwrap();
        }

        Ok(())
    }
}

#[test]
fn test_layout_bus_port() {
    let ctx = setup_ctx();
    ctx.write_layout::<FivePort>(&NoParams, out_path("test_layout_bus_port", "layout.gds"))
        .expect("failed to write layout");
}

#[test]
fn test_layout_bus_port_invalid() {
    let ctx = setup_ctx();
    ctx.write_layout::<InvalidFivePort>(
        &NoParams,
        out_path("test_layout_bus_port_invalid", "layout.gds"),
    )
    .expect_err("expected failed layout generation");
}
