//! The `ElementGroup` type for grouping `Element`s.
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Orientation;
use subgeom::transform::{Transform, Transformation, Translate};
use subgeom::Point;

use super::Group;
use crate::layout::cell::{Element, Flatten};
use crate::layout::layers::{LayerBoundBox, LayerKey};
use crate::layout::placement::align::AlignRect;
use crate::layout::{Draw, DrawRef};

/// A group of layout [`Element`]s.
///
/// Cannot contain instances of cells.
#[derive(Clone, PartialEq, Default, Debug)]
pub struct ElementGroup {
    /// Translates all elements in the group by an offset.
    ///
    /// Translation is performed after applying the [orientation](ElementGroup::orientation).
    loc: Point,

    /// Applies a transformation to all elements in this group.
    ///
    /// Applied before translating the entire group according to [loc](ElementGroup::loc).
    orientation: Orientation,

    /// The list of [`Element`]s in this group.
    elems: Vec<Element>,
}

impl ElementGroup {
    /// Creates a new, empty [`ElementGroup`].
    ///
    /// No translations or orientations are applied by default.
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn transformation(&self) -> Transformation {
        Transformation::with_loc_and_orientation(self.loc, self.orientation)
    }

    /// Gets the origin of this group.
    #[inline]
    pub fn loc(&self) -> Point {
        self.loc
    }

    /// Sets the position of this group.
    ///
    /// The origin of this group will be placed at the given location.
    #[inline]
    pub fn set_loc(&mut self, p: impl Into<Point>) {
        self.loc = p.into();
    }

    /// Gets the orientation of this group.
    ///
    /// Elements have this orientation applied first, followed by translation.
    #[inline]
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Sets the orientation of this group.
    ///
    /// Elements have the given orientation applied first, followed by translation.
    #[inline]
    pub fn set_orientation(&mut self, o: impl Into<Orientation>) {
        self.orientation = o.into();
    }

    /// Adds a single [`Element`] to this group.
    #[inline]
    pub fn add(&mut self, elt: impl Into<Element>) {
        self.elems.push(elt.into());
    }

    /// Adds all elements in the given iterator to this element group.
    #[inline]
    pub fn extend(&mut self, elems: impl IntoIterator<Item = Element>) {
        self.elems.extend(elems);
    }

    /// Returns an iterator over the elements in this group **after transformation**.
    pub fn elements(&self) -> impl Iterator<Item = Element> + '_ {
        let transformation = self.transformation();
        self.elems.iter().map(move |e| e.transform(transformation))
    }
}

/// An owning iterator over the elements of an [`ElementGroup`].
pub struct IntoIter {
    transformation: Transformation,
    elems: std::vec::IntoIter<Element>,
}

/// An iterator over the elements of an [`ElementGroup`].
pub struct Iter<'a> {
    transformation: Transformation,
    elems: std::slice::Iter<'a, Element>,
}

impl IntoIterator for ElementGroup {
    type Item = Element;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            transformation: self.transformation(),
            elems: self.elems.into_iter(),
        }
    }
}

impl Iterator for IntoIter {
    type Item = Element;

    fn next(&mut self) -> Option<Self::Item> {
        self.elems.next().map(|e| e.transform(self.transformation))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.elems.size_hint()
    }

    fn count(self) -> usize {
        self.elems.count()
    }
}

impl<'a> IntoIterator for &'a ElementGroup {
    type Item = Element;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            transformation: self.transformation(),
            elems: self.elems.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Element;

    fn next(&mut self) -> Option<Self::Item> {
        self.elems.next().map(|e| e.transform(self.transformation))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.elems.size_hint()
    }

    fn count(self) -> usize {
        self.elems.count()
    }
}

impl BoundBox for ElementGroup {
    fn bbox(&self) -> Bbox {
        let mut bbox = Bbox::empty();
        for elem in self.elements() {
            bbox = elem.inner.union(bbox);
        }
        bbox
    }
}

impl Translate for ElementGroup {
    #[inline]
    fn translate(&mut self, p: Point) {
        self.loc.translate(p);
    }
}

impl AlignRect for ElementGroup {}

impl Draw for ElementGroup {
    fn draw(self) -> crate::error::Result<Group> {
        Ok(Group {
            loc: self.loc(),
            elems: self.elems,
            ..Default::default()
        })
    }
}

impl DrawRef for ElementGroup {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(Group {
            loc: self.loc(),
            elems: self.elems.clone(),
            ..Default::default()
        })
    }
}

impl LayerBoundBox for ElementGroup {
    fn layer_bbox(&self, key: LayerKey) -> Bbox {
        let mut bbox = Bbox::empty();
        for elem in self.elements() {
            if elem.layer.layer() == key {
                bbox = bbox.union(elem.bbox());
            }
        }
        bbox
    }
}

impl Flatten for ElementGroup {
    /// Element groups have no hierarchy,
    /// so flattening them does nothing.
    fn flatten(&mut self) {}
}
