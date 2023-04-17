//! Core geometric types and their operations/attributes.

use std::cmp::Ordering;
use std::convert::TryFrom;
use std::fmt::Display;
use std::str::FromStr;

use array_map::{ArrayMap, Indexable};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use transform::{Scalable, Transform, Transformation, Translate};

use self::bbox::{Bbox, BoundBox};
use self::trim::Trim;

pub mod bbox;
pub mod orientation;
pub mod ring;
pub mod transform;
pub mod trim;

/// Snaps `pos` to the nearest multiple of `grid`.
pub fn snap_to_grid(pos: i64, grid: i64) -> i64 {
    assert!(grid > 0);

    let rem = pos.rem_euclid(grid);
    assert!(rem >= 0);
    assert!(rem < grid);
    if rem <= grid / 2 {
        pos - rem
    } else {
        pos + grid - rem
    }
}

/// A point in two-dimensional layout-space.
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    /// Creates a new [`Point`] from (x,y) coordinates.
    pub fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// Creates a new point from the given direction and coordinates.
    ///
    /// If `dir` is [`Dir::Horiz`], `a` becomes the x-coordinate and `b` becomes the y-coordinate.
    /// If `dir` is [`Dir::Vert`], `a` becomes the y-coordinate and `b` becomes the x-coordinate.
    pub fn from_dir_coords(dir: Dir, a: i64, b: i64) -> Self {
        match dir {
            Dir::Horiz => Self::new(a, b),
            Dir::Vert => Self::new(b, a),
        }
    }

    /// Returns the origin, (0, 0).
    #[inline]
    pub fn zero() -> Self {
        Self { x: 0, y: 0 }
    }
    /// Creates a new [`Point`] that serves as an offset in direction `dir`.
    pub fn offset(val: i64, dir: Dir) -> Self {
        match dir {
            Dir::Horiz => Self { x: val, y: 0 },
            Dir::Vert => Self { x: 0, y: val },
        }
    }
    /// Gets the coordinate associated with direction `dir`.
    pub fn coord(&self, dir: Dir) -> i64 {
        match dir {
            Dir::Horiz => self.x,
            Dir::Vert => self.y,
        }
    }
    /// Creates a new [`Point`] shifted by `x` in the x-dimension and by `y` in the y-dimension.
    #[inline]
    pub fn translated(&self, p: Point) -> Self {
        let mut pt = *self;
        pt.translate(p);
        pt
    }
    /// Creates a new point scaled by `p.x` in the x-dimension and by `p.y` in the y-dimension.
    #[inline]
    pub fn scaled(&self, p: Point) -> Self {
        let mut pt = *self;
        pt.scale(p);
        pt
    }

    #[inline]
    pub fn snap_to_grid(&self, grid: i64) -> Self {
        self.snap_x_to_grid(grid).snap_y_to_grid(grid)
    }

    #[inline]
    pub fn snap_x_to_grid(&self, grid: i64) -> Self {
        let x = snap_to_grid(self.x, grid);
        Self { x, y: self.y }
    }

    #[inline]
    pub fn snap_y_to_grid(&self, grid: i64) -> Self {
        let y = snap_to_grid(self.y, grid);
        Self { x: self.x, y }
    }
}

impl Trim<Rect> for Point {
    type Output = Self;
    fn trim(&self, bounds: &Rect) -> Option<Self::Output> {
        if bounds.contains(*self) {
            Some(*self)
        } else {
            None
        }
    }
}

