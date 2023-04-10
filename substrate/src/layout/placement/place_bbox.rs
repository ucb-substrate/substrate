//! Place the bounding box of an object.

use subgeom::bbox::BoundBox;
use subgeom::transform::Translate;
use subgeom::{Corner, Point};

pub trait PlaceBbox: BoundBox {
    /// Places the given corner of this object at the given point.
    fn place(&mut self, corner: Corner, pt: Point);
    /// Places the center of this object at the given point.
    fn place_center(&mut self, pt: Point);
    /// Places the center of this object at the given x-coordinate.
    ///
    /// Does not modify the y-coordinate of this object.
    fn place_center_x(&mut self, pt: i64);
    /// Places the center of this object at the given y-coordinate.
    ///
    /// Does not modify the x-coordinate of this object.
    fn place_center_y(&mut self, pt: i64);
}

impl<T> PlaceBbox for T
where
    T: Translate + BoundBox,
{
    /// Places the given corner of this object at the given point.
    ///
    /// # Panics
    ///
    /// This function panics if called on an object with
    /// an empty bounding box.
    fn place(&mut self, corner: Corner, pt: Point) {
        let rect = self.bbox().into_rect();
        let ofs = pt - rect.corner(corner);
        self.translate(ofs);
    }
    /// Places the center of this object at the given point.
    ///
    /// # Panics
    ///
    /// This function panics if called on an object with
    /// an empty bounding box, or if the bounding box's
    /// center is not an integer.
    fn place_center(&mut self, pt: Point) {
        let rect = self.bbox().into_rect();
        assert_eq!((rect.p0.x + rect.p1.x) % 2, 0);
        assert_eq!((rect.p0.y + rect.p1.y) % 2, 0);
        let center = rect.center();
        let ofs = pt - center;
        self.translate(ofs);
    }

    fn place_center_x(&mut self, pt: i64) {
        let rect = self.bbox().into_rect();
        assert_eq!((rect.p0.x + rect.p1.x) % 2, 0);
        let ofs = pt - rect.center().x;
        self.translate(Point::new(ofs, 0));
    }

    fn place_center_y(&mut self, pt: i64) {
        let rect = self.bbox().into_rect();
        assert_eq!((rect.p0.y + rect.p1.y) % 2, 0);
        let ofs = pt - rect.center().y;
        self.translate(Point::new(0, ofs));
    }
}
