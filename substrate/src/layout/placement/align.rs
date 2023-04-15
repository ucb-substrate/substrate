//! Alignment APIs.

use serde::{Deserialize, Serialize};
use subgeom::bbox::BoundBox;
use subgeom::transform::Translate;
use subgeom::{snap_to_grid, Corner, Point, Rect, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlignMode {
    Left,
    Right,
    Bottom,
    Top,
    CenterHorizontal,
    CenterVertical,
    ToTheRight,
    ToTheLeft,
    Beneath,
    Above,
}

pub trait AlignRect: Translate + BoundBox {
    fn align(&mut self, mode: AlignMode, obox: impl BoundBox, space: i64) -> &mut Self {
        let obox = obox.bbox();
        let sbox = self.bbox();

        match mode {
            AlignMode::Left => {
                self.translate(Point::new(obox.p0.x - sbox.p0.x + space, 0));
            }
            AlignMode::Right => {
                self.translate(Point::new(obox.p1.x - sbox.p1.x + space, 0));
            }
            AlignMode::Bottom => {
                self.translate(Point::new(0, obox.p0.y - sbox.p0.y + space));
            }
            AlignMode::Top => {
                self.translate(Point::new(0, obox.p1.y - sbox.p1.y + space));
            }
            AlignMode::ToTheRight => {
                self.translate(Point::new(obox.p1.x - sbox.p0.x + space, 0));
            }
            AlignMode::ToTheLeft => {
                self.translate(Point::new(obox.p0.x - sbox.p1.x - space, 0));
            }
            AlignMode::CenterHorizontal => {
                self.translate(Point::new(
                    ((obox.p0.x + obox.p1.x) - (sbox.p0.x + sbox.p1.x)) / 2 + space,
                    0,
                ));
            }
            AlignMode::CenterVertical => {
                self.translate(Point::new(
                    0,
                    ((obox.p0.y + obox.p1.y) - (sbox.p0.y + sbox.p1.y)) / 2 + space,
                ));
            }
            AlignMode::Beneath => {
                self.translate(Point::new(0, obox.p0.y - sbox.p1.y - space));
            }
            AlignMode::Above => {
                self.translate(Point::new(0, obox.p1.y - sbox.p0.y + space));
            }
        }

        self
    }

    fn align_left(&mut self, other: impl BoundBox) {
        self.align(AlignMode::Left, other, 0);
    }

    fn align_right(&mut self, other: impl BoundBox) {
        self.align(AlignMode::Right, other, 0);
    }

    fn align_bottom(&mut self, other: impl BoundBox) {
        self.align(AlignMode::Bottom, other, 0);
    }

    fn align_top(&mut self, other: impl BoundBox) {
        self.align(AlignMode::Top, other, 0);
    }

    fn align_to_the_right_of(&mut self, other: impl BoundBox, space: i64) {
        self.align(AlignMode::ToTheRight, other, space);
    }

    fn align_to_the_left_of(&mut self, other: impl BoundBox, space: i64) {
        self.align(AlignMode::ToTheLeft, other, space);
    }

    fn align_centers_horizontally(&mut self, other: impl BoundBox) {
        self.align(AlignMode::CenterHorizontal, other, 0);
    }

    fn align_centers_vertically(&mut self, other: impl BoundBox) {
        self.align(AlignMode::CenterVertical, other, 0);
    }

    fn align_centers(&mut self, other: impl BoundBox) {
        self.align_centers_horizontally(&other);
        self.align_centers_vertically(&other);
    }

    fn align_beneath(&mut self, other: impl BoundBox, space: i64) {
        self.align(AlignMode::Beneath, other, space);
    }

    fn align_above(&mut self, other: impl BoundBox, space: i64) {
        self.align(AlignMode::Above, other, space);
    }

    fn align_centers_horizontally_gridded(&mut self, other: impl BoundBox, grid: i64) {
        // Align the center
        self.align(AlignMode::CenterHorizontal, other, 0);

        // Then snap to the nearest grid location
        let bbox = self.bbox();
        assert_eq!(bbox.width() % grid, 0);
        let offset = snap_to_grid(bbox.p0.x, grid) - bbox.p0.x;
        self.translate(Point::new(offset, 0));
    }

    fn align_centers_vertically_gridded(&mut self, other: impl BoundBox, grid: i64) {
        // Align the center
        self.align(AlignMode::CenterVertical, other, 0);

        // Then snap to the nearest grid location
        let bbox = self.bbox();
        assert_eq!(bbox.height() % grid, 0);
        let offset = snap_to_grid(bbox.p0.y, grid) - bbox.p0.y;
        self.translate(Point::new(0, offset));
    }

    fn align_centers_gridded(&mut self, other: impl BoundBox, grid: i64) {
        self.align_centers_horizontally_gridded(&other, grid);
        self.align_centers_vertically_gridded(&other, grid);
    }

    /// Aligns the given corner of this object's bounding box
    /// to the given target point.
    fn align_corner(&mut self, corner: Corner, target: Point) {
        let corner = self.bbox().into_rect().corner(corner);
        let ofs = target - corner;
        self.translate(ofs);
    }

    /// Aligns the given corner to a grid.
    fn align_corner_to_grid(&mut self, corner: Corner, grid: i64) {
        let point = self.bbox().into_rect().corner(corner);
        let target = point.snap_to_grid(grid);
        let ofs = target - point;
        self.translate(ofs);
        let point = self.bbox().into_rect().corner(corner);
        debug_assert_eq!(point.x % grid, 0);
        debug_assert_eq!(point.y % grid, 0);
    }

    /// Aligns the given side of the bounding box to a grid.
    fn align_side_to_grid(&mut self, side: Side, grid: i64) {
        let coord = self.bbox().into_rect().side(side);
        let target = snap_to_grid(coord, grid);
        let ofs = Point::from_dir_coords(side.coord_dir(), target - coord, 0);
        self.translate(ofs);
    }
}

impl AlignRect for Rect {}
