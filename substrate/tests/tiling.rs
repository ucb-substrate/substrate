use arcstr::ArcStr;
use common::{out_path, setup_ctx};
use serde::{Deserialize, Serialize};
use subgeom::orientation::Named;
use subgeom::{Corner as SubCorner, Point, Rect};
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::into_grid;
use substrate::layout::cell::{CellPort, PortConflictStrategy, PortId};
use substrate::layout::context::LayoutCtx;
use substrate::layout::layers::selector::Selector;
use substrate::layout::placement::grid::GridTiler;
use substrate::layout::placement::nine_patch::{NpTiler, Region};
use substrate::layout::placement::place_bbox::PlaceBbox;
use substrate::layout::placement::tile::{Pad, Padding};

mod common;

pub struct Corner;
pub struct Edge;
pub struct TileEdge;
pub struct TileEnd;
pub struct Center;
pub struct TiledPorts;
pub struct MergeTiledPorts;
pub struct TiledCells(TilingParams);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TilingParams {
    nx: usize,
    ny: usize,
    padding: i64,
}

impl Component for Corner {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }
    fn name(&self) -> ArcStr {
        arcstr::literal!("corner")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(2))?;
        let rect = Rect::new(Point::zero(), Point::new(200, 200));
        ctx.draw_rect(layer, rect);

        Ok(())
    }
}

impl Component for Edge {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("edge")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(1))?;
        let rect = Rect::new(Point::zero(), Point::new(200, 200));
        ctx.draw_rect(layer, rect);

        let port = Rect::new(Point::new(0, 50), Point::new(100, 150));
        ctx.add_port(CellPort::builder().id("vdd").add(layer, port).build())
            .unwrap();

        Ok(())
    }
}

impl Component for TileEdge {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("tile_edge")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(1))?;
        let rect = Rect::new(Point::zero(), Point::new(200, 200));
        ctx.draw_rect(layer, rect);

        let data0 = Rect::new(Point::new(0, 50), Point::new(50, 150));
        ctx.add_port(
            CellPort::builder()
                .id(PortId::new("data", 0))
                .add(layer, data0)
                .build(),
        )
        .unwrap();

        let data1 = Rect::new(Point::new(150, 50), Point::new(200, 150));
        ctx.add_port(
            CellPort::builder()
                .id(PortId::new("data", 1))
                .add(layer, data1)
                .build(),
        )
        .unwrap();

        Ok(())
    }
}

impl Component for TileEnd {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("tile_end")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(1))?;
        let rect = Rect::new(Point::zero(), Point::new(200, 200));
        ctx.draw_rect(layer, rect);

        let data = Rect::new(Point::new(150, 50), Point::new(200, 150));
        ctx.add_port(CellPort::builder().id("data").add(layer, data).build())
            .unwrap();

        Ok(())
    }
}

impl Component for Center {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("center")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let layer = ctx.layers().get(Selector::Metal(0))?;
        let rect = Rect::new(Point::zero(), Point::new(200, 200));
        ctx.draw_rect(layer, rect);

        Ok(())
    }
}

impl Component for TiledPorts {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("tiled_ports")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let edge = ctx.instantiate::<Edge>(&NoParams)?;

        let mut tiler = GridTiler::new(into_grid![[
            edge.clone(),
            edge.clone(),
            edge.clone(),
            edge.clone(),
            edge
        ]]);

        tiler.expose_ports(
            |mut port: CellPort, (_, j)| {
                port.set_id(PortId::new(port.name(), j));
                Some(port)
            },
            PortConflictStrategy::Merge,
        )?;

        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw(tiler)?;

        Ok(())
    }
}

impl Component for MergeTiledPorts {
    type Params = NoParams;
    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("merge_tiled_ports")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let edge = ctx.instantiate::<TileEdge>(&NoParams)?;
        let lend = ctx.instantiate::<TileEnd>(&NoParams)?;
        let mut rend = lend.clone();
        rend.set_orientation(Named::ReflectHoriz);

        let mut tiler = GridTiler::new(into_grid![[
            lend,
            edge.clone(),
            edge.clone(),
            edge.clone(),
            edge.clone(),
            edge,
            rend
        ]]);

