use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use subgeom::bbox::{Bbox, BoundBox};
use subgeom::transform::Translate;
use subgeom::{Dims, ExpandMode, Rect, Side, Sides};

use crate::borrow::Shared;
use crate::layout::cell::{Element, Instance};
use crate::layout::group::elements::ElementGroup;
use crate::layout::group::Group;
use crate::layout::layers::{LayerBoundBox, LayerKey};
use crate::layout::{Draw, DrawRef};

pub type Padding = Sides<i64>;

#[non_exhaustive]
#[derive(Clone)]
pub enum Tile<'a> {
    Instance(Shared<'a, Instance>),
    ElementGroup(Shared<'a, ElementGroup>),
    Group(Shared<'a, Group>),
    Element(Shared<'a, Element>),
    Custom(Shared<'a, Arc<dyn CustomTile>>),
}

pub trait CustomTile: BoundBox + DrawRef {}

impl From<Instance> for Tile<'static> {
    fn from(value: Instance) -> Self {
        Self::Instance(Shared::from_owned(value))
    }
}

impl<'a> From<&'a Instance> for Tile<'a> {
    fn from(value: &'a Instance) -> Self {
        Self::Instance(Shared::from_borrow(value))
    }
}

impl From<ElementGroup> for Tile<'static> {
    fn from(value: ElementGroup) -> Self {
        Self::ElementGroup(Shared::from_owned(value))
    }
}

impl<'a> From<&'a ElementGroup> for Tile<'a> {
    fn from(value: &'a ElementGroup) -> Self {
        Self::ElementGroup(Shared::from_borrow(value))
    }
}

impl From<Group> for Tile<'static> {
    fn from(value: Group) -> Self {
        Self::Group(Shared::from_owned(value))
    }
}

impl<'a> From<&'a Group> for Tile<'a> {
    fn from(value: &'a Group) -> Self {
        Self::Group(Shared::from_borrow(value))
    }
}

impl From<Element> for Tile<'static> {
    fn from(value: Element) -> Self {
        Self::Element(Shared::from_owned(value))
    }
}

impl<'a> From<&'a Element> for Tile<'a> {
    fn from(value: &'a Element) -> Self {
        Self::Element(Shared::from_borrow(value))
    }
}

impl<'a> From<&'a Tile<'a>> for Tile<'a> {
    fn from(value: &'a Tile) -> Self {
        value.borrowed()
    }
}

impl<T> From<T> for Tile<'static>
where
    T: CustomTile + 'static,
{
    fn from(value: T) -> Self {
        let x: Arc<dyn CustomTile> = Arc::new(value);
        Self::Custom(Shared::from(x))
    }
}

#[derive(Clone, Default)]
pub struct OptionTile<'a>(Option<Tile<'a>>);

impl<'a> OptionTile<'a> {
    pub fn new(inner: Tile<'a>) -> Self {
        Self(Some(inner))
    }

    pub fn empty() -> Self {
        Self(None)
    }

    #[inline]
    pub fn into_inner(self) -> Option<Tile<'a>> {
        self.0
    }

    pub fn borrowed(&'a self) -> Self {
        Self(self.0.as_ref().map(|t| t.borrowed()))
    }
}

impl<'a> From<Option<Tile<'a>>> for OptionTile<'a> {
    fn from(value: Option<Tile<'a>>) -> Self {
        Self(value)
    }
}

impl<'a> From<OptionTile<'a>> for Option<Tile<'a>> {
    fn from(val: OptionTile<'a>) -> Self {
        val.0
    }
}

impl<'a> Deref for OptionTile<'a> {
    type Target = Option<Tile<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> DerefMut for OptionTile<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// From https://doc.rust-lang.org/std/convert/trait.AsRef.html#generic-implementations
impl<'a, T> AsRef<T> for OptionTile<'a>
where
    T: ?Sized,
    <OptionTile<'a> as Deref>::Target: AsRef<T>,
{
    fn as_ref(&self) -> &T {
        AsRef::as_ref(self.deref())
    }
}

// From https://doc.rust-lang.org/std/convert/trait.AsMut.html#generic-implementations
impl<'a, T> AsMut<T> for OptionTile<'a>
where
    <OptionTile<'a> as Deref>::Target: AsMut<T>,
{
    fn as_mut(&mut self) -> &mut T {
        AsMut::as_mut(self.deref_mut())
    }
}

impl<'a, T> From<T> for OptionTile<'a>
where
    T: Into<Tile<'a>>,
{
    fn from(value: T) -> Self {
        Self(Some(value.into()))
    }
}

impl<'a> Tile<'a> {
    pub fn dims(&self) -> Dims {
        self.bbox().into_rect().dims()
    }

