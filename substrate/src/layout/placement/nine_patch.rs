use array_map::{ArrayMap, Indexable};
use grid::Grid;

use super::grid::GridTiler;
use super::tile::{OptionTile, Tile};
use crate::layout::group::Group;
use crate::layout::Draw;

/// Nine patch tiling regions.
///
///
/// A diagram showing the positioning of each region is shown below.
/// ```text
///           |        |
///  CornerUl |  Top   | CornerUr
///           |        |
/// ----------+--------|----------
///           |        |
///      Left | Center | Right
///           |        |
/// ----------+--------+----------
///           |        |
///  CornerLl | Bottom | CornerLr
///           |        |
/// ```
///
/// The corner regions will not be tiled at all.
/// The edges will be tiled in one dimension.
/// The center will be tiled in two dimensions.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(u8)]
#[derive(Indexable)]
pub enum Region {
    CornerUl,
    CornerUr,
    CornerLr,
    CornerLl,
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

pub struct NpTiler<'a> {
    patches: ArrayMap<Region, OptionTile<'a>, 9>,
    nx: usize,
    ny: usize,
}

impl<'a> NpTiler<'a> {
    #[inline]
    pub fn builder() -> NpTilerBuilder<'a> {
        NpTilerBuilder::new()
    }

    fn patch(&self, x: usize, y: usize) -> &OptionTile<'a> {
        debug_assert!(x <= self.nx + 1);
        debug_assert!(y <= self.ny + 1);

        let region = if x == 0 && y == 0 {
            Region::CornerLl
        } else if x == self.nx + 1 && y == 0 {
            Region::CornerLr
        } else if x == 0 && y == self.ny + 1 {
            Region::CornerUl
        } else if x == self.nx + 1 && y == self.ny + 1 {
            Region::CornerUr
        } else if x == 0 {
            Region::Left
        } else if x == self.nx + 1 {
            Region::Right
        } else if y == 0 {
            Region::Bottom
        } else if y == self.ny + 1 {
            Region::Top
        } else {
            Region::Center
        };

        &self.patches[region]
    }

    pub fn into_grid_tiler(self) -> GridTiler<'a> {
        let mut grid = Grid::<OptionTile>::new(self.ny + 2, self.nx + 2);

        for x in 0..=self.nx + 1 {
            for y in 0..=self.ny + 1 {
                let entry = grid.get_mut(self.ny + 1 - y, x).unwrap();
                *entry = self.patch(x, y).clone();
            }
        }

        GridTiler::new(grid)
    }
}

impl<'a> Draw for NpTiler<'a> {
    fn draw(self) -> crate::error::Result<Group> {
        self.into_grid_tiler().draw()
    }
}

#[derive(Default)]
pub struct NpTilerBuilder<'a> {
    patches: ArrayMap<Region, OptionTile<'a>, 9>,
    nx: Option<usize>,
    ny: Option<usize>,
}

impl<'a> NpTilerBuilder<'a> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<'b>(mut self, region: Region, content: impl Into<Tile<'b>>) -> Self
    where
        'b: 'a,
    {
        self.patches[region] = OptionTile::from(content.into());
        self
    }

    pub fn nx(mut self, nx: usize) -> Self {
        self.nx = Some(nx);
        self
    }

    pub fn ny(mut self, ny: usize) -> Self {
        self.ny = Some(ny);
        self
    }

    pub fn build(self) -> NpTiler<'a> {
        NpTiler {
            patches: self.patches,
            nx: self.nx.unwrap(),
            ny: self.ny.unwrap(),
        }
    }
}
