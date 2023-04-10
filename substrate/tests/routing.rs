use common::{out_path, setup_ctx};
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::{Dir, Point, Rect, Side, Sign, Span};
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::layout::layers::selector::Selector;
use substrate::layout::routing::auto::grid::{
    ExpandToGridStrategy, JogToGrid, OffGridBusTranslation,
};
use substrate::layout::routing::auto::straps::{RoutedStraps, Target};
use substrate::layout::routing::auto::{GreedyRouter, GreedyRouterConfig, LayerConfig};
use substrate::layout::routing::tracks::UniformTracks;

mod common;

pub struct SimpleTwoLayerRouting;

impl Component for SimpleTwoLayerRouting {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("simple_two_layer_routing")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let src = Rect::new(Point::new(0, 0), Point::new(200, 200));
        let dst = Rect::new(Point::new(2_200, 2_200), Point::new(2_400, 2_400));
        let dst2 = Rect::new(Point::new(2_200, 4_200), Point::new(2_400, 4_400));

        let layers = ctx.layers();
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        ctx.draw_rect(m2, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m2, dst2);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(5_000),
            layers: vec![
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        let src = router.register_jog_to_grid(
            JogToGrid::builder()
                .layer(m2)
                .rect(src)
                .width(200)
                .first_dir(Side::Right)
                .extend_first(1)
                .build(),
        );
        let dst = router
            .register_jog_to_grid(JogToGrid::builder().layer(m2).rect(dst).width(200).build());
        let dst2 = router
            .register_jog_to_grid(JogToGrid::builder().layer(m2).rect(dst2).width(200).build());

        ctx.draw_rect(m2, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m2, dst2);

        router.route(ctx, m2, src, m2, dst)?;
        router.route(ctx, m2, src, m2, dst2)?;
        ctx.draw(router)?;

        Ok(())
    }
}

pub struct SimpleThreeLayerRouting;

impl Component for SimpleThreeLayerRouting {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("simple_three_layer_routing")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let src = Rect::new(Point::new(0, 0), Point::new(1_000, 200));
        let dst = Rect::new(Point::new(2_000, 2_000), Point::new(2_200, 2_200));
        let dst2 = Rect::new(Point::new(2_000, 4_000), Point::new(2_200, 4_200));
        let dst3 = Rect::new(Point::new(2_000, -2_000), Point::new(2_200, -2_200));

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);
        ctx.draw_rect(m2, dst3);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(5_000),
            layers: vec![
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        let src = router.expand_to_grid(src, ExpandToGridStrategy::Minimum);
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Minimum);
        let dst2 = router.expand_to_grid(dst2, ExpandToGridStrategy::Minimum);
        let dst3 = router.expand_to_grid(dst3, ExpandToGridStrategy::Minimum);

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);
        ctx.draw_rect(m2, dst3);

        router.route(ctx, m1, src, m2, dst)?;
        router.route(ctx, m1, src, m3, dst2)?;
        router.route(ctx, m1, src, m2, dst3)?;
        ctx.draw(router)?;

        Ok(())
    }
}

pub struct ThreeLayerRoutingWithBlockages;

impl Component for ThreeLayerRoutingWithBlockages {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("three_layer_routing_with_blockages")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let src = Rect::new(Point::new(0, 0), Point::new(1_000, 200));
        let dst = Rect::new(Point::new(2_000, 2_000), Point::new(2_200, 2_200));
        let dst2 = Rect::new(Point::new(2_000, 4_000), Point::new(2_200, 4_200));
        let blockage = Rect::new(Point::new(0, 2_000), Point::new(4_200, 2_200));
        let blockage2 = Rect::new(Point::new(0, 3_500), Point::new(4_200, 3_700));

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);
        ctx.draw_rect(m3, blockage);
        ctx.draw_rect(m1, blockage2);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(5000),
            layers: vec![
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        let src = router.expand_to_grid(src, ExpandToGridStrategy::Minimum);
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Minimum);
        let dst2 = router.expand_to_grid(dst2, ExpandToGridStrategy::Minimum);

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);

        router.block(m3, blockage);
        router.block(m1, blockage2);

        router.route(ctx, m1, src, m2, dst)?;
        router.route(ctx, m1, src, m3, dst2)?;
        ctx.draw(router)?;

        Ok(())
    }
}

pub struct ThreeLayerRoutingWithUnevenGrid;