    pub fn borrowed(&'a self) -> Self {
        use Tile::*;
        match self {
            Instance(v) => Instance(v.borrowed()),
            ElementGroup(v) => ElementGroup(v.borrowed()),
            Group(v) => Group(v.borrowed()),
            Element(v) => Element(v.borrowed()),
            Custom(v) => Custom(v.borrowed()),
        }
    }
}

impl<'a> BoundBox for Tile<'a> {
    fn bbox(&self) -> Bbox {
        match self {
            Self::Instance(v) => v.bbox(),
            Self::ElementGroup(v) => v.bbox(),
            Self::Group(v) => v.bbox(),
            Self::Element(v) => v.bbox(),
            Self::Custom(v) => v.bbox(),
        }
    }
}
impl<'a> Draw for Tile<'a> {
    fn draw(self) -> crate::error::Result<Group> {
        match self {
            Self::Instance(v) => v.draw(),
            Self::ElementGroup(v) => v.draw(),
            Self::Group(v) => v.draw(),
            Self::Element(v) => v.draw(),
            // For boxed items, must draw as a reference
            Self::Custom(v) => v.draw_ref(),
        }
    }
}
impl<'a> DrawRef for Tile<'a> {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        match self {
            Self::Instance(v) => v.draw_ref(),
            Self::ElementGroup(v) => v.draw_ref(),
            Self::Group(v) => v.draw_ref(),
            Self::Element(v) => v.draw_ref(),
            Self::Custom(v) => v.draw_ref(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Pad<T> {
    inner: T,
    padding: Padding,
}

impl<T> Pad<T>
where
    T: BoundBox + DrawRef,
{
    pub fn new(inner: T, padding: impl Into<Padding>) -> Self {
        Self {
            inner,
            padding: padding.into(),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Translate for Pad<T>
where
    T: Translate,
{
    fn translate(&mut self, p: subgeom::Point) {
        self.inner.translate(p);
    }
}

impl<T> BoundBox for Pad<T>
where
    T: BoundBox,
{
    fn bbox(&self) -> Bbox {
        self.inner
            .bbox()
            .into_rect()
            .expand_dims(
                Dims::new(self.padding[Side::Left], self.padding[Side::Bot]),
                ExpandMode::LowerLeft,
            )
            .expand_dims(
                Dims::new(self.padding[Side::Right], self.padding[Side::Top]),
                ExpandMode::UpperRight,
            )
            .bbox()
    }
}

impl<T> Draw for Pad<T>
where
    T: Draw,
{
    fn draw(self) -> crate::error::Result<Group> {
        self.inner.draw()
    }
}

impl<T> DrawRef for Pad<T>
where
    T: DrawRef,
{
    fn draw_ref(&self) -> crate::error::Result<Group> {
        self.inner.draw_ref()
    }
}

impl<T> CustomTile for Pad<T> where T: BoundBox + DrawRef {}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct LayerBbox<T> {
    inner: T,
    layer: LayerKey,
}

impl<T> LayerBbox<T>
where
    T: LayerBoundBox + DrawRef,
{
    pub fn new(inner: T, layer: impl Into<LayerKey>) -> Self {
        Self {
            inner,
            layer: layer.into(),
        }
    }
}

impl<T> BoundBox for LayerBbox<T>
where
    T: LayerBoundBox,
{
    fn bbox(&self) -> Bbox {
        self.inner.layer_bbox(self.layer)
    }
}

impl<T> Draw for LayerBbox<T>
where
    T: Draw,
{
    fn draw(self) -> crate::error::Result<Group> {
        self.inner.draw()
    }
}

impl<T> DrawRef for LayerBbox<T>
where
    T: DrawRef,
{
    fn draw_ref(&self) -> crate::error::Result<Group> {
        self.inner.draw_ref()
    }
}

impl<T> CustomTile for LayerBbox<T> where T: LayerBoundBox + DrawRef {}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct RectBbox<T> {
    inner: T,
    bbox: Rect,
}

impl<T> RectBbox<T> {
    pub fn new(inner: T, bbox: Rect) -> Self {
        Self { inner, bbox }
    }
}

impl<T> BoundBox for RectBbox<T> {
    fn bbox(&self) -> Bbox {
        self.bbox.bbox()
    }
}

impl<T> Draw for RectBbox<T>
where
    T: Draw,
{
    fn draw(self) -> crate::error::Result<Group> {
        self.inner.draw()
    }
}

impl<T> DrawRef for RectBbox<T>
where
    T: DrawRef,
{
    fn draw_ref(&self) -> crate::error::Result<Group> {
        self.inner.draw_ref()
    }
}

impl<T> CustomTile for RectBbox<T> where T: DrawRef {}

pub struct RelativeRectBbox<T> {
    inner: T,
    bbox: Rect,
}

impl<T> RelativeRectBbox<T> {
    pub fn new(inner: T, bbox: Rect) -> Self {
        Self { inner, bbox }
    }
}

impl<T> BoundBox for RelativeRectBbox<T>
where
    T: BoundBox,
{
    fn bbox(&self) -> Bbox {
        let inner = self.inner.bbox();
        Bbox::new(inner.p0 + self.bbox.p0, inner.p0 + self.bbox.p1)
    }
}

impl<T> Draw for RelativeRectBbox<T>
where
    T: Draw,
{
    fn draw(self) -> crate::error::Result<Group> {
        self.inner.draw()
    }
}

impl<T> DrawRef for RelativeRectBbox<T>
where
    T: DrawRef,
{
    fn draw_ref(&self) -> crate::error::Result<Group> {
        self.inner.draw_ref()
    }
}

impl<T> CustomTile for RelativeRectBbox<T> where T: BoundBox + DrawRef {}
