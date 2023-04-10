//! Rectangular ring geometry.
//!
//! May be useful for drawing structures that enclose other structures,
//! such as guard rings.

use array_map::ArrayMap;
use serde::{Deserialize, Serialize};

use super::bbox::BoundBox;
use super::transform::{Translate, TranslateOwned};
use super::{Corner, Dir, Point, Rect, ShapeTrait, Side, Sign, Span};

/// A rectangular ring surrounding an enclosed rectangle.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ring {
    /// Vertical span of top segment.
    topv: Span,
    /// Vertical span of bottom segment.
    botv: Span,
    /// Horizontal span of left segment.
    lefth: Span,
    /// Horizontal span of right segment.
    righth: Span,
}

/// Represents all ways [`Ring`] geometry can be specified.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RingContents {
    /// The ring must fit within the given rectangle.
    Outer(Rect),
    /// The ring must enclose the given rectangle.
    Inner(Rect),
}

impl RingContents {
    pub fn rect(&self) -> Rect {
        match self {
            Self::Outer(r) => *r,
            Self::Inner(r) => *r,
        }
    }

    pub fn is_outer(&self) -> bool {
        matches!(self, Self::Outer(_))
    }
    pub fn is_inner(&self) -> bool {
        matches!(self, Self::Inner(_))
    }
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RingBuilder {
    contents: Option<RingContents>,
    widths: ArrayMap<Side, i64, 4>,
}

impl Ring {
    #[inline]
    pub fn builder() -> RingBuilder {
        RingBuilder::new()
    }

    pub(crate) fn is_valid(&self) -> bool {
        self.topv.start() > self.botv.stop() && self.righth.start() > self.lefth.stop()
    }

    pub fn outer_hspan(&self) -> Span {
        Span::new(self.lefth.start(), self.righth.stop())
    }

    pub fn inner_hspan(&self) -> Span {
        Span::new(self.lefth.stop(), self.righth.start())
    }

    pub fn outer_vspan(&self) -> Span {
        Span::new(self.botv.start(), self.topv.stop())
    }

    pub fn inner_vspan(&self) -> Span {
        Span::new(self.botv.stop(), self.topv.start())
    }

    pub fn outer(&self) -> Rect {
        Rect::from_spans(self.outer_hspan(), self.outer_vspan())
    }

    pub fn inner(&self) -> Rect {
        Rect::from_spans(self.inner_hspan(), self.inner_vspan())
    }

    #[inline]
    pub fn rect(&self, side: Side) -> Rect {
        match side {
            Side::Top => Rect::from_spans(self.outer_hspan(), self.topv),
            Side::Right => Rect::from_spans(self.righth, self.outer_vspan()),
            Side::Bot => Rect::from_spans(self.outer_hspan(), self.botv),
            Side::Left => Rect::from_spans(self.lefth, self.outer_vspan()),
        }
    }

    #[inline]
    pub fn inner_rect(&self, side: Side) -> Rect {
        match side {
            Side::Top => Rect::from_spans(self.inner_hspan(), self.topv),
            Side::Right => Rect::from_spans(self.righth, self.inner_vspan()),
            Side::Bot => Rect::from_spans(self.inner_hspan(), self.botv),
            Side::Left => Rect::from_spans(self.lefth, self.inner_vspan()),
        }
    }

    #[inline]
    pub fn corner(&self, corner: Corner) -> Rect {
        match corner {
            Corner::LowerLeft => Rect::from_spans(self.lefth, self.botv),
            Corner::UpperLeft => Rect::from_spans(self.lefth, self.topv),
            Corner::LowerRight => Rect::from_spans(self.righth, self.botv),
            Corner::UpperRight => Rect::from_spans(self.righth, self.topv),
        }
    }

    #[inline]
    pub fn left(&self) -> Rect {
        self.rect(Side::Left)
    }

    #[inline]
    pub fn right(&self) -> Rect {
        self.rect(Side::Right)
    }

    #[inline]
    pub fn top(&self) -> Rect {
        self.rect(Side::Top)
    }

    #[inline]
    pub fn bot(&self) -> Rect {
        self.rect(Side::Bot)
    }

    #[inline]
    pub fn rects(&self) -> [Rect; 4] {
        [self.top(), self.right(), self.bot(), self.left()]
    }

    /// The [`Rect`]s going in the horizontal direction (ie. the bottom and top rectangles).
    #[inline]
    pub fn hrects(&self) -> [Rect; 2] {
        [self.bot(), self.top()]
    }

    /// The [`Rect`]s going in the vertical direction (ie. the left and right rectangles).
    #[inline]
    pub fn vrects(&self) -> [Rect; 2] {
        [self.left(), self.right()]
    }

    pub fn inner_rects(&self) -> [Rect; 4] {
        [
            self.inner_rect(Side::Top),
            self.inner_rect(Side::Right),
            self.inner_rect(Side::Bot),
            self.inner_rect(Side::Left),
        ]
    }