impl std::ops::Add<Point> for Point {
    type Output = Self;
    fn add(self, rhs: Point) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Add<Dims> for Point {
    type Output = Self;
    fn add(self, rhs: Dims) -> Self::Output {
        Self::new(self.x + rhs.w, self.y + rhs.h)
    }
}

impl std::ops::AddAssign<Point> for Point {
    fn add_assign(&mut self, rhs: Point) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::AddAssign<Dims> for Point {
    fn add_assign(&mut self, rhs: Dims) {
        self.x += rhs.w;
        self.y += rhs.h;
    }
}

impl std::ops::Sub<Point> for Point {
    type Output = Self;
    fn sub(self, rhs: Point) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::SubAssign<Point> for Point {
    fn sub_assign(&mut self, rhs: Point) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl From<(i64, i64)> for Point {
    fn from(value: (i64, i64)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

/// A one-dimensional span.
#[derive(
    Debug, Default, Clone, Copy, Hash, Ord, PartialOrd, Serialize, Deserialize, PartialEq, Eq,
)]
pub struct Span {
    start: i64,
    stop: i64,
}

impl Span {
    /// Creates a new [`Span`] from 0 until the specified stop.
    ///
    /// # Panics
    ///
    /// This function panics if `stop` is less than 0.
    pub fn until(stop: i64) -> Self {
        debug_assert!(stop >= 0);
        Self { start: 0, stop }
    }

    /// Creates a new [`Span`] between two integers.
    ///
    /// The caller must ensure that `start` is less
    /// than or equal to `stop`.
    pub const fn new_unchecked(start: i64, stop: i64) -> Self {
        Self { start, stop }
    }

    /// Creates a new [`Span`] between two integers.
    pub fn new(start: i64, stop: i64) -> Self {
        use std::cmp::{max, min};
        let lower = min(start, stop);
        let upper = max(start, stop);
        Self {
            start: lower,
            stop: upper,
        }
    }

    /// Creates a span of zero length encompassing the given point.
    pub fn from_point(x: i64) -> Self {
        Self { start: x, stop: x }
    }

    pub fn with_start_and_length(start: i64, length: i64) -> Self {
        Self {
            stop: start + length,
            start,
        }
    }

    pub fn with_stop_and_length(stop: i64, length: i64) -> Self {
        Self {
            start: stop - length,
            stop,
        }
    }

    /// Creates a span with the given endpoint and length.
    ///
    /// If `sign` is [`Sign::Pos`], `point` is treated as the ending/stopping point of the span.
    /// If `sign` is [`Sign::Neg`], `point` is treated as the beginning/starting point of the span.
    pub fn with_point_and_length(sign: Sign, point: i64, length: i64) -> Self {
        match sign {
            Sign::Pos => Self::with_stop_and_length(point, length),
            Sign::Neg => Self::with_start_and_length(point, length),
        }
    }

    /// Creates a new [`Span`] expanded by `amount` in the direction indicated by `pos`.
    pub fn expand(mut self, pos: bool, amount: i64) -> Self {
        if pos {
            self.stop += amount;
        } else {
            self.start -= amount;
        }
        self
    }

    /// Creates a new [`Span`] expanded by `amount` in both directions.
    pub fn expand_all(mut self, amount: i64) -> Self {
        self.stop += amount;
        self.start -= amount;
        self
    }

    /// Gets the starting ([`Sign::Neg`]) or stopping ([`Sign::Pos`]) point of a span.
    #[inline]
    pub fn point(&self, sign: Sign) -> i64 {
        match sign {
            Sign::Neg => self.start(),
            Sign::Pos => self.stop(),
        }
    }

    /// Gets the shortest distance to a point.
    pub fn distance_to(&self, point: i64) -> i64 {
        std::cmp::min((point - self.start()).abs(), (point - self.stop()).abs())
    }

    /// Creates a new [`Span`] with center `center` and length `span`.
    pub fn from_center_span(center: i64, span: i64) -> Self {
        assert!(span >= 0);
        assert_eq!(span % 2, 0);

        Self::new(center - (span / 2), center + (span / 2))
    }

    /// Creates a new [`Span`] with center `center` and length `span` and snap the edges to the
    /// grid.
    pub fn from_center_span_gridded(center: i64, span: i64, grid: i64) -> Self {
        assert!(span >= 0);
        assert_eq!(span % 2, 0);
        assert_eq!(span % grid, 0);

        let start = snap_to_grid(center - (span / 2), grid);

        Self::new(start, start + span)
    }

    /// Gets the center of the span.
    #[inline]
    pub fn center(&self) -> i64 {
        (self.start + self.stop) / 2
    }

    /// Gets the length of the span.
    #[inline]
    pub fn length(&self) -> i64 {
        self.stop - self.start
    }

    /// Gets the start of the span.
    #[inline]
    pub fn start(&self) -> i64 {
        self.start
    }

    /// Gets the stop of the span.
    #[inline]
    pub fn stop(&self) -> i64 {
        self.stop
    }

    /// Checks if the span intersects with the [`Span`] `other`.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        !(other.stop < self.start || self.stop < other.start)
    }

    /// Creates a new minimal [`Span`] that contains all of the elements of `spans`.
    pub fn merge(spans: impl IntoIterator<Item = Self>) -> Self {
        use std::cmp::{max, min};
        let mut spans = spans.into_iter();
        let (mut start, mut stop) = spans
            .next()
            .expect("Span::merge requires at least one span")
            .into();

        for span in spans {
            start = min(start, span.start);
            stop = max(stop, span.stop);
        }

        debug_assert!(start <= stop);

        Span { start, stop }
    }

    /// Merges adjacent spans when `merge_fn` evaluates to true.
    pub fn merge_adjacent(
        spans: impl IntoIterator<Item = Self>,
        mut merge_fn: impl FnMut(Span, Span) -> bool,
    ) -> impl Iterator<Item = Span> {
        let mut spans: Vec<Span> = spans.into_iter().collect();
        spans.sort_by_key(|span| span.start());

        let mut merged_spans = Vec::new();

        let mut j = 0;
        while j < spans.len() {
            let mut curr_span = spans[j];
            j += 1;
            while j < spans.len() && merge_fn(curr_span, spans[j]) {
                curr_span = curr_span.union(spans[j]);
                j += 1;
            }
            merged_spans.push(curr_span);
        }

        merged_spans.into_iter()
    }

    pub fn union(self, other: Self) -> Self {
        use std::cmp::{max, min};
        Self {
            start: min(self.start, other.start),
            stop: max(self.stop, other.stop),
        }
    }

    pub fn contains(self, other: Self) -> bool {
        self.union(other) == self
    }

    /// Returns a new [`Span`] representing the union of the current span with the given point.
    pub fn add_point(self, pos: i64) -> Self {
        use std::cmp::{max, min};
        Self {
            start: min(self.start, pos),
            stop: max(self.stop, pos),
        }
    }

    /// Shrinks the given side by the given amount.
    ///
    /// Behavior is controlled by the given [`Sign`]:
    /// * If `side` is [`Sign::Pos`], shrinks from the positive end (ie. decreases the `stop`).
    /// * If `side` is [`Sign::Neg`], shrinks from the negative end (ie. increases the `start`).
    pub fn shrink(self, side: Sign, amount: i64) -> Self {
        assert!(self.length() >= amount);
        match side {
            Sign::Pos => Self::new(self.start, self.stop - amount),
            Sign::Neg => Self::new(self.start + amount, self.stop),
        }
    }

    pub fn shrink_all(self, amount: i64) -> Self {
        assert!(self.length() >= 2 * amount);
        Self {
            start: self.start + amount,
            stop: self.stop - amount,
        }
    }

    pub fn translate(self, amount: i64) -> Self {
        Self {
            start: self.start + amount,
            stop: self.stop + amount,
        }
    }

    pub fn min_distance(self, other: Span) -> i64 {
        std::cmp::max(
            0,
            self.union(other).length() - self.length() - other.length(),
        )
    }
}

impl From<(i64, i64)> for Span {
    #[inline]
    fn from(tup: (i64, i64)) -> Self {
        Self::new(tup.0, tup.1)
    }
}

impl From<Span> for (i64, i64) {
    #[inline]
    fn from(s: Span) -> Self {
        (s.start(), s.stop())
    }
}

/// An enumeration of axis-aligned directions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum Dir {
    /// The horizontal, or x-aligned, direction.
    Horiz,
    /// The vertical, or y-aligned, direction.
    Vert,
}

#[derive(Debug, Clone, Eq, PartialEq, Error)]
#[error("error parsing direction `{original}`; expected horizontal or vertical")]
pub struct DirParseError {
    original: String,
}

impl FromStr for Dir {
    type Err = DirParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowercase = s.to_lowercase();
        match lowercase.trim() {
            "vertical" | "vert" | "v" => Ok(Self::Vert),
            "horizontal" | "horiz" | "h" => Ok(Self::Horiz),
            _ => Err(DirParseError {
                original: s.to_string(),
            }),
        }
    }
}

/// Enumeration over possible signs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[repr(u8)]
#[derive(Indexable)]
pub enum Sign {
    /// Positive.
    Pos,
    /// Negative.
    Neg,
}

impl Sign {
    #[inline]
    pub fn as_int(&self) -> i64 {
        match self {
            Self::Pos => 1,
            Self::Neg => -1,
        }
    }
}

impl std::ops::Not for Sign {
    type Output = Self;
    /// Flips the [`Sign`].
    fn not(self) -> Self::Output {
        match self {
            Self::Pos => Self::Neg,
            Self::Neg => Self::Pos,
        }
    }
}

impl Dir {
    /// Returns the perpendicular direction.
    pub fn other(self) -> Self {
        match self {
            Self::Horiz => Self::Vert,
            Self::Vert => Self::Horiz,
        }
    }
    /// Returns the direction as a string.
    pub fn short_form(&self) -> &'static str {
        match *self {
            Self::Horiz => "h",
            Self::Vert => "v",
        }
    }
}

impl Default for Dir {
    #[inline]
    fn default() -> Self {
        Self::Horiz
    }
}

impl Display for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Horiz => write!(f, "horizontal"),
            Self::Vert => write!(f, "vertical"),
        }
    }
}

impl std::ops::Not for Dir {
    type Output = Self;
    /// Exclamation Operator returns the opposite direction
    fn not(self) -> Self::Output {
        self.other()
    }
}

/// An enumeration of the sides of a axis-aligned rectangle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[repr(u8)]
#[derive(Indexable)]
pub enum Side {
    Top,
    Right,
    Bot,
    Left,
}

