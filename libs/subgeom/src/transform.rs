//! Transformation types and traits.

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use super::orientation::Orientation;
use super::{Path, Point, Polygon, Rect};
use crate::orientation::wrap_angle;

/// A 2x2 rotation-matrix and two-entry translation vector,
/// used for relative movement of [Point]s and [Shape](super::Shape)s.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Transformation {
    /// The transformation matrix represented in row-major order.
    pub a: [[f64; 2]; 2],
    /// The x-y translation applied after the transformation.
    pub b: [f64; 2],
}
impl Transformation {
    /// Returns the identity transform, leaving any transformed object unmodified.
    pub fn identity() -> Self {
        Self {
            a: [[1., 0.], [0., 1.]],
            b: [0., 0.],
        }
    }
    /// Returns a translation by `(x,y)`.
    pub fn translate(x: f64, y: f64) -> Self {
        Self {
            a: [[1., 0.], [0., 1.]],
            b: [x, y],
        }
    }
    /// Returns a rotatation by `angle` degrees.
    pub fn rotate(angle: f64) -> Self {
        let sin = angle.to_radians().sin();
        let cos = angle.to_radians().cos();
        Self {
            a: [[cos, -sin], [sin, cos]],
            b: [0., 0.],
        }
    }
    /// Returns a reflection about the x-axis.
    pub fn reflect_vert() -> Self {
        Self {
            a: [[1., 0.], [0., -1.]],
            b: [0., 0.],
        }
    }

    /// Returns a new [`TransformationBuilder`].
    #[inline]
    pub fn builder() -> TransformationBuilder {
        TransformationBuilder::default()
    }

    /// Creates a transform from a location and [`Orientation`].
    pub fn with_loc_and_orientation(loc: Point, orientation: impl Into<Orientation>) -> Self {
        Self::builder()
            .point(loc)
            .orientation(orientation.into())
            .build()
    }

    /// Creates a transform from a location, angle, and a bool indicating
    /// whether or not to reflect vertically.
    pub fn with_opts(loc: Point, reflect_vert: bool, angle: Option<f64>) -> Self {
        Self::builder()
            .point(loc)
            .reflect_vert(reflect_vert)
            .angle_opt(angle)
            .build()
    }

    /// Create a new [`Transformation`] that is the cascade of `parent` and `child`.
    ///
    /// "Parents" and "children" refer to typical layout-instance hierarchies,
    /// in which each layer of instance has a nested set of transformations relative to its top-level parent.
    ///
    /// Note this operation *is not* commutative.
    /// For example the set of transformations:
    /// * (a) Reflect vertically, then
    /// * (b) Translate by (1,1)
    /// * (c) Place a point at (local coordinate) (1,1)
    /// Lands said point at (2,-2) in top-level space,
    /// whereas reversing the order of (a) and (b) lands it at (2,0).
    pub fn cascade(parent: Transformation, child: Transformation) -> Transformation {
        // The result-transform's origin is the parent's origin,
        // plus the parent-transformed child's origin
        let mut b = matvec(&parent.a, &child.b);
        b[0] += parent.b[0];
        b[1] += parent.b[1];
        // And the cascade-matrix is the product of the parent's and child's
        let a = matmul(&parent.a, &child.a);
        Self { a, b }
    }

    pub fn offset_point(&self) -> Point {
        Point {
            x: self.b[0].round() as i64,
            y: self.b[1].round() as i64,
        }
    }

    pub fn orientation(&self) -> Orientation {
        let reflect_vert = self.a[0][0].signum() != self.a[1][1].signum();
        let sin = self.a[1][0];
        let cos = self.a[0][0];
        let angle = cos.acos().to_degrees();
        let angle = if sin > 0f64 {
            angle
        } else {
            wrap_angle(-angle)
        };
        Orientation {
            reflect_vert,
            angle,
        }
    }
}

impl<T> From<T> for Transformation
where
    T: Into<Orientation>,
{
    fn from(value: T) -> Self {
        Self::builder().orientation(value).build()
    }
}

/// A builder for creating transformations from translations and [`Orientation`]s.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TransformationBuilder {
    x: f64,
    y: f64,
    reflect_vert: bool,
    angle: f64,
}

impl TransformationBuilder {
    /// Specifies the x-y translation encoded by the transformation.
    pub fn point(&mut self, point: impl Into<Point>) -> &mut Self {
        let point = point.into();
        self.x = point.x as f64;
        self.y = point.y as f64;
        self
    }

    /// Specifies the [`Orientation`] applied by this transformation.
    pub fn orientation(&mut self, o: impl Into<Orientation>) -> &mut Self {
        let o = o.into();
        self.reflect_vert = o.reflect_vert;
        self.angle = o.angle;
        self
    }

    /// Specifies the angle of rotation encoded by this transformation.
    pub fn angle(&mut self, angle: f64) -> &mut Self {
        self.angle = angle;
        self
    }

    /// Specifies the angle of rotation encoded by this transformation as an [`Option`].
    pub fn angle_opt(&mut self, angle: Option<f64>) -> &mut Self {
        self.angle = angle.unwrap_or_default();
        self
    }

    /// Specifies whether the transformation results in a vertical reflection.
    pub fn reflect_vert(&mut self, reflect_vert: bool) -> &mut Self {
        self.reflect_vert = reflect_vert;
        self
    }

    /// Builds a [`Transformation`] from the specified parameters.
    pub fn build(&mut self) -> Transformation {
        let b = [self.x, self.y];
        let sin = self.angle.to_radians().sin();
        let cos = self.angle.to_radians().cos();
        let sin_refl = if self.reflect_vert { sin } else { -sin };
        let cos_refl = if self.reflect_vert { -cos } else { cos };
        let a = [[cos, sin_refl], [sin, cos_refl]];
        Transformation { a, b }
    }
}

