//! Rectangular bounding boxes and associated trait implementations.

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use super::{Point, Rect, Shape};

/// An axis-aligned rectangular bounding box.
///
/// Points `p0` and `p1` represent opposite corners of a bounding rectangle.
/// `p0` is always closest to negative-infinity, in both x and y,
/// and `p1` is always closest to positive-infinity.
///
/// This differs from [`Rect`] in that it could be empty, meaning that `p0`
/// is to the upper right of `p1`.
///
#[derive(Debug, Default, Copy, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Bbox {
    pub p0: Point,
    pub p1: Point,
}
impl Bbox {
    /// Create a new [`Bbox`] from two [`Point`]s.
    #[inline]
    pub fn new(p0: Point, p1: Point) -> Self {
        Self {
            p0: Point::new(p0.x.min(p1.x), p0.y.min(p1.y)),
            p1: Point::new(p0.x.max(p1.x), p0.y.max(p1.y)),
        }
    }
    /// Creates a bounding box that encloses only the origin `(0, 0)`.
    #[inline]
    pub fn zero() -> Self {
        Self::new(Point::zero(), Point::zero())
    }
    /// Finds the width of the bounding box in the x-direction.
    #[inline]
    pub fn width(&self) -> i64 {
        self.p1.x - self.p0.x
    }
    /// Finds the height of the bounding box in the y-direction.
    #[inline]
    pub fn height(&self) -> i64 {
        self.p1.y - self.p0.y
    }
    /// Creates a new [`Bbox`] from a single [`Point`].
    ///
    /// The resultant [`Bbox`] comprises solely of the point, having zero area.
    pub fn from_point(pt: Point) -> Self {
        Self { p0: pt, p1: pt }
    }
    /// Creates a new [`Bbox`] from two points without any computation.
    ///
    /// Callers are responsible for ensuring that p0.x <= p1.x, and p0.y <= p1.y.
    fn from_points(p0: Point, p1: Point) -> Self {
        Self { p0, p1 }
    }
    /// Creates an empty, otherwise invalid bounding box.
    pub fn empty() -> Self {
        Self {
            p0: Point::new(i64::MAX, i64::MAX),
            p1: Point::new(i64::MIN, i64::MIN),
        }
    }
    /// Returns `true` if the bounding box is empty.
    pub fn is_empty(&self) -> bool {
        self.p0.x > self.p1.x || self.p0.y > self.p1.y
    }
    /// Returns true if [`Point`] `pt` lies inside the bounding box.
    pub fn contains(&self, pt: Point) -> bool {
        self.p0.x <= pt.x && self.p1.x >= pt.x && self.p0.y <= pt.y && self.p1.y >= pt.y
    }
    /// Expands an existing [`Bbox`] in all directions by `delta`.
    pub fn expand(&mut self, delta: i64) {
        self.p0.x -= delta;
        self.p0.y -= delta;
        self.p1.x += delta;
        self.p1.y += delta;
    }
    /// Returns the bounding box's size as an (x,y) tuple.
    pub fn size(&self) -> (i64, i64) {
        (self.width(), self.height())
    }
    /// Returns the bounding box's center.
    pub fn center(&self) -> Point {
        Point::new((self.p0.x + self.p1.x) / 2, (self.p0.y + self.p1.y) / 2)
    }

    /// Converts a bounding box into a [`Rect`].
    #[inline]
    pub fn into_rect(self) -> Rect {
        Rect::from(self)
    }
}

impl From<Rect> for Bbox {
    fn from(r: Rect) -> Self {
        debug_assert!(r.p0.x <= r.p1.x);
        debug_assert!(r.p0.y <= r.p1.y);
        Self { p0: r.p0, p1: r.p1 }
    }
}

/// A trait representing functions available for objects with a bounding box.
#[enum_dispatch]
pub trait BoundBox {
    /// Compute a rectangular bounding box around the implementing type.
    fn bbox(&self) -> Bbox;
    /// Computes the rectangular bounding box and converts it to a [`Rect`].
    ///
    /// # Panics
    ///
    /// This function may panic if the bounding box is empty.
    fn brect(&self) -> Rect {
        self.bbox().into_rect()
    }
    /// Computes the intersection with rectangular bounding box `bbox`.
    ///
    /// Creates and returns a new [`Bbox`].
    /// Default implementation is to return the intersection of `self.bbox()` and `bbox`.
    fn intersection(&self, bbox: Bbox) -> Bbox {
        self.bbox().intersection(bbox)
    }
    /// Computes the union with rectangular bounding box `bbox`.
    ///
    /// Creates and returns a new [Bbox].
    /// Default implementation is to return the union of `self.bbox()` and `bbox`.
    fn union(&self, bbox: Bbox) -> Bbox {
        self.bbox().union(bbox)
    }
}

impl<T> BoundBox for &T
where
    T: BoundBox,
{
    fn bbox(&self) -> Bbox {
        T::bbox(*self)
    }
}

impl BoundBox for Bbox {
    fn bbox(&self) -> Bbox {
        // We're great as we are, as a [Bbox] already.
        // Create a clone to adhere to our "new bbox" return-type.
        *self
    }
    fn intersection(&self, bbox: Bbox) -> Bbox {
        let pmin = Point::new(self.p0.x.max(bbox.p0.x), self.p0.y.max(bbox.p0.y));
        let pmax = Point::new(self.p1.x.min(bbox.p1.x), self.p1.y.min(bbox.p1.y));
        // Check for empty intersection, and return an empty box if so
        if pmin.x > pmax.x || pmin.y > pmax.y {
            return Bbox::empty();
        }
        // Otherwise return the intersection
        Bbox::new(pmin, pmax)
    }
    fn union(&self, bbox: Bbox) -> Bbox {
        if bbox.is_empty() {
            return *self;
        }
        if self.is_empty() {
            return bbox;
        }
        // Take the minimum and maximum of the two bounding boxes
        Bbox::new(
            Point::new(self.p0.x.min(bbox.p0.x), self.p0.y.min(bbox.p0.y)),
            Point::new(self.p1.x.max(bbox.p1.x), self.p1.y.max(bbox.p1.y)),
        )
    }
}
impl BoundBox for Point {
    fn bbox(&self) -> Bbox {
        Bbox::from_point(*self)
    }
    fn intersection(&self, bbox: Bbox) -> Bbox {
        if !bbox.contains(*self) {
            return Bbox::empty();
        }
        bbox.intersection(Bbox::from_point(*self))
    }
    fn union(&self, bbox: Bbox) -> Bbox {
        Bbox::new(
            Point::new(self.x.min(bbox.p0.x), self.y.min(bbox.p0.y)),
            Point::new(self.x.max(bbox.p1.x), self.y.max(bbox.p1.y)),
        )
    }
}
impl BoundBox for Shape {
    fn bbox(&self) -> Bbox {
        // Dispatch based on shape-type, either two-Point or multi-Point form.
        match self {
            Shape::Rect(r) => Bbox::from_points(r.p0, r.p1),
            Shape::Polygon(p) => p.points.bbox(),
            Shape::Path(p) => p.points.bbox(),
            Shape::Point(p) => Bbox::from_point(*p),
        }
    }
}

impl BoundBox for Rect {
    fn bbox(&self) -> Bbox {
        Bbox::from_points(self.p0, self.p1)
    }
}

impl BoundBox for Vec<Point> {
    fn bbox(&self) -> Bbox {
        // Take the union of all points in the vector
        let mut bbox = Bbox::empty();
        for pt in self {
            bbox = bbox.union(pt.bbox());
        }
        bbox
    }
}