impl Side {
    /// Gets the direction of the coordinate corresponding to this side.
    ///
    /// Top and bottom edges are y-coordinates, so they are on the **vertical** axis.
    /// Left and right edges are x-coordinates, so they are on the **horizontal** axis.
    ///
    /// Also see [`Side::edge_dir`].
    pub fn coord_dir(&self) -> Dir {
        use Dir::*;
        use Side::*;
        match self {
            Top | Bot => Vert,
            Left | Right => Horiz,
        }
    }

    /// Gets the direction of the edge corresponding to this side.
    ///
    /// Top and bottom edges are **horizontal** line segments;
    /// left and right edges are **vertical** line segments.
    ///
    /// Also see [`Side::coord_dir`].
    pub fn edge_dir(&self) -> Dir {
        use Dir::*;
        use Side::*;
        match self {
            Top | Bot => Horiz,
            Left | Right => Vert,
        }
    }

    /// Returns the opposite direction.
    pub fn other(&self) -> Self {
        match self {
            Side::Top => Side::Bot,
            Side::Right => Side::Left,
            Side::Bot => Side::Top,
            Side::Left => Side::Right,
        }
    }

    /// Returns the sign corresponding to moving towards this side.
    pub fn sign(&self) -> Sign {
        use Side::*;
        use Sign::*;
        match self {
            Top | Right => Pos,
            Bot | Left => Neg,
        }
    }

    /// Returns the side corresponding with the given [`Dir`] and [`Sign`].
    pub fn with_dir_and_sign(dir: Dir, sign: Sign) -> Side {
        match dir {
            Dir::Horiz => match sign {
                Sign::Pos => Side::Right,
                Sign::Neg => Side::Left,
            },
            Dir::Vert => match sign {
                Sign::Pos => Side::Top,
                Sign::Neg => Side::Bot,
            },
        }
    }

    /// Returns sides that bound the given direction.
    pub fn with_dir(dir: Dir) -> impl Iterator<Item = Side> {
        match dir {
            Dir::Horiz => [Side::Left, Side::Right].into_iter(),
            Dir::Vert => [Side::Bot, Side::Top].into_iter(),
        }
    }
}

impl std::ops::Not for Side {
    type Output = Self;
    /// Exclamation Operator returns the opposite direction
    fn not(self) -> Self::Output {
        self.other()
    }
}

/// An association of a value with type `T` to each of the four [`Side`]s.
#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
pub struct Sides<T> {
    inner: ArrayMap<Side, T, 4>,
}

impl<T> Sides<T>
where
    T: Clone,
{
    /// Creates a new [`Sides`] with `value` associated with all sides.
    ///
    /// The value will be cloned for each [`Side`].
    ///
    /// If your value is [`Copy`], consider using [`Sides::uniform`] instead.
    pub fn uniform_cloned(value: T) -> Self {
        Self {
            inner: ArrayMap::from_value(value),
        }
    }
}

impl<T> Sides<T>
where
    T: Copy,
{
    /// Creates a new [`Sides`] with `value` associated with all sides.
    pub const fn uniform(value: T) -> Self {
        Self {
            inner: ArrayMap::new([value; 4]),
        }
    }
}

impl<T> Sides<T> {
    /// Creates a new [`Sides`] with with the provided values for each side.
    pub const fn new(top: T, right: T, bot: T, left: T) -> Self {
        // IMPORTANT: the ordering of array elements here must match
        // the ordering of variants in the [`Side`] enum.
        Self {
            inner: ArrayMap::new([top, right, bot, left]),
        }
    }

    /// Maps a function over the provided [`Sides`], returning a new [`Sides`].
    pub fn map<B>(self, f: impl FnMut(&Side, T) -> B) -> Sides<B> {
        Sides {
            inner: self.inner.map(f),
        }
    }
}

impl<T> std::ops::Index<Side> for Sides<T> {
    type Output = T;
    fn index(&self, index: Side) -> &Self::Output {
        self.inner.index(index)
    }
}

impl<T> std::ops::IndexMut<Side> for Sides<T> {
    fn index_mut(&mut self, index: Side) -> &mut Self::Output {
        self.inner.index_mut(index)
    }
}

/// An edge of a rectangle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct Edge {
    /// The side of the rectangle this edge corresponds to.
    side: Side,
    /// The coordinate of the edge.
    coord: i64,
    /// The perpendicular span of the edge.
    span: Span,
}

impl Edge {
    /// Create a new edge.
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// ```
    pub fn new(side: Side, coord: i64, span: Span) -> Self {
        Self { side, coord, span }
    }

    /// The side (of a rectangle) to which this edge corresponds.
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.side(), Side::Left);
    /// ```
    pub fn side(&self) -> Side {
        self.side
    }

    /// The coordinate of the edge.
    ///
    /// For left/right edges, this will be the x coordinate of the edge.
    /// For top/bottom edges, this will be the y coordinate of the edge.
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.coord(), 20);
    /// ```
    pub fn coord(&self) -> i64 {
        self.coord
    }

    /// The span of the edge.
    ///
    /// For left/right edges, this will be the range of y-coordinates encompassed by the edge.
    /// For top/bottom edges, this will be the range of x-coordinates encompassed by the edge.
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.span(), Span::new(40, 100));
    /// ```
    pub fn span(&self) -> Span {
        self.span
    }

    /// Returns an `Edge` with the same properties as the provided `Edge` but with a new span.
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.span(), Span::new(40, 100));
    /// let edge_new = edge.with_span(Span::new(20, 100));
    /// assert_eq!(edge_new.span(), Span::new(20, 100));
    /// ```
    pub fn with_span(&self, span: Span) -> Edge {
        Edge { span, ..*self }
    }

    /// The direction perpendicular to the edge.
    ///
    /// For left/right edges, this will be [`Dir::Horiz`].
    /// For top/bottom edges, this will be [`Dir::Vert`].
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.norm_dir(), Dir::Horiz);
    /// let edge = Edge::new(Side::Right, 20, Span::new(40, 100));
    /// assert_eq!(edge.norm_dir(), Dir::Horiz);
    /// let edge = Edge::new(Side::Top, 20, Span::new(40, 100));
    /// assert_eq!(edge.norm_dir(), Dir::Vert);
    /// let edge = Edge::new(Side::Bot, 20, Span::new(40, 100));
    /// assert_eq!(edge.norm_dir(), Dir::Vert);
    /// ```
    pub fn norm_dir(&self) -> Dir {
        self.side.coord_dir()
    }

    /// The direction parallel to the edge.
    ///
    /// For left/right edges, this will be [`Dir::Vert`].
    /// For top/bottom edges, this will be [`Dir::Horiz`].
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.edge_dir(), Dir::Vert);
    /// let edge = Edge::new(Side::Right, 20, Span::new(40, 100));
    /// assert_eq!(edge.edge_dir(), Dir::Vert);
    /// let edge = Edge::new(Side::Top, 20, Span::new(40, 100));
    /// assert_eq!(edge.edge_dir(), Dir::Horiz);
    /// let edge = Edge::new(Side::Bot, 20, Span::new(40, 100));
    /// assert_eq!(edge.edge_dir(), Dir::Horiz);
    /// ```
    pub fn edge_dir(&self) -> Dir {
        self.side.edge_dir()
    }

    /// Returns a new [`Edge`] offset some amount **away** from this edge.
    ///
    /// Left edges will be offset to the left; right edges will be offset to the right.
    /// Top edges will be offset upwards; bottom edges will be offset downwards.
    ///
    /// # Example
    ///
    /// ```
    /// # use subgeom::*;
    /// let edge = Edge::new(Side::Left, 20, Span::new(40, 100));
    /// assert_eq!(edge.offset(10), Edge::new(Side::Left, 10, Span::new(40, 100)));
    ///
    /// let edge = Edge::new(Side::Top, 20, Span::new(40, 100));
    /// assert_eq!(edge.offset(10), Edge::new(Side::Top, 30, Span::new(40, 100)));
    /// ```
    pub fn offset(&self, offset: i64) -> Edge {
        Edge {
            coord: self.coord + self.side.sign().as_int() * offset,
            ..*self
        }
    }
}