        tiler.expose_ports(
            |mut port: CellPort, (_, j)| {
                port.set_id(PortId::new(
                    port.name(),
                    if j == 0 { 0 } else { j + port.id().index() - 1 },
                ));
                Some(port)
            },
            PortConflictStrategy::Merge,
        )?;

        ctx.add_ports(tiler.ports().cloned()).unwrap();

        ctx.draw(tiler)?;

        Ok(())
    }
}

impl Component for TiledCells {
    type Params = TilingParams;
    fn new(params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self(params.clone()))
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("tiled_array")
    }

    fn layout(&self, ctx: &mut LayoutCtx) -> substrate::error::Result<()> {
        let corner = ctx.instantiate::<Corner>(&NoParams)?;
        let edge = ctx.instantiate::<Edge>(&NoParams)?;
        let center = ctx.instantiate::<Center>(&NoParams)?;

        let padding = Padding::uniform(self.0.padding);

        let corner = Pad::new(corner, padding);
        let mut edge = Pad::new(edge, padding);
        let center = Pad::new(center, padding);

        let tiler = NpTiler::builder()
            .set(Region::CornerLl, corner.clone())
            .set(Region::CornerLr, corner.clone())
            .set(Region::CornerUl, corner.clone())
            .set(Region::CornerUr, corner)
            .set(Region::Top, edge.clone())
            .set(Region::Bottom, edge.clone())
            .set(Region::Left, edge.clone())
            .set(Region::Right, edge.clone())
            .set(Region::Center, center)
            .nx(self.0.nx)
            .ny(self.0.ny)
            .build();

        let grid = tiler.into_grid_tiler();
        let cell = grid.cell(0, 2);
        edge.place(SubCorner::LowerLeft, cell.corner(SubCorner::LowerLeft));
        let edge = edge.into_inner();
        let port = edge.port(PortId::from("vdd"))?;
        ctx.add_port(port).unwrap();

        let cell = grid.cell(0, 3);
        let edge = ctx.instantiate::<Edge>(&NoParams)?;
        let mut edge = Pad::new(edge, padding);
        edge.place(SubCorner::LowerLeft, cell.corner(SubCorner::LowerLeft));
        let edge = edge.into_inner();
        let port = edge.port(PortId::from("vdd"))?;
        ctx.merge_port(port);

        ctx.draw(grid)?;

        Ok(())
    }
}

#[test]
fn test_tiling_grid_with_ports() {
    setup_ctx()
        .write_layout::<TiledPorts>(
            &NoParams,
            out_path("test_tiling_grid_with_ports", "layout.gds"),
        )
        .expect("failed to write layout");
}

#[test]
fn test_tiling_grid_with_merged_ports() {
    setup_ctx()
        .write_layout::<MergeTiledPorts>(
            &NoParams,
            out_path("test_tiling_grid_with_merged_ports", "layout.gds"),
        )
        .expect("failed to write layout");
}

#[test]
fn test_tiling_ninepatch_6x4() {
    setup_ctx()
        .write_layout::<TiledCells>(
            &TilingParams {
                nx: 6,
                ny: 4,
                padding: 0,
            },
            out_path("test_tiling_ninepatch", "layout_6x4.gds"),
        )
        .expect("failed to write layout");
}

#[test]
fn test_tiling_ninepatch_3x5() {
    setup_ctx()
        .write_layout::<TiledCells>(
            &TilingParams {
                nx: 3,
                ny: 5,
                padding: 0,
            },
            out_path("test_tiling_ninepatch", "layout_3x5.gds"),
        )
        .expect("failed to write layout");
}

#[test]
fn test_tiling_ninepatch_8x5_padded() {
    setup_ctx()
        .write_layout::<TiledCells>(
            &TilingParams {
                nx: 8,
                ny: 5,
                padding: 40,
            },
            out_path("test_tiling_ninepatch", "layout_8x5_padded.gds"),
        )
        .expect("failed to write layout");
}

#[test]
fn test_tiling_ninepatch_4x9_padded_rect_mode() {
    setup_ctx()
        .write_layout::<TiledCells>(
            &TilingParams {
                nx: 4,
                ny: 9,
                padding: 80,
            },
            out_path("test_tiling_ninepatch", "layout_4x9_padded.gds"),
        )
        .expect("failed to write layout");
}
