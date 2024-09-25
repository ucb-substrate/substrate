use itertools::Itertools;
use subgeom::bbox::BoundBox;
use subgeom::transform::Translate;
use subgeom::{Point, Rect};

use super::align::{AlignMode, AlignRect};
use super::tile::Tile;
use crate::layout::cell::{CellPort, PortConflictStrategy, PortMap, PortMapFn};
use crate::layout::group::Group;
use crate::layout::{Draw, DrawRef};

#[derive(Clone)]
pub struct ArrayTiler<'a> {
    tiles: Vec<Tile<'a>>,
    ports: PortMap,
    mode: AlignMode,
    alt_mode: Option<AlignMode>,
    cells: Vec<Rect>,
}

pub trait ArrayPortMapFn: PortMapFn<usize> {}
impl<F> ArrayPortMapFn for F where F: PortMapFn<usize> {}

#[derive(Clone, Default)]
pub struct ArrayTilerBuilder<'a> {
    tiles: Vec<Tile<'a>>,
    mode: Option<AlignMode>,
    alt_mode: Option<AlignMode>,
    space: i64,
    alt_space: i64,
}

impl<'a> ArrayTilerBuilder<'a> {
    #[inline]
    pub fn new() -> Self {
        ArrayTilerBuilder::<'a>::default()
    }

    #[inline]
    pub fn mode(&mut self, mode: impl Into<AlignMode>) -> &mut Self {
        self.mode = Some(mode.into());
        self
    }

    #[inline]
    pub fn space(&mut self, space: i64) -> &mut Self {
        self.space = Some(space);
        self
    }

    #[inline]
    pub fn alt_mode(&mut self, mode: impl Into<AlignMode>) -> &mut Self {
        self.alt_mode = Some(mode.into());
        self
    }

    #[inline]
    pub fn alt_space(&mut self, alt_space: i64) -> &mut Self {
        self.alt_space = Some(alt_space);
        self
    }

    #[inline]
    pub fn push<'b>(&mut self, tile: impl Into<Tile<'b>>) -> &mut Self
    where
        'b: 'a,
    {
        self.tiles.push(tile.into());
        self
    }

    pub fn push_num_ref<'b, 'c>(&mut self, tile: &'b Tile<'c>, num: usize) -> &mut Self
    where
        'b: 'a,
        'c: 'a,
    {
        self.tiles.reserve(num);
        (0..num).for_each(|_| self.tiles.push(tile.borrowed()));
        self
    }

    pub fn push_num<'b>(&mut self, tile: impl Into<Tile<'b>>, num: usize) -> &mut Self
    where
        'b: 'a,
    {
        self.tiles.reserve(num);
        let tile = tile.into();
        (0..num).for_each(|_| self.tiles.push(tile.clone()));
        self
    }

    #[inline]
    pub fn build(&mut self) -> ArrayTiler<'a> {
        ArrayTiler::new(self.clone())
    }
}

impl<'a> ArrayTiler<'a> {
    #[inline]
    pub fn builder() -> ArrayTilerBuilder<'a> {
        ArrayTilerBuilder::<'a>::new()
    }

    pub fn new<'b>(builder: ArrayTilerBuilder<'b>) -> Self
    where
        'b: 'a,
    {
        let mode = builder.mode.unwrap();

        let mut prev = Rect::from_point(Point::zero());
        let cells = builder
            .tiles
            .iter()
            .map(|tile| {
                let mut rect = tile.brect();
                rect.align(mode, prev, builder.space);
                if let Some(mode) = builder.alt_mode {
                    rect.align(mode, prev, builder.alt_space);
                }
                prev = rect;
                rect
            })
            .collect_vec();

        debug_assert_eq!(builder.tiles.len(), cells.len());

        Self {
            tiles: builder.tiles,
            ports: PortMap::new(),
            mode,
            alt_mode: builder.alt_mode,
            cells,
        }
    }

    pub fn expose_ports(
        &mut self,
        mut port_map_fn: impl ArrayPortMapFn,
        port_conflict_strategy: PortConflictStrategy,
    ) -> crate::error::Result<()> {
        for (i, (tile, cell)) in self.tiles.iter().zip(self.cells.iter()).enumerate() {
            let mut tgroup = tile.draw_ref()?;
            tgroup.translate(translation(tile, cell));
            self.ports.add_ports_with_strategy(
                tgroup.ports().filter_map(|port| port_map_fn.map(port, i)),
                port_conflict_strategy,
            )?;
        }

        Ok(())
    }

    #[inline]
    pub fn ports(&self) -> impl Iterator<Item = &CellPort> {
        self.ports.ports()
    }

    #[inline]
    pub fn port_map(&self) -> &PortMap {
        &self.ports
    }

    #[inline]
    pub fn mode(&self) -> AlignMode {
        self.mode
    }

    #[inline]
    pub fn alt_mode(&self) -> Option<AlignMode> {
        self.alt_mode
    }

    pub fn generate(&self) -> crate::error::Result<Group> {
        let mut group = Group::new();

        for (tile, cell) in self.tiles.iter().zip(self.cells.iter()) {
            let mut tgroup = tile.draw_ref()?;
            tgroup.translate(translation(tile, cell));
            group.add_group(tgroup);
        }

        group.add_ports(self.ports().cloned()).unwrap();

        Ok(group)
    }

    #[inline]
    pub fn translation(&self, i: usize) -> Point {
        translation(&self.tiles[i], &self.cells[i])
    }

    #[inline]
    pub fn cell(&self, i: usize) -> Rect {
        self.cells[i]
    }
}

#[inline]
fn translation(tile: &Tile, cell: &Rect) -> Point {
    cell.p0 - tile.bbox().p0
}

impl<'a> Draw for ArrayTiler<'a> {
    fn draw(self) -> crate::error::Result<Group> {
        let group = self.generate()?;
        Ok(group)
    }
}

impl<'a> DrawRef for ArrayTiler<'a> {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        let group = self.generate()?;
        Ok(group)
    }
}