/// An enumeration of the corners of an axis-aligned rectangle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[repr(u8)]
#[derive(Indexable)]
pub enum Corner {
    /// The lower-left corner.
    LowerLeft,
    /// The lower-right corner.
    LowerRight,
    /// The upper-left corner.
    UpperLeft,
    /// The upper-right corner.
    UpperRight,
}

impl Corner {
    pub fn side(&self, dir: Dir) -> Side {
        use Corner::*;
        use Dir::*;
        use Side::*;
        match dir {
            Horiz => match self {
                LowerLeft | UpperLeft => Left,
                LowerRight | UpperRight => Right,
            },
            Vert => match self {
                LowerLeft | LowerRight => Bot,
                UpperLeft | UpperRight => Top,
            },
        }
    }
}

/// An open-ended geometric path with non-zero width.
///
/// Primarily consists of a series of ordered [`Point`]s.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Path {
    pub points: Vec<Point>,
    pub width: usize,
}
impl Translate for Path {
    fn translate(&mut self, p: Point) {
        for pt in self.points.iter_mut() {
            pt.translate(p);
        }
    }
}
/// A closed n-sided polygon with arbitrary number of vertices.
///
/// Closure from the last point back to the first is implied;
/// the initial point need not be repeated at the end.
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Polygon {
    pub points: Vec<Point>,
}
impl Translate for Polygon {
    fn translate(&mut self, p: Point) {
        for pt in self.points.iter_mut() {
            pt.translate(p);
        }
    }
}

/// An axis-aligned rectangle, specified by lower-left and upper-right corners.
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rect {
    /// The lower-left corner.
    pub p0: Point,
    /// The upper-right corner.
    pub p1: Point,
}
impl Rect {
    /// Creates a rectangle with points `(0, 0), (dims.w(), dims.h())`.
    ///
    /// The caller should ensure that `dims.w()` and `dims.h()` are non-negative.
    /// See [`Dims`] for more information.
    pub fn with_dims(dims: Dims) -> Self {
        Self::new(Point::zero(), Point::new(dims.w(), dims.h()))
    }

    /// Returns the center point of the rectangle.
    pub fn center(&self) -> Point {
        Point::new((self.p0.x + self.p1.x) / 2, (self.p0.y + self.p1.y) / 2)
    }

    /// Creates an empty rectangle containing the given point.
    pub fn from_point(p: Point) -> Self {
        Self { p0: p, p1: p }
    }

    /// Creates an empty rectangle containing the given `(x, y)` coordinates.
    pub fn from_xy(x: i64, y: i64) -> Self {
        let p = Point::new(x, y);
        Self::from_point(p)
    }

    /// Creates a new rectangle.
    pub fn new(p0: Point, p1: Point) -> Self {
        Self {
            p0: Point::new(p0.x.min(p1.x), p0.y.min(p1.y)),
            p1: Point::new(p0.x.max(p1.x), p0.y.max(p1.y)),
        }
    }

    /// Creates a rectangle from horizontal and vertical [`Span`]s.
    pub fn from_spans(h: Span, v: Span) -> Self {
        Self {
            p0: Point::new(h.start(), v.start()),
            p1: Point::new(h.stop(), v.stop()),
        }
    }

    /// Returns the bottom y-coordinate of the rectangle.
    #[inline]
    pub fn bottom(&self) -> i64 {
        self.p0.y
    }

    /// Returns the top y-coordinate of the rectangle.
    #[inline]
    pub fn top(&self) -> i64 {
        self.p1.y
    }

    /// Returns the left x-coordinate of the rectangle.
    #[inline]
    pub fn left(&self) -> i64 {
        self.p0.x
    }

    /// Returns the right x-coordinate of the rectangle.
    #[inline]
    pub fn right(&self) -> i64 {
        self.p1.x
    }

    /// Returns the horizontal span of the rectangle.
    pub fn hspan(&self) -> Span {
        Span::new(self.p0.x, self.p1.x)
    }

    /// Returns a [`Rect`] with the given `hspan` and the same vertical span.
    pub fn with_hspan(self, hspan: Span) -> Self {
        Rect::new(
            Point::new(hspan.start(), self.p0.y),
            Point::new(hspan.stop(), self.p1.y),
        )
    }

    /// Returns a [`Rect`] with the given `vspan` and the same horizontal span.
    pub fn with_vspan(self, vspan: Span) -> Self {
        Rect::new(
            Point::new(self.p0.x, vspan.start()),
            Point::new(self.p1.x, vspan.stop()),
        )
    }

    /// Returns a [`Rect`] with the given `span` in the given `dir`, and the current span in the
    /// other direction.
    pub fn with_span(self, span: Span, dir: Dir) -> Self {
        match dir {
            Dir::Vert => self.with_vspan(span),
            Dir::Horiz => self.with_hspan(span),
        }
    }

    /// Returns the vertical span of the rectangle.
    pub fn vspan(&self) -> Span {
        Span::new(self.p0.y, self.p1.y)
    }

    /// Returns the horizontal width of the rectangle.
    #[inline]
    pub fn width(&self) -> i64 {
        self.hspan().length()
    }

    /// Returns the vertical height of the rectangle.
    #[inline]
    pub fn height(&self) -> i64 {
        self.vspan().length()
    }

    /// Returns the area of the rectangle.
    #[inline]
    pub fn area(&self) -> i64 {
        self.width() * self.height()
    }

    /// Returns the lower edge of the rectangle in the [`Dir`] `dir.
    pub fn lower_edge(&self, dir: Dir) -> i64 {
        self.span(dir).start()
    }

    /// Returns the upper edge of the rectangle in the [`Dir`] `dir.
    pub fn upper_edge(&self, dir: Dir) -> i64 {
        self.span(dir).stop()
    }

    /// Returns the span of the rectangle in the [`Dir`] `dir.
    pub fn span(&self, dir: Dir) -> Span {
        match dir {
            Dir::Horiz => self.hspan(),
            Dir::Vert => self.vspan(),
        }
    }

