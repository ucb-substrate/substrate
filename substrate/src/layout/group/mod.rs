//! Groups of layout objects.
//!
//! For cases when you want a collection of objects,
//! but you don't want to create a separate [`Component`](crate::component::Component).
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Orientation;
use subgeom::transform::{Transform, Transformation, Translate};
use subgeom::{Point, Rect, Shape};

use super::cell::{
    BusPort, CellPort, Instance, PortConflictStrategy, PortError, PortId, PortMap, PortMapFn,
    TextElement, TransformedPort,
};
use super::layers::{LayerBoundBox, LayerKey, LayerPurpose, UserLayer};
use super::{Draw, DrawRef};
use crate::deps::arcstr::ArcStr;
use crate::layout::cell::{flatten_recur, Element, Flatten};
use crate::layout::placement::align::AlignRect;

pub mod elements;

/// A group of layout [`Element`]s, [`Instance`]s, and/or [`TextElement`]s.
///
/// Cannot contain ports or blockages. If you need those features, create a
/// [`Component`](crate::component::Component).
///
/// If you only need to store [`Element`]s, you may get more flexibility
/// by using [`ElementGroup`](elements::ElementGroup) instead.
///
/// In particular, [`Group`]s can only be translated, not rotated/reflected.
#[derive(Clone, Default, Debug)]
pub struct Group {
    /// Translates all elements in the group by an offset.
    loc: Point,
    /// The orientation of the group.
    orientation: Orientation,
    /// The list of [`Element`]s in this group.
    elems: Vec<Element>,
    /// The list of [`Instance`]s in this group.
    insts: Vec<Instance>,
    /// The list of [`TextElement`]s in this group.
    annotations: Vec<TextElement>,
    /// A map of ports.
    ports: PortMap,
}

pub trait GroupPortMapFn: PortMapFn<Instance> {}
impl<F> GroupPortMapFn for F where F: PortMapFn<Instance> {}

impl Group {
    /// Creates a new, empty [`Group`].
    ///
    /// No translations or orientations are applied by default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the transformation of the group.
    #[inline]
    pub fn transformation(&self) -> Transformation {
        Transformation::with_loc_and_orientation(self.loc, self.orientation)
    }

    /// Gets the origin of the group.
    #[inline]
    pub fn loc(&self) -> Point {
        self.loc
    }

    /// Sets the position of the group.
    ///
    /// All elements of this group will be translated by the given location.
    #[inline]
    pub fn set_loc(&mut self, p: impl Into<Point>) {
        self.loc = p.into();
    }

    /// Returns the orientation of the group.
    #[inline]
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Returns a mutable reference to the orientation of the group.
    ///
    /// Note that changing the orientation of a translated group can result in
    /// unpredictable results, since translations are applied **after** orientations.
    #[inline]
    pub fn orientation_mut(&mut self) -> &mut Orientation {
        &mut self.orientation
    }

    /// Sets the orientation of the group.
    ///
    /// Note that changing the orientation of a translated group can result in
    /// unpredictable results, since translations are applied **after** orientations.
    #[inline]
    pub fn set_orientation(&mut self, o: impl Into<Orientation>) {
        self.orientation = o.into();
    }

    /// Adds an item to the group.
    pub fn add(&mut self, item: impl Into<GroupItem>) {
        let item = item.into();
        match item {
            GroupItem::Element(elt) => self.elems.push(elt),
            GroupItem::Instance(inst) => self.insts.push(inst),
            GroupItem::TextElement(text) => self.annotations.push(text),
        }
    }

    /// Adds all items in the given iterator to this element group.
    #[inline]
    pub fn extend(&mut self, items: impl IntoIterator<Item = GroupItem>) {
        for item in items {
            self.add(item);
        }
    }

    /// Adds a single [`Element`] to this group.
    #[inline]
    pub fn add_element(&mut self, elt: impl Into<Element>) {
        self.elems.push(elt.into());
    }