impl Component for ThreeLayerRoutingWithUnevenGrid {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("three_layer_routing_with_uneven_grid")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let src = Rect::new(Point::new(0, 0), Point::new(1_000, 200));
        let dst = Rect::new(Point::new(2_000, 2_000), Point::new(2_200, 2_200));
        let dst2 = Rect::new(Point::new(2_000, 4_000), Point::new(2_200, 4_200));
        let blockage = Rect::new(Point::new(0, 2_000), Point::new(4_200, 2_200));
        let blockage2 = Rect::new(Point::new(0, 3_500), Point::new(4_200, 3_700));

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);
        ctx.draw_rect(m3, blockage);
        ctx.draw_rect(m1, blockage2);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(10_000),
            layers: vec![
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 680,
                    space: 320,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 340,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        let src = router.expand_to_grid(src, ExpandToGridStrategy::Minimum);
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Minimum);
        let dst2 = router.expand_to_grid(dst2, ExpandToGridStrategy::Minimum);

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);

        router.block(m3, blockage);
        router.block(m1, blockage2);

        router.route(ctx, m1, src, m2, dst)?;
        router.route(ctx, m1, src, m3, dst2)?;
        ctx.draw(router)?;

        Ok(())
    }
}

pub struct OffGridRouting;

impl Component for OffGridRouting {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("off_grid_routing")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        let bus1_tracks = UniformTracks::builder()
            .line(320)
            .space(160)
            .start(400)
            .sign(Sign::Pos)
            .build()
            .unwrap();

        let mut bus1_bbox = Bbox::empty();
        let bus1_span = Span::new(8_000, 9_000);
        for j in 0..6usize {
            let tr = bus1_tracks.index(j);
            let rect = Rect::from_spans(tr, bus1_span);
            ctx.draw_rect(m3, rect);
            bus1_bbox = bus1_bbox.union(rect.bbox());
        }
        let bus1_bbox = bus1_bbox.into_rect();

        let bus2_tracks = UniformTracks::builder()
            .line(320)
            .space(160)
            .start(100)
            .sign(Sign::Pos)
            .build()
            .unwrap();

        let mut bus2_bbox = Bbox::empty();
        let bus2_span = Span::new(-5_000, -4_000);
        for j in 0..6usize {
            let tr = bus2_tracks.index(j);
            let rect = Rect::from_spans(bus2_span, tr);
            ctx.draw_rect(m2, rect);
            bus2_bbox = bus2_bbox.union(rect.bbox());
        }
        let bus2_bbox = bus2_bbox.into_rect();

        let bus3_tracks = UniformTracks::builder()
            .line(320)
            .space(160)
            .start(20_000)
            .sign(Sign::Pos)
            .build()
            .unwrap();

        let mut bus3_bbox = Bbox::empty();
        let bus3_span = Span::new(10_000, 11_000);
        for j in 0..6usize {
            let tr = bus3_tracks.index(j);
            let rect = Rect::from_spans(bus3_span, tr);
            ctx.draw_rect(m2, rect);
            bus3_bbox = bus3_bbox.union(rect.bbox());
        }
        let bus3_bbox = bus3_bbox.into_rect();

        let bus4_tracks = UniformTracks::builder()
            .line(320)
            .space(160)
            .start(20_000)
            .sign(Sign::Pos)
            .build()
            .unwrap();

        let mut bus4_bbox = Bbox::empty();
        let bus4_span = Span::new(8_000, 9_000);
        for j in 0..6usize {
            let tr = bus4_tracks.index(j);
            let rect = Rect::from_spans(tr, bus4_span);
            ctx.draw_rect(m1, rect);
            bus4_bbox = bus4_bbox.union(rect.bbox());
        }
        let bus4_bbox = bus4_bbox.into_rect();

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(10_000),
            layers: vec![
                LayerConfig {
                    line: 170,
                    space: 170,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 320,
                    space: 360,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });
        let bus1b = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .line_and_space(bus1_tracks.line, bus1_tracks.space)
                .layer(m3)
                .output(bus1_bbox.edge(Side::Bot))
                .start(bus1_bbox.left())
                .n(6)
                .build(),
        );

        let bus1t = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .line_and_space(bus1_tracks.line, bus1_tracks.space)
                .layer(m3)
                .output(bus1_bbox.edge(Side::Top))
                .start(bus1_bbox.left())
                .n(6)
                .build(),
        );

        let bus2 = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .line_and_space(bus2_tracks.line, bus2_tracks.space)
                .layer(m2)
                .output(bus2_bbox.edge(Side::Right))
                .start(bus2_bbox.bottom())
                .n(6)
                .build(),
        );

        let bus3l = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .line_and_space(bus3_tracks.line, bus3_tracks.space)
                .layer(m2)
                .output(bus3_bbox.edge(Side::Left))
                .start(bus3_bbox.bottom())
                .n(6)
                .shift(2)
                .build(),
        );

        let bus3r = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .line_and_space(bus3_tracks.line, bus3_tracks.space)
                .layer(m2)
                .output(bus3_bbox.edge(Side::Right))
                .n(6)
                .start(bus3_bbox.bottom())
                .build(),
        );

        let bus4 = router.register_off_grid_bus_translation(
            OffGridBusTranslation::builder()
                .line_and_space(bus4_tracks.line, bus4_tracks.space)
                .layer(m1)
                .output(bus4_bbox.edge(Side::Top))
                .start(bus4_bbox.left())
                .n(6)
                .output_pitch(2)
                .build(),
        );

        for (port1, port2) in bus1b.ports().zip(bus2.ports()) {
            router.route(ctx, m3, port1, m2, port2)?;
        }

        for (port1, port2) in bus1t.ports().zip(bus3l.ports()) {
            router.route(ctx, m3, port1, m2, port2)?;
        }

        for (port1, port2) in bus4.ports().zip(bus3r.ports()) {
            router.route(ctx, m1, port1, m2, port2)?;
        }

        ctx.draw(router)?;

        Ok(())
    }
}