    /// Returns the edges of two rectangles along the [`Dir`] `dir` in increasing order.
    fn sorted_edges(&self, other: &Self, dir: Dir) -> [i64; 4] {
        let mut edges = [
            self.lower_edge(dir),
            self.upper_edge(dir),
            other.lower_edge(dir),
            other.upper_edge(dir),
        ];
        edges.sort();
        edges
    }

    /// Returns the inner two edges of two rectangles along the [`Dir`] `dir` in increasing order.
    #[inline]
    pub fn inner_span(&self, other: &Self, dir: Dir) -> Span {
        let edges = self.sorted_edges(other, dir);
        Span::new(edges[1], edges[2])
    }

    /// Returns the outer two edges of two rectangles along the [`Dir`] `dir` in increasing order.
    #[inline]
    pub fn outer_span(&self, other: &Self, dir: Dir) -> Span {
        let edges = self.sorted_edges(other, dir);
        Span::new(edges[0], edges[3])
    }

    /// Returns the edge of a rectangle closest to the coordinate `x` along a given direction.
    pub fn edge_closer_to(&self, x: i64, dir: Dir) -> i64 {
        let (x0, x1) = self.span(dir).into();
        if (x - x0).abs() <= (x - x1).abs() {
            x0
        } else {
            x1
        }
    }

    /// Returns the edge of a rectangle farthest from the coordinate `x` along a given direction.
    pub fn edge_farther_from(&self, x: i64, dir: Dir) -> i64 {
        let (x0, x1) = self.span(dir).into();
        if (x - x0).abs() <= (x - x1).abs() {
            x1
        } else {
            x0
        }
    }

    /// Returns a builder for creating a rectangle from [`Span`]s.
    #[inline]
    pub fn span_builder() -> RectSpanBuilder {
        RectSpanBuilder::new()
    }

    /// Returns the length of this rectangle in the given direction.
    pub fn length(&self, dir: Dir) -> i64 {
        self.span(dir).length()
    }

    /// Returns the direction in which the rectangle is longer, choosing [`Dir::Vert`] if the sides
    /// are equal.
    #[inline]
    pub fn longer_dir(&self) -> Dir {
        if self.width() > self.height() {
            Dir::Horiz
        } else {
            Dir::Vert
        }
    }

    /// Returns the direction in which the rectangle is longer, choosing [`Dir::Vert`] if the sides
    /// are equal.
    #[inline]
    pub fn shorter_dir(&self) -> Dir {
        !self.longer_dir()
    }

    /// Expands the rectangle by `amount` on all sides.
    #[inline]
    pub fn expand(&self, amount: i64) -> Self {
        Self::new(
            Point::new(self.p0.x - amount, self.p0.y - amount),
            Point::new(self.p1.x + amount, self.p1.y + amount),
        )
    }

    /// Shrinks the rectangle by `amount` on all sides.
    #[inline]
    pub fn shrink(&self, amount: i64) -> Self {
        assert!(2 * amount <= self.width());
        assert!(2 * amount <= self.height());
        Self::new(
            Point::new(self.p0.x + amount, self.p0.y + amount),
            Point::new(self.p1.x - amount, self.p1.y - amount),
        )
    }

    /// Expands the rectangle by `amount` on both sides associated with the direction `dir`.
    #[inline]
    pub fn expand_dir(&self, dir: Dir, amount: i64) -> Self {
        match dir {
            Dir::Horiz => Self::new(
                Point::new(self.p0.x - amount, self.p0.y),
                Point::new(self.p1.x + amount, self.p1.y),
            ),
            Dir::Vert => Self::new(
                Point::new(self.p0.x, self.p0.y - amount),
                Point::new(self.p1.x, self.p1.y + amount),
            ),
        }
    }

    /// Expands the rectangle by `amount` on the given side.
    #[inline]
    pub fn expand_side(&self, side: Side, amount: i64) -> Self {
        match side {
            Side::Top => Self::new(
                Point::new(self.p0.x, self.p0.y),
                Point::new(self.p1.x, self.p1.y + amount),
            ),
            Side::Bot => Self::new(
                Point::new(self.p0.x, self.p0.y - amount),
                Point::new(self.p1.x, self.p1.y),
            ),
            Side::Right => Self::new(
                Point::new(self.p0.x, self.p0.y),
                Point::new(self.p1.x + amount, self.p1.y),
            ),
            Side::Left => Self::new(
                Point::new(self.p0.x - amount, self.p0.y),
                Point::new(self.p1.x, self.p1.y),
            ),
        }
    }

    /// Expands this rectangle by the given dimensions.
    ///
    /// The exact behavior depends on the provided [`ExpandMode`]:
    /// * [`ExpandMode::All`]: expands the top and bottom edges by `dims.h()`
    /// and the left and right edges by `dims.w()`.
    /// Note that the total horizontal expansion is `2 * dims.w()` and the
    /// total vertical expansion is `2 * dims.h()`.
    /// * [`ExpandMode::LowerLeft`]: expands the lower edge by `dims.h()` and
    /// the left edge by `dims.w()`.
    /// * [`ExpandMode::LowerRight`]: expands the lower edge by `dims.h()` and
    /// the right edge by `dims.w()`.
    /// * [`ExpandMode::UpperLeft`]: expands the upper edge by `dims.h()` and
    /// the left edge by `dims.w()`.
    /// * [`ExpandMode::UpperRight`]: expands the upper edge by `dims.h()` and
    /// the right edge by `dims.w()`.
    ///
    /// See [`Dims`] for more information.
    pub fn expand_dims(self, dims: Dims, mode: ExpandMode) -> Self {
        use ExpandMode::*;
        let left = match mode {
            All | LowerLeft | UpperLeft => self.p0.x - dims.w(),
            _ => self.p0.x,
        };
        let bot = match mode {
            All | LowerLeft | LowerRight => self.p0.y - dims.h(),
            _ => self.p0.y,
        };
        let right = match mode {
            All | LowerRight | UpperRight => self.p1.x + dims.w(),
            _ => self.p1.x,
        };
        let top = match mode {
            All | UpperLeft | UpperRight => self.p1.y + dims.h(),
            _ => self.p1.y,
        };

        Self::new(Point::new(left, bot), Point::new(right, top))
    }

    /// Returns the dimensions of the rectangle as [`Dims`].
    #[inline]
    pub fn dims(&self) -> Dims {
        Dims::new(self.width(), self.height())
    }

    /// Returns the desired corner of the rectangle.
    pub fn corner(&self, corner: Corner) -> Point {
        match corner {
            Corner::LowerLeft => self.p0,
            Corner::LowerRight => Point::new(self.p1.x, self.p0.y),
            Corner::UpperLeft => Point::new(self.p0.x, self.p1.y),
            Corner::UpperRight => self.p1,
        }
    }