    /// Adds a single [`Rect`] to this group.
    pub fn add_rect(&mut self, layer: impl Into<UserLayer>, rect: impl Into<Rect>) {
        let layer = layer.into().to_spec(LayerPurpose::Drawing);
        self.elems.push(Element::new(layer, rect.into()));
    }

    /// Adds all elements in the given iterator to this element group.
    #[inline]
    pub fn extend_elements(&mut self, elems: impl IntoIterator<Item = Element>) {
        self.elems.extend(elems);
    }

    /// Returns an iterator over the elements in this group **after transformation**.
    pub fn elements(&self) -> impl Iterator<Item = Element> + '_ {
        let transformation = self.transformation();
        self.elems.iter().map(move |e| e.transform(transformation))
    }

    /// Adds a single [`Instance`] to this group.
    #[inline]
    pub fn add_instance(&mut self, elt: impl Into<Instance>) {
        self.insts.push(elt.into());
    }

    /// Adds all instances in the given iterator to this element group.
    #[inline]
    pub fn extend_insts(&mut self, insts: impl IntoIterator<Item = Instance>) {
        self.insts.extend(insts);
    }

    /// Returns an iterator over the instances in this group **after transformation**.
    pub fn instances(&self) -> impl Iterator<Item = Instance> + '_ {
        let tf = self.transformation();
        self.insts.iter().map(move |i| i.transform(tf))
    }

    /// Returns an iterator over the text annotations in this group **after transformation**.
    pub fn annotations(&self) -> impl Iterator<Item = TextElement> + '_ {
        let tf = self.transformation();
        self.annotations.iter().map(move |i| i.transform(tf))
    }

    /// Exposes ports from [`Instance`]s within this group.
    pub fn expose_ports(
        &mut self,
        mut port_map_fn: impl GroupPortMapFn,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        let insts: Vec<Instance> = self.instances().collect();
        for inst in insts {
            for port in inst.ports() {
                if let Some(port) = port_map_fn.map(port, inst.clone()) {
                    self.add_port_with_strategy(port, port_conflict_strategy)?;
                }
            }
        }
        Ok(())
    }

    #[inline]
    pub fn port_map(&self) -> &PortMap {
        &self.ports
    }

    /// Returns an iterator over the bus ports in the cell.
    #[inline]
    pub fn bus_ports(&self) -> impl Iterator<Item = (&ArcStr, &BusPort)> {
        self.ports.bus_ports()
    }

    /// Retrieves a reference to the [`CellPort`] with id `id`.
    pub fn port(
        &self,
        id: impl Into<PortId>,
    ) -> std::result::Result<TransformedPort<CellPort>, PortError> {
        let port = self.ports.port(id)?;
        Ok(TransformedPort {
            transformation: self.transformation(),
            inner: port,
        })
    }

    /// Returns an iterator over the ports in the cell.
    #[inline]
    pub fn ports(&self) -> impl Iterator<Item = CellPort> + '_ {
        self.ports
            .ports()
            .map(|port| self.port(port.id.clone()).unwrap().into_cell_port())
    }

    /// Adds a single [`CellPort`] to this group.
    #[inline]
    pub fn add_port(&mut self, port: impl Into<CellPort>) -> Result<(), PortError> {
        self.ports.add_port(port)
    }

    /// Adds a single [`CellPort`] to this group, resolving conflicts with the provided strategy.
    #[inline]
    pub fn add_port_with_strategy(
        &mut self,
        port: impl Into<CellPort>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        self.ports
            .add_port_with_strategy(port, port_conflict_strategy)
    }

    /// Adds all ports in the given iterator to this element group.
    pub fn add_ports(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
    ) -> Result<(), PortError> {
        self.ports.add_ports(ports)
    }

    /// Adds all ports in the given iterator to this element group, resolving conflicts with the
    /// provided strategy.
    pub fn add_ports_with_strategy(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        for port in ports.into_iter() {
            self.add_port_with_strategy(port, port_conflict_strategy)?
        }
        Ok(())
    }

    pub fn add_group(&mut self, other: Group) {
        self.elems.extend(other.elements());
        self.insts.extend(other.instances());
        self.annotations.extend(other.annotations());
    }

    /// Reflects the group vertically without modifying its bounding box.
    pub fn reflect_vert_anchored(&mut self) -> &mut Self {
        let box0 = self.bbox();
        self.orientation.reflect_vert();

        let box1 = self.bbox();
        self.loc.y += box0.p0.y - box1.p0.y;
        self.loc.x += box0.p0.x - box1.p0.x;

        #[cfg(debug_assertions)]
        {
            let final_box = self.bbox();
            debug_assert_eq!(final_box, box0);
        }
        self
    }

    /// Reflects the group horizontally without modifying its bounding box.
    pub fn reflect_horiz_anchored(&mut self) -> &mut Self {
        let box0 = self.bbox();
        self.orientation.reflect_horiz();

        let box1 = self.bbox();
        self.loc.x += box0.p0.x - box1.p0.x;
        self.loc.y += box0.p0.y - box1.p0.y;

        #[cfg(debug_assertions)]
        {
            let final_box = self.bbox();
            debug_assert_eq!(final_box, box0);
        }

        self
    }

    pub fn shapes_on(&self, layer: LayerKey) -> Box<dyn Iterator<Item = Shape> + '_> {
        let tf = self.transformation();
        let recur = self.instances().flat_map(move |inst| {
            inst.shapes_on(layer)
                .map(|shape| shape.transform(tf).clone())
                .collect::<Vec<Shape>>()
        });
        let curr = self
            .elements()
            .filter(move |elem| elem.layer.layer() == layer)
            .map(|elem| elem.inner.clone());
        Box::new(curr.chain(recur))
    }
}

