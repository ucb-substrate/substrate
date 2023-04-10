use common::{out_path, setup_ctx};
use itertools::Itertools;
use subgeom::bbox::BoundBox;
use subgeom::orientation::Named;
use subgeom::transform::{Transform, Transformation, Translate, TranslateOwned};
use subgeom::{Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::layout::group::Group;
use substrate::layout::layers::selector::Selector;

mod common;

pub struct SimpleRectangle;

impl Component for SimpleRectangle {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("simple_rectangle")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let r = Rect::new(Point::new(100, 100), Point::new(200, 400));

        let l = ctx.layers();
        let m1 = l.get(Selector::Metal(1))?;
        ctx.draw_rect(m1, r);

        Ok(())
    }
}

pub struct GroupTransformations;

impl Component for GroupTransformations {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("group_transformations")
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let r = Rect::new(Point::new(100, 100), Point::new(200, 200));
        let mut inst = ctx.instantiate::<SimpleRectangle>(&NoParams)?;
        inst.set_loc((-40, 60));

        let l = ctx.layers();
        let m1 = l.get(Selector::Metal(1))?;
        let mut group = Group::new();
        group.add_rect(m1, r);
        group.add_instance(inst);

        group.set_loc(Point::new(80, -20));
        group.orientation_mut().r90();

        let elts = group.elements().collect_vec();
        assert_eq!(elts.len(), 1);
        let r = elts[0].inner.as_rect().unwrap();
        assert_eq!(r, Rect::new(Point::new(-120, 80), Point::new(-20, 180)));

        let insts = group.instances().collect_vec();
        assert_eq!(insts.len(), 1);

        group.reflect_vert_anchored();
        group.reflect_horiz_anchored();

        let bbox = group.bbox();
        let pt = Point::new(40, -220);
        group.translate(pt);
        let r = bbox.into_rect().translate_owned(pt);
        assert_eq!(group.bbox(), r.bbox());

        // Make the group immutable.
        let group = group;
        ctx.draw(group)?;

        Ok(())
    }
}

#[test]
fn test_group_transformations() {
    let ctx = setup_ctx();
    ctx.write_layout::<GroupTransformations>(
        &NoParams,
        out_path("test_group_transformations", "layout.gds"),
    )
    .expect("failed to write layout");

    let inst = ctx
        .instantiate_layout::<GroupTransformations>(&NoParams)
        .expect("failed to instantiate layout");
    let r = inst.brect();

    assert_eq!(inst.transformation(), Transformation::identity());

    for orientation in Named::all_rectangular() {
        let tf = Transformation::with_loc_and_orientation(Point::new(40, 80), orientation);
        let transformed = inst.transform(tf);
        assert_eq!(transformed.transformation(), tf);
        assert_eq!(
            inst.transform(tf).brect(),
            r.transform(tf),
            "transformation failed for orientation {:?}",
            orientation
        );
    }
}