    /// Grows this rectangle by a factor of 2 on the given [`Side`].
    ///
    /// Sometimes useful for half-track geometry.
    pub fn double(self, side: Side) -> Self {
        match side {
            Side::Top => Self::from_spans(
                self.hspan(),
                Span::with_start_and_length(self.bottom(), 2 * self.height()),
            ),
            Side::Bot => Self::from_spans(
                self.hspan(),
                Span::with_stop_and_length(self.top(), 2 * self.height()),
            ),
            Side::Left => Self::from_spans(
                Span::with_stop_and_length(self.right(), 2 * self.width()),
                self.vspan(),
            ),
            Side::Right => Self::from_spans(
                Span::with_start_and_length(self.left(), 2 * self.width()),
                self.vspan(),
            ),
        }
    }

    #[inline]
    pub fn side(&self, side: Side) -> i64 {
        match side {
            Side::Top => self.top(),
            Side::Bot => self.bottom(),
            Side::Right => self.right(),
            Side::Left => self.left(),
        }
    }

    #[inline]
    pub fn edge(&self, side: Side) -> Edge {
        Edge::new(side, self.side(side), self.span(side.edge_dir()))
    }

    /// Snaps the corners of this rectangle to the given grid.
    ///
    /// Note that the rectangle may have zero area after snapping.
    #[inline]
    pub fn snap_to_grid(&self, grid: i64) -> Self {
        Self::new(self.p0.snap_to_grid(grid), self.p1.snap_to_grid(grid))
    }

    pub fn cutout(&self, clip: Rect) -> [Rect; 4] {
        let src = *self;
        let t_span = Span::new(clip.top(), src.top());
        let b_span = Span::new(src.bottom(), clip.bottom());
        let l_span = Span::new(src.left(), clip.left());
        let r_span = Span::new(clip.right(), src.right());

        [
            Rect::from_spans(src.hspan(), t_span),
            Rect::from_spans(src.hspan(), b_span),
            Rect::from_spans(l_span, src.vspan()),
            Rect::from_spans(r_span, src.vspan()),
        ]
    }
}

impl Trim<Rect> for Rect {
    type Output = Self;

    fn trim(&self, bounds: &Rect) -> Option<Self::Output> {
        let intersect = self.bbox().intersection(bounds.bbox());
        if intersect.is_empty() {
            None
        } else {
            Some(intersect.into_rect())
        }
    }
}

/// Specifies how to expand geometry.
///
/// See [`Rect::expand_dims`] for more information.
#[derive(Copy, Clone, Eq, PartialEq, Default, Debug, Hash, Serialize, Deserialize)]
pub enum ExpandMode {
    #[default]
    All,
    LowerLeft,
    LowerRight,
    UpperLeft,
    UpperRight,
}

/// A helper struct for building [`Rect`]s from [`Span`]s.
#[derive(Clone, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct RectSpanBuilder {
    hspan: Option<Span>,
    vspan: Option<Span>,
}

impl RectSpanBuilder {
    /// Creates a new [`RectSpanBuilder`].
    pub fn new() -> Self {
        Default::default()
    }

    /// Associates [`Span`] span with direction `dir`.
    pub fn with(&mut self, dir: Dir, span: Span) -> &mut Self {
        match dir {
            Dir::Horiz => self.hspan = Some(span),
            Dir::Vert => self.vspan = Some(span),
        }
        self
    }

    /// Builds a [`Rect`] from the specified spans.
    ///
    /// Panics if one or more directions were left unspecified.
    pub fn build(&self) -> Rect {
        Rect::from_spans(self.hspan.unwrap(), self.vspan.unwrap())
    }
}

impl From<Bbox> for Rect {
    fn from(r: Bbox) -> Self {
        debug_assert!(!r.is_empty());
        debug_assert!(r.p0.x <= r.p1.x);
        debug_assert!(r.p0.y <= r.p1.y);
        Self { p0: r.p0, p1: r.p1 }
    }
}

/// The primary geometric primitive comprising raw layout.
///
/// Variants include [`Rect`], [`Polygon`], and [`Path`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[enum_dispatch(ShapeTrait)]
pub enum Shape {
    Rect(Rect),
    Polygon(Polygon),
    Path(Path),
    Point(Point),
}

impl Trim<Rect> for Shape {
    type Output = Self;
    fn trim(&self, bounds: &Rect) -> Option<Self::Output> {
        match self {
            Self::Rect(r) => r.trim(bounds).map(Self::Rect),
            Self::Point(r) => r.trim(bounds).map(Self::Point),
            _ => todo!(),
        }
    }
}

impl Transform for Shape {
    fn transform(&self, trans: Transformation) -> Self {
        match self {
            Self::Rect(s) => Self::Rect(s.transform(trans)),
            Self::Polygon(s) => Self::Polygon(s.transform(trans)),
            Self::Path(s) => Self::Path(s.transform(trans)),
            Self::Point(s) => Self::Point(s.transform(trans)),
        }
    }
}

impl Translate for Shape {
    fn translate(&mut self, p: Point) {
        match self {
            Self::Rect(s) => s.translate(p),
            Self::Polygon(s) => s.translate(p),
            Self::Path(s) => s.translate(p),
            Self::Point(s) => s.translate(p),
        }
    }
}

impl Shape {
    /// Returns `true` if the shape intersects with [`Shape`] `other`.
    pub fn intersects(&self, _other: &Shape) -> bool {
        todo!() // FIXME!
    }

    pub fn as_rect(&self) -> Option<Rect> {
        if let Shape::Rect(rect) = self {
            Some(*rect)
        } else {
            None
        }
    }
}

/// Common shape operations, dispatched from the [`Shape`] enum to its variants by [mod@enum_dispatch].
#[enum_dispatch]
pub trait ShapeTrait {
    /// Returns our "origin", an arbitrary [`Point`] on the shape.
    fn point0(&self) -> Point;
    /// Returns the direction along which the shape is primarily oriented.
    ///
    /// Primarily used for orienting label-text.
    fn orientation(&self) -> Dir;
    /// Returns `true` if the [`Shape`] contains [`Point`] `pt`.
    ///
    /// Containment is *inclusive* for all [`Shape`] types.
    /// [`Point`]s on their boundary, which generally include all points specifying the shape itself, are regarded throughout as "inside" the shape.
    fn contains(&self, pt: Point) -> bool;
    /// Converts the shape to a [`Polygon`], the most general of shapes.
    fn to_poly(&self) -> Polygon;
}