    pub fn inner_vrects(&self) -> [Rect; 2] {
        [self.inner_rect(Side::Left), self.inner_rect(Side::Right)]
    }

    pub fn inner_hrects(&self) -> [Rect; 2] {
        [self.inner_rect(Side::Bot), self.inner_rect(Side::Top)]
    }

    /// The [`Rect`]s going in the given direction.
    ///
    /// Also see [`Ring::hrects`] and [`Ring::vrects`].
    pub fn dir_rects(&self, dir: Dir) -> [Rect; 2] {
        match dir {
            Dir::Horiz => self.hrects(),
            Dir::Vert => self.vrects(),
        }
    }
}

impl BoundBox for Ring {
    #[inline]
    fn bbox(&self) -> super::bbox::Bbox {
        self.outer().bbox()
    }

    #[inline]
    fn brect(&self) -> Rect {
        self.outer()
    }
}

impl ShapeTrait for Ring {
    fn point0(&self) -> super::Point {
        Point::new(self.lefth.center(), self.topv.center())
    }

    fn orientation(&self) -> super::Dir {
        self.outer().orientation()
    }

    fn contains(&self, pt: Point) -> bool {
        self.rects().into_iter().any(move |r| r.contains(pt))
    }

    /// This function currently panics, since conversion of Rings to Polygons is not possible.
    ///
    /// The implementation of this function may change in the future.
    fn to_poly(&self) -> super::Polygon {
        unimplemented!("cannot convert Ring to Polygon")
    }
}

impl Translate for Ring {
    fn translate(&mut self, p: Point) {
        *self = self.translate_owned(p);
    }
}

impl TranslateOwned for Ring {
    fn translate_owned(self, p: Point) -> Self {
        Self {
            lefth: self.lefth.translate(p.x),
            righth: self.righth.translate(p.x),
            topv: self.topv.translate(p.y),
            botv: self.botv.translate(p.y),
        }
    }
}

impl From<RingBuilder> for Ring {
    fn from(value: RingBuilder) -> Self {
        let contents = value.contents.unwrap();
        let r = contents.rect();

        let sign = if contents.is_outer() {
            Sign::Pos
        } else {
            Sign::Neg
        };

        let topv = Span::with_point_and_length(sign, r.top(), value.widths[Side::Top]);
        let righth = Span::with_point_and_length(sign, r.right(), value.widths[Side::Right]);
        let lefth = Span::with_point_and_length(!sign, r.left(), value.widths[Side::Left]);
        let botv = Span::with_point_and_length(!sign, r.bottom(), value.widths[Side::Bot]);

        let res = Self {
            topv,
            botv,
            lefth,
            righth,
        };

        if contents.is_outer() {
            assert_eq!(res.outer(), r);
        } else {
            assert_eq!(res.inner(), r);
        }

        assert!(res.is_valid());
        res
    }
}

impl RingBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn build(&mut self) -> Ring {
        Ring::from(*self)
    }

    pub fn outer(&mut self, rect: Rect) -> &mut Self {
        self.contents = Some(RingContents::Outer(rect));
        self
    }

    pub fn inner(&mut self, rect: Rect) -> &mut Self {
        self.contents = Some(RingContents::Inner(rect));
        self
    }

    pub fn left_width(&mut self, value: i64) -> &mut Self {
        self.widths[Side::Left] = value;
        self
    }

    pub fn right_width(&mut self, value: i64) -> &mut Self {
        self.widths[Side::Right] = value;
        self
    }

    pub fn bot_height(&mut self, value: i64) -> &mut Self {
        self.widths[Side::Bot] = value;
        self
    }

    pub fn top_height(&mut self, value: i64) -> &mut Self {
        self.widths[Side::Top] = value;
        self
    }

    /// Sets the widths of the vertical-going parts of the ring to the given value.
    pub fn widths(&mut self, value: i64) -> &mut Self {
        self.left_width(value);
        self.right_width(value)
    }

    /// Sets the heights of the horizontal-going parts of the ring to the given value.
    pub fn heights(&mut self, value: i64) -> &mut Self {
        self.top_height(value);
        self.bot_height(value)
    }

    /// Sets the width of all ring edges to the given value.
    pub fn uniform_width(&mut self, value: i64) -> &mut Self {
        self.widths(value);
        self.heights(value)
    }

    pub fn dir_widths(&mut self, dir: Dir, value: i64) -> &mut Self {
        match dir {
            Dir::Vert => self.widths(value),
            Dir::Horiz => self.heights(value),
        }
    }

    pub fn side_width(&mut self, side: Side, value: i64) -> &mut Self {
        use Side::*;
        match side {
            Top => self.top_height(value),
            Bot => self.bot_height(value),
            Left => self.left_width(value),
            Right => self.right_width(value),
        }
    }
}