pub struct ThreeLayerRoutingWithStrapFill;

impl Component for ThreeLayerRoutingWithStrapFill {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("three_layer_routing_with_strap_fill")
    }
    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let src = Rect::new(Point::new(0, 0), Point::new(1_000, 200));
        let dst = Rect::new(Point::new(2_000, 2_000), Point::new(2_200, 2_200));
        let dst2 = Rect::new(Point::new(2_000, 4_000), Point::new(2_200, 4_200));
        let blockage = Rect::new(Point::new(0, 2_000), Point::new(4_200, 2_200));
        let blockage2 = Rect::new(Point::new(0, 3_500), Point::new(4_200, 3_700));

        let layers = ctx.layers();
        let m1 = layers.get(Selector::Metal(1))?;
        let m2 = layers.get(Selector::Metal(2))?;
        let m3 = layers.get(Selector::Metal(3))?;

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);
        ctx.draw_rect(m3, blockage);
        ctx.draw_rect(m1, blockage2);

        let mut router = GreedyRouter::with_config(GreedyRouterConfig {
            area: ctx.brect().expand(2000),
            layers: vec![
                LayerConfig {
                    line: 260,
                    space: 160,
                    dir: Dir::Vert,
                    layer: m1,
                },
                LayerConfig {
                    line: 260,
                    space: 160,
                    dir: Dir::Horiz,
                    layer: m2,
                },
                LayerConfig {
                    line: 440,
                    space: 400,
                    dir: Dir::Vert,
                    layer: m3,
                },
            ],
        });

        let src = router.expand_to_grid(src, ExpandToGridStrategy::Minimum);
        let dst = router.expand_to_grid(dst, ExpandToGridStrategy::Minimum);
        let dst2 = router.expand_to_grid(dst2, ExpandToGridStrategy::Minimum);

        ctx.draw_rect(m1, src);
        ctx.draw_rect(m2, dst);
        ctx.draw_rect(m3, dst2);

        router.block(m3, blockage);
        router.block(m1, blockage2);

        router.route(ctx, m1, src, m2, dst)?;
        router.route(ctx, m1, src, m3, dst2)?;

        let target = Rect::new(Point::new(-1_000, -100), Point::new(-400, 2_000));
        ctx.draw_rect(m1, target);

        let mut straps = RoutedStraps::new();
        straps.set_strap_layers([m2, m3]);
        straps.add_target(
            m1,
            Target::new(substrate::layout::straps::SingleSupplyNet::Vdd, target),
        );
        straps.fill(&router, ctx)?;

        ctx.draw(router)?;

        Ok(())
    }
}

#[test]
fn test_greedy_two_layer_router_basic() {
    let ctx = setup_ctx();
    ctx.write_layout::<SimpleTwoLayerRouting>(
        &NoParams,
        out_path("test_greedy_two_layer_router_basic", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_greedy_three_layer_router_basic() {
    let ctx = setup_ctx();
    ctx.write_layout::<SimpleThreeLayerRouting>(
        &NoParams,
        out_path("test_greedy_three_layer_router_basic", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_greedy_three_layer_router_with_blockages() {
    let ctx = setup_ctx();
    ctx.write_layout::<ThreeLayerRoutingWithBlockages>(
        &NoParams,
        out_path(
            "test_greedy_three_layer_router_with_blockages",
            "layout.gds",
        ),
    )
    .expect("failed to write layout");
}

#[test]
fn test_greedy_three_layer_router_with_uneven_grid() {
    let ctx = setup_ctx();
    ctx.write_layout::<ThreeLayerRoutingWithUnevenGrid>(
        &NoParams,
        out_path(
            "test_greedy_three_layer_router_with_uneven_grid",
            "layout.gds",
        ),
    )
    .expect("failed to write layout");
}

#[test]
fn test_off_grid_routing() {
    let ctx = setup_ctx();
    ctx.write_layout::<OffGridRouting>(&NoParams, out_path("test_off_grid_routing", "layout.gds"))
        .expect("failed to write layout");
}

#[test]
fn test_greedy_three_layer_router_with_strap_fill() {
    let ctx = setup_ctx();
    ctx.write_layout::<ThreeLayerRoutingWithStrapFill>(
        &NoParams,
        out_path(
            "test_greedy_three_layer_router_with_strap_fill",
            "layout.gds",
        ),
    )
    .expect("failed to write layout");
}