/// Multiples two 2x2 matrices, returning a new 2x2 matrix
fn matmul(a: &[[f64; 2]; 2], b: &[[f64; 2]; 2]) -> [[f64; 2]; 2] {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ],
    ]
}
/// Multiplies a 2x2 matrix by a 2-entry vector, returning a new 2-entry vector.
fn matvec(a: &[[f64; 2]; 2], b: &[f64; 2]) -> [f64; 2] {
    [
        a[0][0] * b[0] + a[0][1] * b[1],
        a[1][0] * b[0] + a[1][1] * b[1],
    ]
}

/// A trait for specifying how an object is changed by a transformation.
#[enum_dispatch]
pub trait Transform {
    /// Applies matrix-vector [`Transformation`] `trans`.
    ///
    /// Creates a new shape at a location equal to the transformation of our own.
    fn transform(&self, trans: Transformation) -> Self;
}

impl Transform for Point {
    fn transform(&self, trans: Transformation) -> Self {
        let xf = self.x as f64;
        let yf = self.y as f64;
        let x = trans.a[0][0] * xf + trans.a[0][1] * yf + trans.b[0];
        let y = trans.a[1][0] * xf + trans.a[1][1] * yf + trans.b[1];
        Self {
            x: x.round() as i64,
            y: y.round() as i64,
        }
    }
}

impl Transform for Rect {
    fn transform(&self, trans: Transformation) -> Self {
        let (p0, p1) = (self.p0, self.p1);
        let p0p = p0.transform(trans);
        let p1p = p1.transform(trans);

        let p0 = Point::new(std::cmp::min(p0p.x, p1p.x), std::cmp::min(p0p.y, p1p.y));
        let p1 = Point::new(std::cmp::max(p0p.x, p1p.x), std::cmp::max(p0p.y, p1p.y));
        Rect { p0, p1 }
    }
}
impl Transform for Polygon {
    fn transform(&self, trans: Transformation) -> Self {
        Polygon {
            points: self.points.iter().map(|p| p.transform(trans)).collect(),
        }
    }
}
impl Transform for Path {
    fn transform(&self, trans: Transformation) -> Self {
        Path {
            points: self.points.iter().map(|p| p.transform(trans)).collect(),
            width: self.width,
        }
    }
}

/// A trait for specifying how a shape is translated by a [`Point`].
#[enum_dispatch]
pub trait Translate {
    /// Translates the shape by a [`Point`] through mutation.
    fn translate(&mut self, p: Point);
}

/// A trait for specifying how a shape is translated by a [`Point`].
pub trait TranslateOwned {
    /// Consumes and translates the shape by a [`Point`], returning the new shape.
    fn translate_owned(self, p: Point) -> Self
    where
        Self: Sized;
}

impl Translate for Point {
    fn translate(&mut self, p: Point) {
        self.x += p.x;
        self.y += p.y;
    }
}

impl TranslateOwned for Point {
    fn translate_owned(mut self, p: Point) -> Self {
        self.x += p.x;
        self.y += p.y;
        self
    }
}

impl Translate for Rect {
    fn translate(&mut self, p: Point) {
        self.p0.translate(p);
        self.p1.translate(p);
    }
}

impl TranslateOwned for Rect {
    fn translate_owned(self, p: Point) -> Self {
        Self::new(self.p0.translate_owned(p), self.p1.translate_owned(p))
    }
}

/// A trait for specifying how a shape is scaled by a [`Point`].
pub trait Scalable {
    /// Scales the shape by a [`Point`], scaling each dimension by its corresponding coordinate.
    fn scale(&mut self, p: Point);
}

impl Scalable for Point {
    fn scale(&mut self, p: Point) {
        self.x *= p.x;
        self.y *= p.y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orientation::Named;

    #[test]
    fn test_matvec() {
        let a = [[1., 2.], [3., 4.]];
        let b = [5., 6.];
        assert_eq!(matvec(&a, &b), [17., 39.]);
    }

    #[test]
    fn test_matmul() {
        let a = [[1., 2.], [3., 4.]];
        let b = [[5., 6.], [7., 8.]];
        assert_eq!(matmul(&a, &b), [[19., 22.], [43., 50.]]);
    }

    #[test]
    fn cascade_identity() {
        for orientation in Named::all_rectangular() {
            let tf = Transformation::with_loc_and_orientation(Point::new(520, 130), orientation);
            let casc = Transformation::cascade(tf, Transformation::identity());
            assert_eq!(
                tf, casc,
                "Cascading with identity produced incorrect transformation for orientation {:?}",
                orientation
            );
        }
    }

    #[test]
    fn transformation_loc_and_orientation_components() {
        let pt = Point::new(8930, 730);
        for orientation in Named::all_rectangular() {
            println!("Testing orientation {:?}", orientation);
            let tf = Transformation::with_loc_and_orientation(pt, orientation);
            assert_eq!(tf.orientation(), orientation.into());
            assert_eq!(tf.offset_point(), pt);
        }
    }

    #[test]
    fn transformation_equivalent_to_loc_and_orientation() {
        for orientation in Named::all_rectangular() {
            println!("Testing orientation {:?}", orientation);
            let tf1 = Transformation::with_loc_and_orientation(Point::new(380, 340), orientation);
            assert_eq!(tf1.orientation(), orientation.into());
            let tf2 =
                Transformation::with_loc_and_orientation(tf1.offset_point(), tf1.orientation());
            assert_eq!(tf1, tf2);
        }
    }
}