impl ShapeTrait for Rect {
    fn point0(&self) -> Point {
        self.p0
    }
    fn orientation(&self) -> Dir {
        let (p0, p1) = (&self.p0, &self.p1);
        if (p1.x - p0.x).abs() < (p1.y - p0.y).abs() {
            return Dir::Vert;
        }
        Dir::Horiz
    }
    fn contains(&self, pt: Point) -> bool {
        let (p0, p1) = (&self.p0, &self.p1);
        p0.x.min(p1.x) <= pt.x
            && p0.x.max(p1.x) >= pt.x
            && p0.y.min(p1.y) <= pt.y
            && p0.y.max(p1.y) >= pt.y
    }
    fn to_poly(&self) -> Polygon {
        // Create a four-sided polygon, cloning our corners
        Polygon {
            points: vec![
                self.p0,
                Point::new(self.p1.x, self.p0.y),
                self.p1,
                Point::new(self.p0.x, self.p1.y),
            ],
        }
    }
}
impl ShapeTrait for Polygon {
    fn point0(&self) -> Point {
        self.points[0]
    }
    fn orientation(&self) -> Dir {
        // FIXME: always horizontal, at least for now
        Dir::Horiz
    }
    fn contains(&self, pt: Point) -> bool {
        // First check for the fast way out: if the point is outside the bounding box, it can't be in the polygon.
        if !self.points.bbox().contains(pt) {
            return false;
        }

        // Not quite so lucky this time. Now do some real work. Using the "winding number" algorithm, which works for all (realistically useful) layout-polygons.
        let mut winding_num: isize = 0;
        for idx in 0..self.points.len() {
            // Grab the segment's start and end points.
            // Note these accesses go one past `points.len`, closing the polygon back at its first point.
            let (past, next) = (
                &self.points[idx],
                &self.points[(idx + 1) % self.points.len()],
            );

            // First check whether the point is anywhere in the y-range of this segment
            if past.y.min(next.y) <= pt.y && past.y.max(next.y) >= pt.y {
                // May have a hit here. Sort out whether the semi-infinite horizontal line at `y=pt.y` intersects the edge.
                if next.y == past.y {
                    // This is a horizontal segment, and we're on the same y-level as the point.
                    // If its x-coordinate also lies within range, no need for further checks, we've got a hit.
                    if past.x.min(next.x) <= pt.x && past.x.max(next.x) >= pt.x {
                        return true;
                    }
                    // Otherwise "hits" against these horizontal segments are not counted in `winding_num`.
                    // (FIXME: double-check this.)
                } else {
                    // This is a non-horizontal segment. Check for intersection.
                    let xsolve = (next.x - past.x) * (pt.y - past.y) / (next.y - past.y) + past.x;

                    match xsolve.cmp(&pt.x) {
                        Ordering::Equal => return true,
                        Ordering::Greater => {
                            if next.y > past.y {
                                winding_num += 1;
                            } else {
                                winding_num -= 1;
                            }
                        }
                        Ordering::Less => (),
                    }
                }
            }
        }
        // Trick is: if the winding number is non-zero, we're inside the polygon. And if it's zero, we're outside.
        winding_num != 0
    }
    fn to_poly(&self) -> Polygon {
        self.clone()
    }
}
impl ShapeTrait for Path {
    fn point0(&self) -> Point {
        self.points[0]
    }
    fn orientation(&self) -> Dir {
        // FIXME: always horizontal, at least for now
        Dir::Horiz
    }
    fn contains(&self, pt: Point) -> bool {
        // Break into segments, and check for intersection with each
        // Probably not the most efficient way to do this, but a start.
        // Only "Manhattan paths", i.e. those with segments solely running vertically or horizontally, are supported.
        // FIXME: even with this method, there are some small pieces at corners which we'll miss.
        // Whether these are relevant in real life, tbd.
        let (points, width) = (&self.points, self.width);
        let width = i64::try_from(width).unwrap(); // FIXME: probably store these signed, check them on creation
        for k in 0..points.len() - 1 {
            let rect = if points[k].x == points[k + 1].x {
                Rect {
                    p0: Point::new(points[k].x - width / 2, points[k].y),
                    p1: Point::new(points[k].x + width / 2, points[k + 1].y),
                }
            } else if points[k].y == points[k + 1].y {
                Rect {
                    p0: Point::new(points[k].x, points[k].y - width / 2),
                    p1: Point::new(points[k + 1].x, points[k].y + width / 2),
                }
            } else {
                unimplemented!("Unsupported Non-Manhattan Path")
            };
            if rect.contains(pt) {
                return true;
            }
        }
        false
    }
    fn to_poly(&self) -> Polygon {
        unimplemented!("Path::to_poly")
    }
}
impl ShapeTrait for Point {
    fn point0(&self) -> Point {
        *self
    }
    fn orientation(&self) -> Dir {
        // FIXME: always horizontal, at least for now
        Dir::Horiz
    }
    fn contains(&self, pt: Point) -> bool {
        pt == *self
    }
    fn to_poly(&self) -> Polygon {
        panic!("Cannot convert a Point to a Polygon")
    }
}

/// A horizontal and vertical rectangular dimension with no specified location.
#[derive(
    Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, Serialize, Deserialize,
)]
pub struct Dims {
    /// The width dimension.
    w: i64,
    /// The height dimension.
    h: i64,
}

/// A structure for building [`Dims`] from a width and height.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, Serialize, Deserialize)]
pub struct DimsBuilder {
    w: Option<i64>,
    h: Option<i64>,
}

impl DimsBuilder {
    /// Creates a new [`DimsBuilder`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the length of the dimensions along the direction `dir`.
    pub fn set(mut self, dir: Dir, value: i64) -> Self {
        match dir {
            Dir::Horiz => self.w = Some(value),
            Dir::Vert => self.h = Some(value),
        }
        self
    }

    /// Creates a [`Dims`] object, panicking if the width or height was not specified.
    pub fn build(self) -> Dims {
        Dims::new(self.w.unwrap(), self.h.unwrap())
    }
}

impl Dims {
    /// Creates a new [`Dims`] from a width and height.
    pub fn new(w: i64, h: i64) -> Self {
        Self { w, h }
    }
    /// Creates a new [`Dims`] with width and height equal to `value`.
    pub fn square(value: i64) -> Self {
        Self { w: value, h: value }
    }
    /// Creates a new [`DimsBuilder`].
    pub fn builder() -> DimsBuilder {
        DimsBuilder::default()
    }

    /// Returns the dimension in the specified direction.
    pub fn dim(&self, dir: Dir) -> i64 {
        match dir {
            Dir::Vert => self.h,
            Dir::Horiz => self.w,
        }
    }

    /// Returns the direction of the longer dimension.
    ///
    /// If the width and height are equal, returns [`Dir::Horiz`].
    pub fn longer_dir(&self) -> Dir {
        if self.w >= self.h {
            Dir::Horiz
        } else {
            Dir::Vert
        }
    }

    /// Returns the direction of the longer dimension.
    ///
    /// If the width and height are equal, returns [`None`].
    /// Otherwise, returns a `Some` variant containing the longer direction.
    pub fn longer_dir_strict(&self) -> Option<Dir> {
        match self.w.cmp(&self.h) {
            Ordering::Greater => Some(Dir::Horiz),
            Ordering::Equal => None,
            Ordering::Less => Some(Dir::Vert),
        }
    }

    /// Returns a new [`Dims`] object with the horizontal and vertical dimensions flipped.
    pub fn transpose(self) -> Self {
        Self {
            w: self.h,
            h: self.w,
        }
    }

    /// Returns the width (ie. the horizontal dimension).
    #[inline]
    pub fn width(&self) -> i64 {
        self.w
    }