impl BoundBox for Group {
    fn bbox(&self) -> Bbox {
        let mut bbox = Bbox::empty();
        for elem in self.elements() {
            bbox = elem.inner.union(bbox);
        }
        for inst in self.instances() {
            bbox = inst.bbox().union(bbox);
        }
        bbox
    }
}

impl LayerBoundBox for Group {
    fn layer_bbox(&self, layer: super::layers::LayerKey) -> Bbox {
        let mut bbox = Bbox::empty();
        for elem in self.elements().filter(|e| e.layer.layer() == layer) {
            bbox = elem.inner.union(bbox);
        }
        for inst in self.instances() {
            bbox = inst.layer_bbox(layer).union(bbox);
        }
        bbox
    }
}

impl Translate for Group {
    #[inline]
    fn translate(&mut self, p: Point) {
        self.loc.translate(p);
    }
}

impl AlignRect for Group {}

/// An enumeration of items that may be added to a [`Group`].
#[derive(Debug, Clone)]
pub enum GroupItem {
    Element(Element),
    Instance(Instance),
    TextElement(TextElement),
}

impl From<Element> for GroupItem {
    fn from(value: Element) -> Self {
        Self::Element(value)
    }
}

impl From<Instance> for GroupItem {
    fn from(value: Instance) -> Self {
        Self::Instance(value)
    }
}

impl From<TextElement> for GroupItem {
    fn from(value: TextElement) -> Self {
        Self::TextElement(value)
    }
}

impl Draw for Group {
    fn draw(self) -> crate::error::Result<Group> {
        Ok(self)
    }
}

impl DrawRef for Group {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.clone())
    }
}

impl From<Instance> for Group {
    fn from(value: Instance) -> Self {
        let mut group = Group::new();
        // Should be able to unwrap since the group is originally empty.
        group.add_ports(value.ports()).unwrap();
        group.add_instance(value);
        group
    }
}

impl From<Element> for Group {
    fn from(value: Element) -> Self {
        Self {
            elems: vec![value],
            ..Default::default()
        }
    }
}

impl Flatten for Group {
    fn flatten(&mut self) {
        flatten_recur(
            &mut self.elems,
            &mut self.annotations,
            Transformation::identity(),
            &self.insts,
        );
        self.insts.clear();
    }
}
