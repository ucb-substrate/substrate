use grid::Grid;
use subgeom::bbox::BoundBox;
use subgeom::transform::Translate;
use subgeom::{Dims, Point, Rect};

use super::nine_patch::NpTiler;
use super::tile::{OptionTile, Tile};
use crate::layout::cell::{CellPort, PortConflictStrategy, PortMap, PortMapFn};
use crate::layout::group::Group;
use crate::layout::{Draw, DrawRef};

pub struct GridTiler<'a> {
    tiles: Grid<OptionTile<'a>>,
    ports: PortMap,
    /// The x coordinate of the lower left of each column.
    ///
    /// `pos_ll_x[0]` represents the position of the **left-most** column.
    pos_ll_x: Vec<i64>,

    /// The y coordinate of the lower left of each row.
    ///
    /// `pos_ll_y[0]` represents the position of the **bottom-most** row.
    pos_ll_y: Vec<i64>,

    /// The height of each row.
    ///
    /// `row_heights[0]` represents the height of the **top-most** row.
    row_heights: Vec<i64>,

    /// The width of each column.
    ///
    /// `col_widths[0]` represents the width of the **left-most** column.
    col_widths: Vec<i64>,
}

pub trait GridPortMapFn: PortMapFn<(usize, usize)> {}
impl<F> GridPortMapFn for F where F: PortMapFn<(usize, usize)> {}

impl<'a> GridTiler<'a> {
    pub fn new(tiles: Grid<OptionTile<'a>>) -> Self {
        let rows = tiles.rows();
        let cols = tiles.cols();

        let mut row_heights: Vec<Option<i64>> = vec![None; rows];
        let mut col_widths: Vec<Option<i64>> = vec![None; cols];

        for (i, height) in row_heights.iter_mut().enumerate() {
            for (j, width) in col_widths.iter_mut().enumerate() {
                if let Some(tile) = tiles.get(i, j).unwrap().as_ref() {
                    let dims = tile.dims();
                    if let Some(height) = *height {
                        assert_eq!(dims.height(), height);
                    }
                    if let Some(width) = *width {
                        assert_eq!(dims.width(), width);
                    }
                    *height = Some(dims.height());
                    *width = Some(dims.width());
                }
            }
        }

        let mut pos_ll_y: Vec<i64> = vec![0; rows];
        let mut pos_ll_x: Vec<i64> = vec![0; cols];
        for i in 0..rows {
            pos_ll_y[i] = if i == 0 {
                0
            } else {
                pos_ll_y[i - 1] + row_heights[rows - i].unwrap_or_default()
            };
        }
        for j in 0..cols {
            pos_ll_x[j] = if j == 0 {
                0
            } else {
                pos_ll_x[j - 1] + col_widths[j - 1].unwrap_or_default()
            };
        }
        Self {
            tiles,
            ports: PortMap::new(),
            pos_ll_x,
            pos_ll_y,
            row_heights: row_heights
                .into_iter()
                .map(|x| x.unwrap_or_default())
                .collect(),
            col_widths: col_widths
                .into_iter()
                .map(|x| x.unwrap_or_default())
                .collect(),
        }
    }

    pub fn new_with_ports(
        tiles: Grid<OptionTile<'a>>,
        port_map_fn: impl GridPortMapFn,
        port_conflict_strategy: PortConflictStrategy,
    ) -> crate::error::Result<Self> {
        let mut tiler = Self::new(tiles.clone());

        tiler.expose_ports(port_map_fn, port_conflict_strategy)?;

        Ok(tiler)
    }

    pub fn expose_ports(
        &mut self,
        mut port_map_fn: impl GridPortMapFn,
        port_conflict_strategy: PortConflictStrategy,
    ) -> crate::error::Result<()> {
        let rows = self.tiles.rows();
        let cols = self.tiles.cols();

        for i in 0..rows {
            for j in 0..cols {
                if let Some(tile) = self.tiles[i][j].as_ref() {
                    let pos = self.pos_ll(j, rows - i - 1);
                    let pt = pos - tile.bbox().p0;
                    let mut tgroup = tile.draw_ref()?;
                    tgroup.translate(pt);
                    self.ports.add_ports_with_strategy(
                        tgroup
                            .ports()
                            .filter_map(|port| port_map_fn.map(port, (i, j))),
                        port_conflict_strategy,
                    )?;
                }
            }
        }

        Ok(())
    }

    pub fn ports(&self) -> impl Iterator<Item = &CellPort> {
        self.ports.ports()
    }

    pub(crate) fn generate(&self) -> crate::error::Result<Group> {
        let mut group = Group::new();

        let rows = self.tiles.rows();
        let cols = self.tiles.cols();

        for i in 0..rows {
            for j in 0..cols {
                if let Some(tile) = self.tiles[i][j].as_ref() {
                    let pos = self.pos_ll(j, rows - i - 1);
                    let pt = pos - tile.bbox().p0;
                    let mut tgroup = tile.draw_ref()?;
                    tgroup.translate(pt);
                    group.add_group(tgroup);
                }
            }
        }

        Ok(group)
    }

    /// Gets the [`Tile`] at the given row and column index.
    ///
    /// # Panics
    ///
    /// This function panics if `i` or `j` are out of bounds,
    /// or if no [`Tile`] was specified at the given position.
    pub fn tile(&self, i: usize, j: usize) -> &Tile {
        self.tiles[i][j].as_ref().unwrap()
    }

    /// Gets the [`Rect`] representing the cell in row `i`, column `j`.
    ///
    /// # Panics
    ///
    /// This function panics if `i` or `j` are out of bounds.
    pub fn cell(&self, i: usize, j: usize) -> Rect {
        let p0 = self.pos_ll(j, self.tiles.rows() - i - 1);
        let dims = Dims::new(self.col_widths[j], self.row_heights[i]);
        let p1 = p0 + dims;
        Rect::new(p0, p1)
    }

    pub fn translation(&self, i: usize, j: usize) -> Point {
        let pos = self.pos_ll(j, self.tiles.rows() - i - 1);
        let pt = pos - self.tile(i, j).bbox().p0;
        pt
    }

    fn pos_ll(&self, x: usize, y: usize) -> Point {
        Point::new(self.pos_ll_x(x), self.pos_ll_y(y))
    }

    fn pos_ll_x(&self, n: usize) -> i64 {
        self.pos_ll_x[n]
    }

    fn pos_ll_y(&self, n: usize) -> i64 {
        self.pos_ll_y[n]
    }
}

impl<'a> Draw for GridTiler<'a> {
    fn draw(self) -> crate::error::Result<Group> {
        self.draw_ref()
    }
}

impl<'a> DrawRef for GridTiler<'a> {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        let group = self.generate()?;
        Ok(group)
    }
}

impl<'a> From<NpTiler<'a>> for GridTiler<'a> {
    #[inline]
    fn from(value: NpTiler<'a>) -> Self {
        value.into_grid_tiler()
    }
}