    /// Returns the height (ie. the vertical dimension).
    #[inline]
    pub fn height(&self) -> i64 {
        self.h
    }

    /// Returns the width (ie. the horizontal dimension).
    ///
    /// A shorthand for [`Dims::width`].
    #[inline]
    pub fn w(&self) -> i64 {
        self.width()
    }

    /// Returns the height (ie. the vertical dimension).
    ///
    /// A shorthand for [`Dims::height`].
    #[inline]
    pub fn h(&self) -> i64 {
        self.height()
    }

    /// Converts this dimension object into a [`Rect`].
    ///
    /// See [`Rect::with_dims`] for more information.
    #[inline]
    pub fn into_rect(self) -> Rect {
        Rect::with_dims(self)
    }

    /// Converts this dimension object into a [`Point`] with coordinates `(self.w(), self.h())`.
    #[inline]
    pub fn into_point(self) -> Point {
        Point::new(self.w(), self.h())
    }
}

impl std::ops::Add<Dims> for Dims {
    type Output = Self;
    fn add(self, rhs: Dims) -> Self::Output {
        Self {
            w: self.w + rhs.w,
            h: self.h + rhs.h,
        }
    }
}

impl std::ops::Sub<Dims> for Dims {
    type Output = Self;
    fn sub(self, rhs: Dims) -> Self::Output {
        Self {
            w: self.w - rhs.w,
            h: self.h - rhs.h,
        }
    }
}

impl std::ops::Mul<i64> for Dims {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self::Output {
        Self {
            w: self.w * rhs,
            h: self.h * rhs,
        }
    }
}

impl std::ops::Mul<(usize, usize)> for Dims {
    type Output = Self;
    fn mul(self, rhs: (usize, usize)) -> Self::Output {
        Self {
            w: self.w * rhs.0 as i64,
            h: self.h * rhs.1 as i64,
        }
    }
}

impl std::ops::AddAssign<Dims> for Dims {
    fn add_assign(&mut self, rhs: Dims) {
        self.w += rhs.w;
        self.h += rhs.h;
    }
}

impl std::ops::SubAssign<Dims> for Dims {
    fn sub_assign(&mut self, rhs: Dims) {
        self.w -= rhs.w;
        self.h -= rhs.h;
    }
}

impl std::ops::MulAssign<i64> for Dims {
    fn mul_assign(&mut self, rhs: i64) {
        self.w *= rhs;
        self.h *= rhs;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    #[test]
    fn transform_identity() {
        let shape1 = Shape::Rect(Rect {
            p0: Point::new(0, 0),
            p1: Point::new(1, 1),
        });
        let trans = Transformation::identity();
        let shape2 = shape1.transform(trans);
        assert_eq!(shape2, shape1);
    }
    #[test]
    fn transform_rotate() {
        let shape1 = Shape::Rect(Rect {
            p0: Point::new(0, 0),
            p1: Point::new(1, 1),
        });
        let trans = Transformation::rotate(90.);
        let shape2 = shape1.transform(trans);
        assert_eq!(
            shape2,
            Shape::Rect(Rect {
                p0: Point::new(-1, 0),
                p1: Point::new(0, 1),
            })
        );
        let shape3 = shape2.transform(trans);
        assert_eq!(
            shape3,
            Shape::Rect(Rect {
                p0: Point::new(-1, -1),
                p1: Point::new(0, 0),
            })
        );
        let shape4 = shape3.transform(trans);
        assert_eq!(
            shape4,
            Shape::Rect(Rect {
                p0: Point::new(0, -1),
                p1: Point::new(1, 0),
            })
        );
        let shape0 = shape4.transform(trans);
        assert_eq!(shape0, shape1);
    }
    #[test]
    fn test_cascade1() {
        let trans1 = Transformation::reflect_vert();
        let trans2 = Transformation::translate(1., 1.);

        let p = Point::new(1, 1);
        let cascade1 = Transformation::cascade(trans1, trans2);
        let pc1 = p.transform(cascade1);
        assert_eq!(pc1, Point::new(2, -2));

        let cascade2 = Transformation::cascade(trans2, trans1);
        let pc1 = p.transform(cascade2);
        assert_eq!(pc1, Point::new(2, 0));
    }
    #[test]
    fn test_polygon_contains() {
        // Test polygon-point containment of several flavors

        // Create a right triangle at the origin
        let triangle = Polygon {
            points: vec![Point::new(0, 0), Point::new(2, 0), Point::new(0, 2)],
        };
        assert!(triangle.contains(Point::new(0, 0)));
        assert!(triangle.contains(Point::new(1, 0)));
        assert!(triangle.contains(Point::new(2, 0)));
        assert!(triangle.contains(Point::new(0, 1)));
        assert!(triangle.contains(Point::new(1, 1)));
        assert!(!triangle.contains(Point::new(2, 2)));

        // Create a 2:1 tall-ish diamond-shape
        let diamond = Polygon {
            points: vec![
                Point::new(1, 0),
                Point::new(2, 2),
                Point::new(1, 4),
                Point::new(0, 2),
            ],
        };
        assert!(!diamond.contains(Point::new(0, 0)));
        assert!(!diamond.contains(Point::new(100, 100)));
        // Check a few points through its vertical center
        assert!(diamond.contains(Point::new(1, 0)));
        assert!(diamond.contains(Point::new(1, 1)));
        assert!(diamond.contains(Point::new(1, 2)));
        assert!(diamond.contains(Point::new(1, 3)));
        assert!(diamond.contains(Point::new(1, 4)));
        // And its horizontal center
        assert!(diamond.contains(Point::new(0, 2)));
        assert!(diamond.contains(Point::new(1, 2)));
        assert!(diamond.contains(Point::new(2, 2)));

        // More fun: create a U-shaped polygon, inside a 10x10 square
        let u = Polygon {
            points: vec![
                Point::new(0, 0),
                Point::new(0, 10),
                Point::new(2, 10),
                Point::new(2, 2),
                Point::new(8, 2),
                Point::new(8, 10),
                Point::new(10, 10),
                Point::new(10, 0),
            ],
        };
        for pt in &u.points {
            assert!(u.contains(*pt));
        }
        assert!(u.contains(Point::new(1, 1)));
        assert!(u.contains(Point::new(1, 9)));
        assert!(u.contains(Point::new(9, 9)));
        assert!(u.contains(Point::new(9, 1)));
        // Points "inside" the u-part, i.e. "outside" the polygon
        assert!(!u.contains(Point::new(3, 3)));
        assert!(!u.contains(Point::new(3, 9)));
        assert!(!u.contains(Point::new(7, 3)));
        assert!(!u.contains(Point::new(7, 9)));
    }

    #[test]
    fn test_point_snap_to_grid() {
        let pt = Point::new(1, 1);
        let pt = pt.snap_to_grid(500);
        assert_eq!(pt, Point::zero());

        let pt = Point::new(999, 260);
        let pt = pt.snap_to_grid(500);
        assert_eq!(pt, Point::new(1_000, 500));
    }
}
