use arcstr::ArcStr;
use subgeom::{Point, Rect};
use substrate::component::Component;
use substrate::data::SubstrateCtx;

mod common;
use common::{out_path, setup_ctx};
use substrate::layout::layers::selector::Selector;

pub struct SimpleComponent(usize);

impl Component for SimpleComponent {
    type Params = usize;

    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(*params))
    }

    fn name(&self) -> ArcStr {
        // In particular, this does not depend on the parameters.
        arcstr::format!("simple_component")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let layer = layers.get(Selector::Metal(1))?;

        if self.0 == 0 {
            ctx.draw_rect(layer, Rect::new(Point::zero(), Point::new(600, 400)));
            return Ok(());
        }

        for _ in 0..self.0 {
            let inst = ctx.instantiate::<SimpleComponent>(&(self.0 - 1))?;
            ctx.draw(inst)?;
        }

        Ok(())
    }
}

#[test]
fn test_gds_cell_renaming() {
    let gds_path = out_path("test_gds_cell_renaming", "layout.gds");
    let ctx = setup_ctx();
    ctx.write_layout::<SimpleComponent>(&4, &gds_path)
        .expect("failed to generate layout");

    let ctx_new = setup_ctx();
    let cell_map = ctx_new
        .from_gds(gds_path)
        .expect("failed to import GDS file");
    let component = cell_map.get("simple_component").unwrap();
    assert_eq!(component.insts().count(), 4);
}
