//! Types related to the creation and instantiation of [`Cell`]s.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;
use state::Container;
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::orientation::Orientation;
use subgeom::transform::{Transform, Transformation, Translate};
use subgeom::trim::Trim;
use subgeom::{Dir, Point, Rect, Shape, Side};
use thiserror::Error;

use super::context::LayoutCtx;
use super::group::Group;
use super::layers::{LayerBoundBox, LayerKey, LayerSpec};
use super::placement::align::AlignRect;
use super::validation::validate_cell;
use super::{Draw, DrawRef};
use crate::deps::arcstr::ArcStr;
use crate::error::ErrorSource;
use crate::fmt::signal::{format_signal, BusFmt};

pub type BusPort = HashMap<usize, CellPort>;

/// The layout view of a cell.
#[derive(Debug, Default)]
pub struct Cell {
    /// The cell's identifier.
    id: CellKey,
    /// The cell's name.
    name: ArcStr,
    /// A list of instances contained in the cell.
    insts: Vec<Instance>,
    /// A list of primitive/geometric elements.
    elems: Vec<Element>,
    /// A list of text annotations.
    annotations: Vec<TextElement>,
    /// A map of names to a map of bus indices to geometric ports.
    ports: PortMap,
    /// A map of blockages on each layer.
    #[allow(dead_code)]
    blockages: HashMap<LayerKey, Vec<Shape>>,

    /// Cache of values that are frequently used
    /// after a cell is done being generated.
    ///
    /// Cached values are computed after the cell is [frozen](Cell::freeze).
    cache: Option<Cache>,

    /// User-defined metadata.
    metadata: Container![Send + Sync],
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub(crate) struct Cache {
    bbox: Bbox,
}

new_key_type! {
    /// A unique identifier for cells.
    pub struct CellKey;
}

/// An instance of a cell in a layout.
#[derive(Debug, Clone, Builder)]
pub struct Instance {
    /// The instance name.
    #[builder(default)]
    pub(crate) name: ArcStr,
    /// A pointer to the reference cell.
    pub(crate) cell: Arc<Cell>,
    /// The location of the cell.
    #[builder(default)]
    pub(crate) loc: Point,
    /// The orientation of the cell.
    #[builder(default)]
    pub(crate) orientation: Orientation,
}

impl DrawRef for Instance {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.clone().into())
    }
}

impl Draw for Instance {
    fn draw(self) -> crate::error::Result<Group> {
        Ok(self.into())
    }
}

/// A primitive geometric element.
///
/// Combines a geometric [`Shape`] with a [`LayerSpec`],
/// and optional net connectivity annotation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Element {
    /// The element's net name.
    pub net: Option<ArcStr>,
    /// The layer spec where the element is located.
    pub layer: LayerSpec,
    /// The element's shape.
    pub inner: Shape,
}

impl Element {
    /// Creates a new [`Element`].
    pub fn new(layer: LayerSpec, shape: impl Into<Shape>) -> Self {
        Self {
            net: None,
            layer,
            inner: shape.into(),
        }
    }

    /// Creates a new [`Element`] with net name `net`.
    pub fn with_net_name(
        net: impl Into<ArcStr>,
        layer: LayerSpec,
        shape: impl Into<Shape>,
    ) -> Self {
        Self {
            net: Some(net.into()),
            layer,
            inner: shape.into(),
        }
    }

    pub fn into_inner(self) -> Shape {
        self.inner
    }
}

impl<T> Trim<T> for Element
where
    Shape: Trim<T, Output = Shape>,
{
    type Output = Self;
    fn trim(&self, bounds: &T) -> Option<Self::Output> {
        self.inner.trim(bounds).map(|inner| Self {
            net: self.net.clone(),
            layer: self.layer.clone(),
            inner,
        })
    }
}

impl BoundBox for Element {
    #[inline]
    fn bbox(&self) -> Bbox {
        self.inner.bbox()
    }
}

impl Draw for Element {
    fn draw(self) -> crate::error::Result<Group> {
        Ok(self.into())
    }
}
impl DrawRef for Element {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.clone().into())
    }
}

impl Transform for Element {
    fn transform(&self, trans: Transformation) -> Self {
        Self {
            net: self.net.clone(),
            layer: self.layer.clone(),
            inner: self.inner.transform(trans),
        }
    }
}

impl Translate for Element {
    fn translate(&mut self, p: Point) {
        self.inner.translate(p);
    }
}

/// A text annotation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextElement {
    /// The string value of the annotation.
    pub string: ArcStr,
    /// The location of the annotation
    pub loc: Point,
    /// The layer on which the annotation resides.
    pub layer: LayerSpec,
}

impl<T> Trim<T> for TextElement
where
    Point: Trim<T, Output = Point>,
{
    type Output = Self;
    fn trim(&self, bounds: &T) -> Option<Self::Output> {
        self.loc.trim(bounds).map(|loc| Self {
            string: self.string.clone(),
            loc,
            layer: self.layer.clone(),
        })
    }
}

impl Translate for TextElement {
    fn translate(&mut self, p: Point) {
        self.loc.translate(p);
    }
}

impl Transform for TextElement {
    fn transform(&self, trans: Transformation) -> Self {
        let loc = self.loc.transform(trans);
        Self {
            string: self.string.clone(),
            loc,
            layer: self.layer.clone(),
        }
    }
}

/// Specifies how a (port)[`CellPort`] must be electrically connected.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Serialize, Deserialize)]
pub enum MustConnect {
    /// The shapes in this port are internally connected.
    #[default]
    No,
    /// The shapes in this port must be connected by higher levels of the layout hierarchy.
    Yes,
    /// The shapes in this port are internally connected, but must be externally
    /// connected to all ports with the same `MustConnect` group name.
    Group { name: ArcStr },
}

/// Strategy for resolving conflicts in port identifiers.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortConflictStrategy {
    /// Merge conflicting ports, will never error.
    Merge,
    /// Overwrite conflicting port, will never error.
    Overwrite,
    /// Return an error on conflicting ports.
    #[default]
    Error,
}

pub trait PortMapFn<M> {
    fn map(&mut self, port: CellPort, metadata: M) -> Option<CellPort>;
}

impl<T, M> PortMapFn<M> for T
where
    T: FnMut(CellPort, M) -> Option<CellPort>,
{
    fn map(&mut self, port: CellPort, metadata: M) -> Option<CellPort> {
        self(port, metadata)
    }
}

#[derive(Debug, Default, Clone)]
pub struct PortMap {
    ports: HashMap<ArcStr, BusPort>,
}

impl PortMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_map(ports: HashMap<ArcStr, BusPort>) -> Self {
        Self { ports }
    }
    pub fn add_port(&mut self, port: impl Into<CellPort>) -> Result<(), PortError> {
        self.add_port_with_strategy(port, PortConflictStrategy::default())
    }
    pub fn add_port_with_strategy(
        &mut self,
        port: impl Into<CellPort>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        let port = port.into();
        let id = &port.id;
        match self.ports.entry(port.id.name.clone()) {
            Entry::Occupied(mut o) => match o.get_mut().entry(id.index) {
                Entry::Occupied(mut o) => match port_conflict_strategy {
                    PortConflictStrategy::Error => {
                        return Err(PortError::PortAlreadyExists(id.clone()));
                    }
                    PortConflictStrategy::Merge => {
                        o.get_mut().merge(port);
                    }
                    PortConflictStrategy::Overwrite => {
                        use crate::log::warn;
                        warn!("overwriting existing port with ID {}", port.id);
                        *o.get_mut() = port;
                    }
                },
                Entry::Vacant(v) => {
                    v.insert(port);
                }
            },
            Entry::Vacant(v) => {
                v.insert(HashMap::from([(id.index, port)]));
            }
        }
        Ok(())
    }
    pub fn add_ports(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
    ) -> Result<(), PortError> {
        for port in ports.into_iter() {
            self.add_port(port)?
        }
        Ok(())
    }
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

    /// Retrieves a reference to the [`BusPort`] with name `name`.
    pub fn bus_port(&self, name: &str) -> std::result::Result<&BusPort, PortError> {
        self.ports
            .get(name)
            .ok_or_else(|| PortError::BusNotFound(name.to_string()))
    }

    /// Returns an iterator over the bus ports in the cell.
    #[inline]
    pub fn bus_ports(&self) -> impl Iterator<Item = (&ArcStr, &BusPort)> {
        self.ports.iter()
    }

    /// Retrieves a reference to the [`CellPort`] with id `id`.
    pub fn port(&self, id: impl Into<PortId>) -> std::result::Result<&CellPort, PortError> {
        let id = id.into();

        self.ports
            .get(&id.name)
            .and_then(|bus| bus.get(&id.index))
            .ok_or_else(|| PortError::PortNotFound(id.clone()))
    }

    /// Returns an iterator over the ports in the cell.
    #[inline]
    pub fn ports(&self) -> impl Iterator<Item = &CellPort> {
        self.ports.values().flat_map(|bus| bus.values())
    }

    /// Returns a mutable iterator over the ports in the cell.
    #[inline]
    pub fn ports_mut(&mut self) -> impl Iterator<Item = &mut CellPort> {
        self.ports
            .iter_mut()
            .flat_map(|(_, bus)| bus.iter_mut().map(|(_, port)| port))
    }
}

// An identifier for a `CellPort`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PortId {
    /// The name of the port.
    ///
    /// Should be compatible with SPICE/GDSII identifier requirements.
    pub(crate) name: ArcStr,
    /// The index of this port in its bus.
    pub(crate) index: usize,
}

impl PortId {
    pub fn new(name: impl Into<ArcStr>, index: usize) -> Self {
        Self {
            name: name.into(),
            index,
        }
    }

    pub fn name(&self) -> ArcStr {
        self.name.clone()
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn format_signal(&self, width: usize, format: BusFmt) -> ArcStr {
        format_signal(&self.name, self.index, width, format)
    }
}

impl<T> From<T> for PortId
where
    T: Into<ArcStr>,
{
    fn from(value: T) -> Self {
        Self {
            name: value.into(),
            index: 0,
        }
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.name, self.index)
    }
}

/// The layout representation of a port in a cell.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellPort {
    /// The port's identifier.
    pub(crate) id: PortId,
    /// Shapes, grouped by layer.
    pub(crate) shapes: HashMap<LayerKey, Vec<Shape>>,
    /// Information on how this port must be electrically connected.
    ///
    /// See [`MustConnect`] for more information.
    pub(crate) must_connect: MustConnect,
}

impl Translate for CellPort {
    fn translate(&mut self, p: Point) {
        for shapes in self.shapes.values_mut() {
            for s in shapes.iter_mut() {
                s.translate(p);
            }
        }
    }
}

impl Transform for CellPort {
    fn transform(&self, trans: Transformation) -> Self {
        let shapes = self
            .shapes
            .iter()
            .map(|(k, v)| {
                let v = v.iter().map(|s| s.transform(trans)).collect::<Vec<_>>();
                (*k, v)
            })
            .collect::<HashMap<_, _>>();

        Self {
            id: self.id.clone(),
            shapes,
            must_connect: self.must_connect.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CellPortBuilder {
    id: Option<PortId>,
    shapes: HashMap<LayerKey, Vec<Shape>>,
    must_connect: MustConnect,
}

impl CellPortBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_id(id: impl Into<PortId>) -> Self {
        Self {
            id: Some(id.into()),
            ..Default::default()
        }
    }

    #[inline]
    pub fn id(&mut self, id: impl Into<PortId>) -> &mut Self {
        self.id = Some(id.into());
        self
    }

    pub fn add(&mut self, layer: LayerKey, shape: impl Into<Shape>) -> &mut Self {
        let vec = self
            .shapes
            .entry(layer)
            .or_insert_with(|| Vec::with_capacity(1));
        vec.push(shape.into());
        self
    }

    pub fn must_connect(&mut self, must_connect: impl Into<MustConnect>) -> &mut Self {
        self.must_connect = must_connect.into();
        self
    }

    pub fn build(&mut self) -> CellPort {
        CellPort {
            id: self.id.clone().unwrap(),
            shapes: self.shapes.clone(),
            must_connect: self.must_connect.clone(),
        }
    }
}

/// An enumeration of port-related errors.
#[derive(Debug, Error)]
pub enum PortError {
    /// The desired bus was not found.
    #[error("bus not found: {0}")]
    BusNotFound(String),

    /// The desired port was not found.
    #[error("port not found: {0}")]
    PortNotFound(PortId),

    /// Port already exists.
    #[error("port already exists: {0}")]
    PortAlreadyExists(PortId),

    /// The port does not have geometry on the given layer.
    #[error("no geometry on the given layer")]
    LayerNotPresent(LayerKey),
}

/// A port with a transformation applied during instantiation.
pub struct TransformedPort<'a, P> {
    pub(crate) transformation: Transformation,
    pub(crate) inner: &'a P,
}

/// An iterator over [`TransformedPort`]s.
pub struct TransformedPortIterator<I> {
    inner: I,
    transformation: Transformation,
}

impl Cell {
    /// Create a new and empty Cell with ID `id`.
    pub fn new(id: CellKey) -> Self {
        Self {
            id,
            name: arcstr::literal!("unnamed"),
            ..Default::default()
        }
    }

    /// Returns the ID of the cell.
    #[inline]
    pub fn id(&self) -> CellKey {
        self.id
    }

    /// Returns the name of the cell.
    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    /// Sets the name of the cell.
    #[inline]
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = name.into();
    }

    /// Returns an iterator over the instances in the cell.
    #[inline]
    pub fn insts(&self) -> impl Iterator<Item = &Instance> {
        self.insts.iter()
    }

    /// Adds an instance to the cell.
    pub fn add_inst(&mut self, inst: impl Into<Instance>) {
        debug_assert!(!self.is_frozen());
        self.insts.push(inst.into());
    }

    /// Adds several instances to the cell.
    pub fn add_insts(&mut self, insts: impl IntoIterator<Item = impl Into<Instance>>) {
        debug_assert!(!self.is_frozen());
        for inst in insts {
            self.insts.push(inst.into());
        }
    }

    /// Returns an iterator over the elements in the cell.
    #[inline]
    pub fn elems(&self) -> impl Iterator<Item = &Element> {
        self.elems.iter()
    }

    /// Sets the elements in the cell.
    #[inline]
    pub fn set_elems(&mut self, elems: Vec<Element>) {
        debug_assert!(!self.is_frozen());
        self.elems = elems;
    }

    /// Adds an element to the cell.
    pub fn add<T>(&mut self, elem: T)
    where
        T: Into<Element>,
    {
        debug_assert!(!self.is_frozen());
        self.elems.push(elem.into());
    }

    /// Adds all elements from the given iterator to this cell.
    pub fn add_elements(&mut self, elems: impl IntoIterator<Item = Element>) {
        debug_assert!(!self.is_frozen());
        self.elems.extend(elems);
    }

    /// Adds all instances from the given iterator to this cell.
    pub fn add_instances(&mut self, insts: impl IntoIterator<Item = Instance>) {
        debug_assert!(!self.is_frozen());
        self.insts.extend(insts);
    }

    /// Adds all annotations from the given iterator to this cell.
    pub fn add_annotations(&mut self, annotations: impl IntoIterator<Item = TextElement>) {
        debug_assert!(!self.is_frozen());
        self.annotations.extend(annotations);
    }

    /// Draws a rectangle on the given layer.
    pub fn draw_rect(&mut self, layer: LayerSpec, rect: Rect) {
        debug_assert!(!self.is_frozen());
        self.elems.push(Element {
            net: None,
            inner: Shape::Rect(rect),
            layer,
        });
    }

    /// Returns the annotations in the cell.
    #[inline]
    pub fn annotations(&self) -> impl Iterator<Item = &TextElement> {
        self.annotations.iter()
    }

    /// Adds an annotation to the cell.
    #[inline]
    pub fn add_annotation(&mut self, text_elem: impl Into<TextElement>) {
        debug_assert!(!self.is_frozen());
        self.annotations.push(text_elem.into());
    }

    /// Retrieves a reference to the [`BusPort`] with name `name`.
    pub fn bus_port(&self, name: &str) -> std::result::Result<&BusPort, PortError> {
        self.ports.bus_port(name)
    }

    /// Returns an iterator over the bus ports in the cell.
    #[inline]
    pub fn bus_ports(&self) -> impl Iterator<Item = (&ArcStr, &BusPort)> {
        self.ports.bus_ports()
    }

    /// Retrieves a reference to the [`CellPort`] with id `id`.
    pub fn port(&self, id: impl Into<PortId>) -> std::result::Result<&CellPort, PortError> {
        self.ports.port(id)
    }

    /// Returns an iterator over the ports in the cell.
    #[inline]
    pub fn ports(&self) -> impl Iterator<Item = &CellPort> {
        self.ports.ports()
    }

    /// Returns a mutable iterator over the ports in the cell.
    #[inline]
    pub fn ports_mut(&mut self) -> impl Iterator<Item = &mut CellPort> {
        self.ports.ports_mut()
    }

    /// Adds a [`CellPort`] to the cell.
    pub fn add_port(&mut self, port: impl Into<CellPort>) -> Result<(), PortError> {
        debug_assert!(!self.is_frozen());
        self.ports.add_port(port)
    }

    /// Adds a [`CellPort`] to the cell with strategy [`PortConflictStrategy::Merge`].
    pub fn merge_port(&mut self, port: impl Into<CellPort>) {
        debug_assert!(!self.is_frozen());
        // Can unwrap error since merging ports should never cause an error.
        self.ports
            .add_port_with_strategy(port, PortConflictStrategy::Merge)
            .unwrap()
    }

    /// Adds a [`CellPort`] to the cell, resolving conflicts using the provided strategy.
    pub fn add_port_with_strategy(
        &mut self,
        port: impl Into<CellPort>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        debug_assert!(!self.is_frozen());
        self.ports
            .add_port_with_strategy(port, port_conflict_strategy)
    }

    /// Adds several [`CellPort`]s to the cell.
    pub fn add_ports(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
    ) -> Result<(), PortError> {
        debug_assert!(!self.is_frozen());
        self.ports.add_ports(ports)
    }

    /// Adds several [`CellPort`]s to the cell.
    pub fn add_ports_with_strategy(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        debug_assert!(!self.is_frozen());
        self.ports
            .add_ports_with_strategy(ports, port_conflict_strategy)
    }

    /// Returns an iterator over the blockages in the cell.
    pub fn blockages(&self) -> impl Iterator<Item = (LayerKey, &Vec<Shape>)> {
        self.blockages.iter().map(|(k, v)| (*k, v))
    }

    /// Adds a blockage to the cell.
    pub fn add_blockage(&mut self, layer: LayerKey, shapes: Vec<Shape>) {
        debug_assert!(!self.is_frozen());
        self.blockages.insert(layer, shapes);
    }

    /// Adds several blockages to the cell.
    pub fn add_blockages(&mut self, blockages: impl IntoIterator<Item = (LayerKey, Vec<Shape>)>) {
        debug_assert!(!self.is_frozen());
        for (layer, shapes) in blockages {
            self.add_blockage(layer, shapes);
        }
    }

    /// Creates a rectangular [`Bbox`] surrounding all elements in the layout.
    pub fn bbox(&self) -> Bbox {
        // Return the cached bbox, if it exists.
        // Note that the cache cannot be updated with the results
        // of this function, since we don't have a mutable reference to self.
        // We may have some form of interior mutability in the future.
        if let Some(ref cache) = self.cache {
            return cache.bbox;
        }
        let mut bbox = Bbox::empty();
        for elem in &self.elems {
            bbox = elem.inner.union(bbox);
        }
        for inst in &self.insts {
            let b = inst.bbox();
            if !b.is_empty() {
                let s = Shape::Rect(Rect { p0: b.p0, p1: b.p1 });
                bbox = s.union(bbox);
            }
        }
        bbox
    }

    /// Adds all elements, instances, annotations, ports, and blockages from another cell.
    pub fn add_cell_flattened(&mut self, cell: Arc<Cell>) -> crate::error::Result<()> {
        debug_assert!(!self.is_frozen());
        self.add_elements(cell.elems().cloned());
        self.add_instances(cell.insts().cloned());
        self.add_annotations(cell.annotations().cloned());
        self.add_ports(cell.ports().cloned())?;
        self.add_blockages(cell.blockages().map(|(k, v)| (k, v.clone())));
        Ok(())
    }

    /// Adds all elements, instances, annotations, ports, and blockages from another cell,
    /// resolving port conflicts with the provided strategy.
    pub fn add_cell_flattened_with_strategy(
        &mut self,
        cell: Arc<Cell>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> crate::error::Result<()> {
        debug_assert!(!self.is_frozen());
        self.add_elements(cell.elems().cloned());
        self.add_instances(cell.insts().cloned());
        self.add_annotations(cell.annotations().cloned());
        self.add_ports_with_strategy(cell.ports().cloned(), port_conflict_strategy)?;
        self.add_blockages(cell.blockages().map(|(k, v)| (k, v.clone())));
        Ok(())
    }

    /// Trims the cell so that it lies within `bounds`.
    ///
    /// The current implementation does not trim blockages,
    /// but this is subject to change.
    ///
    /// # Panics
    ///
    /// Panics if the cell has any [`Instance`]s.
    /// Instances can be removed by [flattening](Cell::flatten)
    /// prior to trimming.
    pub fn trim<T>(&mut self, bounds: &T)
    where
        T: ?Sized,
        Element: Trim<T, Output = Element>,
        Shape: Trim<T, Output = Shape>,
        CellPort: Trim<T, Output = CellPort>,
        TextElement: Trim<T, Output = TextElement>,
    {
        debug_assert!(!self.is_frozen());

        // Instances cannot be trimmed
        assert!(self.insts.is_empty(), "must flatten Cell before trimming");

        // Trim elements
        let elems = std::mem::take(&mut self.elems);
        self.elems = elems.into_iter().filter_map(|e| e.trim(bounds)).collect();

        // Trim annotations
        let annotations = std::mem::take(&mut self.annotations);
        self.annotations = annotations
            .into_iter()
            .filter_map(|a| a.trim(bounds))
            .collect();

        self.ports = PortMap::from_map(HashMap::from_iter(self.ports.bus_ports().filter_map(
            |(k, bus)| {
                let new_bus: HashMap<usize, CellPort> = HashMap::from_iter(
                    bus.iter()
                        .filter_map(|(index, port)| port.trim(bounds).map(|port| (*index, port))),
                );
                if new_bus.is_empty() {
                    None
                } else {
                    Some((k.clone(), new_bus))
                }
            },
        )));
    }

    /// Sets the origin of the cell to the desired location.
    pub fn set_origin(&mut self, origin: Point) {
        let translation = Point::zero() - origin;
        self.translate(translation);
    }

    /// Freezes the cell, caching useful values and preventing further modification.
    #[inline]
    pub(crate) fn freeze(&mut self) {
        self.metadata.freeze();
        self.compute_cache();
    }

    /// Returns true if the cell is frozen.
    ///
    /// See [`Cell::freeze`] for more information.
    #[inline]
    fn is_frozen(&self) -> bool {
        self.cache.is_some()
    }

    /// (Re)computes cached values and updates the cache.
    fn compute_cache(&mut self) {
        let cache = Cache { bbox: self.bbox() };
        self.cache = Some(cache);
    }

    pub fn validate(&self) -> crate::error::Result<()> {
        let validation = validate_cell(self);
        validation.log();
        if validation.has_errors() {
            return Err(ErrorSource::InvalidLayout(validation.first_error()).into());
        }
        Ok(())
    }

    /// The instances of the cell, as a slice.
    ///
    /// Prefer to use the [`Cell::insts`] function where possible.
    pub(crate) fn _insts(&self) -> &[Instance] {
        &self.insts
    }

    pub fn set_metadata<T: Send + Sync + 'static>(&mut self, data: T) -> bool {
        self.metadata.set(data)
    }

    pub fn get_metadata<T: Send + Sync + 'static>(&self) -> &T {
        self.metadata.get::<T>()
    }

    pub fn shapes_on(&self, layer: LayerKey) -> Box<dyn Iterator<Item = Shape> + '_> {
        let recur = self.insts().flat_map(move |inst| inst.shapes_on(layer));
        let curr = self
            .elems()
            .filter(move |&elem| elem.layer.layer() == layer)
            .map(|elem| elem.inner.clone());
        Box::new(curr.chain(recur))
    }
}

impl Translate for Cell {
    fn translate(&mut self, p: Point) {
        debug_assert!(!self.is_frozen());

        for inst in self.insts.iter_mut() {
            inst.translate(p);
        }
        for elem in self.elems.iter_mut() {
            elem.translate(p);
        }
        for ann in self.annotations.iter_mut() {
            ann.translate(p);
        }
        for port in self.ports_mut() {
            port.translate(p);
        }
        for shapes in self.blockages.values_mut() {
            for s in shapes.iter_mut() {
                s.translate(p);
            }
        }
    }
}

impl Flatten for Cell {
    /// Flattens this cell, recursively replacing any [`Instance`]s with their contents.
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

impl BoundBox for Cell {
    fn bbox(&self) -> Bbox {
        let mut bbox = Bbox::empty();
        for elem in &self.elems {
            bbox = elem.inner.union(bbox);
        }
        for inst in &self.insts {
            let b = inst.bbox();
            if !b.is_empty() {
                let r = b.into_rect();
                bbox = r.union(bbox);
            }
        }
        bbox
    }
}

impl LayerBoundBox for Cell {
    fn layer_bbox(&self, layer: LayerKey) -> Bbox {
        let mut bbox = Bbox::empty();
        for elem in &self.elems {
            if elem.layer.layer() == layer {
                bbox = elem.inner.union(bbox);
            }
        }
        for inst in &self.insts {
            let b = inst.layer_bbox(layer);
            if !b.is_empty() {
                let r = b.into_rect();
                bbox = r.union(bbox);
            }
        }
        bbox
    }
}

impl CellPort {
    /// Create a new [`CellPort`] with the given `id`
    pub fn new(id: impl Into<PortId>) -> Self {
        Self {
            id: id.into(),
            shapes: HashMap::new(),
            must_connect: Default::default(),
        }
    }

    #[inline]
    pub fn builder() -> CellPortBuilder {
        CellPortBuilder::new()
    }

    pub fn with_shape(id: impl Into<PortId>, layer: LayerKey, shape: impl Into<Shape>) -> Self {
        let mut shapes = HashMap::with_capacity(1);
        shapes.insert(layer, vec![shape.into()]);
        Self {
            id: id.into(),
            shapes,
            must_connect: Default::default(),
        }
    }

    pub fn with_element(id: impl Into<PortId>, elem: Element) -> Self {
        let mut shapes = HashMap::with_capacity(1);
        shapes.insert(elem.layer.layer(), vec![elem.into_inner()]);
        Self {
            id: id.into(),
            shapes,
            must_connect: Default::default(),
        }
    }

    pub fn named(mut self, name: impl Into<ArcStr>) -> Self {
        self.id.name = name.into();
        self
    }

    pub fn map_index(mut self, mut map_fn: impl FnMut(usize) -> usize) -> Self {
        self.id.index = map_fn(self.id.index);
        self
    }

    pub fn with_index(mut self, index: usize) -> Self {
        self.id.index = index;
        self
    }

    pub fn with_id(mut self, id: impl Into<PortId>) -> Self {
        self.set_id(id);
        self
    }

    /// Returns the ID of the port.
    #[inline]
    pub fn id(&self) -> &PortId {
        &self.id
    }

    /// Returns the name of the port.
    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.id.name
    }

    /// Adds a shape to the port on layer `layer`.
    pub fn add(&mut self, layer: LayerKey, shape: Shape) -> &mut Self {
        let v = self
            .shapes
            .entry(layer)
            .or_insert_with(|| Vec::with_capacity(1));
        v.push(shape);
        self
    }

    /// Adds a shape to the port on layer `layer`.
    pub fn add_all(&mut self, layer: LayerKey, shapes: impl Iterator<Item = Shape>) -> &mut Self {
        for shape in shapes {
            self.add(layer, shape);
        }
        self
    }

    /// Sets the ID of the port.
    pub fn set_id(&mut self, id: impl Into<PortId>) -> &mut Self {
        self.id = id.into();
        self
    }

    /// Adds the shapes of `other` to this [`CellPort`].
    pub fn merge(&mut self, other: impl Into<Self>) {
        let other = other.into();
        for (k, mut v) in other.shapes.into_iter() {
            let shapes = self
                .shapes
                .entry(k)
                .or_insert_with(|| Vec::with_capacity(v.len()));
            shapes.append(&mut v);
        }

        for shapes in self.shapes.values_mut() {
            let mut rects: Vec<Rect> = shapes.iter().filter_map(|shape| shape.as_rect()).collect();
            let mut i = 0;
            while i < rects.len() {
                let mut j = i + 1;
                while j < rects.len() {
                    let rect1 = rects[i];
                    let rect2 = rects[j];
                    for dir in [Dir::Horiz, Dir::Vert] {
                        if rect1.span(dir) == rect2.span(dir)
                            && rect1.span(!dir).intersects(&rect2.span(!dir))
                        {
                            rects[i] = rect1.union(rect2.bbox()).into_rect();
                            rects.swap_remove(j);
                            j = i + 1;
                            break;
                        }
                    }
                    j += 1;
                }
                i += 1;
            }
            *shapes = shapes
                .iter()
                .filter_map(|shape| {
                    if let Shape::Rect(_) = shape {
                        None
                    } else {
                        Some(shape.clone())
                    }
                })
                .chain(rects.iter().map(|rect| Shape::Rect(*rect)))
                .collect();
        }
    }

    /// Adds the shapes of `other` to this [`CellPort`].
    ///
    /// Equivalent to [`CellPort::merge`], but may allow for easier chaining in some contexts.
    #[inline]
    pub fn merged_with(mut self, other: impl Into<Self>) -> Self {
        self.merge(other);
        self
    }

    pub fn set_must_connect(&mut self, must_connect: impl Into<MustConnect>) {
        self.must_connect = must_connect.into();
    }

    pub fn with_must_connect(mut self, must_connect: impl Into<MustConnect>) -> Self {
        self.set_must_connect(must_connect);
        self
    }

    pub fn shapes(&self, layer: LayerKey) -> impl Iterator<Item = &Shape> {
        self.shapes
            .get(&layer)
            .map(|shapes| shapes.iter())
            .unwrap_or([].iter())
    }
}

impl<T> Trim<T> for CellPort
where
    Shape: Trim<T, Output = Shape>,
{
    type Output = Self;
    fn trim(&self, bounds: &T) -> Option<Self::Output> {
        let iter = self
            .shapes
            .iter()
            .map(|(layer, shapes)| {
                (
                    *layer,
                    shapes
                        .iter()
                        .filter_map(|shape| shape.trim(bounds))
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, v)| !v.is_empty());
        let shapes = HashMap::from_iter(iter);
        if shapes.is_empty() {
            return None;
        }
        Some(Self {
            id: self.id.clone(),
            shapes,
            must_connect: self.must_connect.clone(),
        })
    }
}

/// A common interface for layout ports.
///
/// There are 2 implementors of `Port`: [`CellPort`] and [`TransformedPort`].
/// `CellPort` references a port in a cell; `TransformedPort` references a
/// port on a cell **instance**, which may have been rotated, translated,
/// and/or reflected.
pub trait Port {
    type ShapeIter: Iterator<Item = Shape>;
    type LayerIter: Iterator<Item = LayerKey>;

    /// Returns the ID of the [`Port`].
    fn id(&self) -> &PortId;

    /// Returns an iterator over the [`Shape`]s in the port.
    fn shapes(&self, layer: LayerKey) -> Self::ShapeIter;

    /// Returns an iterator over the [`LayerKey`]s in the port.
    fn layers(&self) -> Self::LayerIter;

    /// Get any layer with geometry in this port.
    ///
    /// It is guaranteed that there is at least one shape on the given layer in this port.
    ///
    /// # Panics
    ///
    /// This function panics if the port is empty, ie. there are no layers with geometry.
    fn any_layer(&self) -> LayerKey {
        self.layers()
            .find(|&layer| self.shapes(layer).next().is_some())
            .unwrap()
    }

    /// Returns the largest rectangle contained in the port.
    fn largest_rect(&self, layer: LayerKey) -> Result<Rect, PortError> {
        let shapes = self.shapes(layer);
        let mut best = None;
        let mut best_area = 0;
        for s in shapes {
            let r = match s {
                Shape::Rect(r) => r,
                _ => continue,
            };
            let area = r.area();
            if area > best_area {
                best_area = area;
                best = Some(r)
            }
        }
        best.ok_or(PortError::LayerNotPresent(layer))
    }

    /// Returns the first rectangle in the given direction contained in the port.
    fn first_rect(&self, layer: LayerKey, side: Side) -> Result<Rect, PortError> {
        let shapes = self.shapes(layer);
        let mut best = None;
        let mut best_coord = match side {
            Side::Bot | Side::Left => i64::MAX,
            Side::Top | Side::Right => i64::MIN,
        };
        for s in shapes {
            let r = match s {
                Shape::Rect(r) => r,
                _ => continue,
            };
            let coord_start = r.side(side);
            match side {
                Side::Bot | Side::Left => {
                    if coord_start < best_coord {
                        best_coord = coord_start;
                        best = Some(r)
                    }
                }
                Side::Top | Side::Right => {
                    if coord_start > best_coord {
                        best_coord = coord_start;
                        best = Some(r)
                    }
                }
            }
        }
        best.ok_or(PortError::LayerNotPresent(layer))
    }

    /// Returns the bounding box of the port on layer `layer`.
    fn bbox(&self, layer: LayerKey) -> Bbox {
        let mut bbox = Bbox::empty();
        for s in self.shapes(layer) {
            bbox = s.union(bbox);
        }
        bbox
    }
}

impl<I> Iterator for TransformedPortIterator<I>
where
    I: Iterator<Item = Shape>,
{
    type Item = Shape;

    fn next(&mut self) -> Option<Self::Item> {
        let shape = self.inner.next()?.transform(self.transformation);
        Some(shape)
    }
}

impl<'a, P> TransformedPort<'a, P>
where
    P: Port,
{
    pub fn into_cell_port(self) -> CellPort {
        self.into()
    }
}

impl<'a, P> Port for TransformedPort<'a, P>
where
    P: Port,
{
    type ShapeIter = TransformedPortIterator<P::ShapeIter>;
    type LayerIter = P::LayerIter;

    fn shapes(&self, layer: LayerKey) -> Self::ShapeIter {
        TransformedPortIterator {
            inner: self.inner.shapes(layer),
            transformation: self.transformation,
        }
    }

    #[inline]
    fn layers(&self) -> Self::LayerIter {
        self.inner.layers()
    }

    #[inline]
    fn id(&self) -> &PortId {
        self.inner.id()
    }
}

impl<'a> Port for &'a CellPort {
    type ShapeIter = std::iter::Cloned<std::slice::Iter<'a, Shape>>;
    type LayerIter = std::iter::Copied<std::collections::hash_map::Keys<'a, LayerKey, Vec<Shape>>>;

    fn id(&self) -> &PortId {
        &self.id
    }

    fn shapes(&self, layer: LayerKey) -> Self::ShapeIter {
        self._shapes(layer)
    }
    fn layers(&self) -> Self::LayerIter {
        self.shapes.keys().copied()
    }
}

impl Port for CellPort {
    type ShapeIter = std::vec::IntoIter<Shape>;
    type LayerIter = std::vec::IntoIter<LayerKey>;

    fn id(&self) -> &PortId {
        &self.id
    }

    fn shapes(&self, layer: LayerKey) -> Self::ShapeIter {
        let shapes = self.shapes.get(&layer).cloned().unwrap_or_default();
        shapes.into_iter()
    }
    fn layers(&self) -> Self::LayerIter {
        let layers = self.shapes.keys().copied().collect::<Vec<_>>();
        layers.into_iter()
    }
}

impl Instance {
    /// Creates a new [`Instance`].
    pub fn new(cell: impl Into<Arc<Cell>>) -> Self {
        let cell = cell.into();
        Self {
            name: cell.name.clone(),
            cell,
            loc: Point::new(0, 0),
            orientation: Orientation::default(),
        }
    }

    pub fn with_orientation(&self, o: impl Into<Orientation>) -> Self {
        let mut res = self.clone();
        res.set_orientation(o);
        res
    }

    /// Returns the name of the instance.
    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    /// Returns a pointer to the instance's reference cell.
    #[inline]
    pub fn cell(&self) -> &Arc<Cell> {
        &self.cell
    }

    /// Creates a new [`InstanceBuilder`].
    #[inline]
    pub fn builder() -> InstanceBuilder {
        InstanceBuilder::default()
    }

    /// Returns the transformation associated with the instance.
    #[inline]
    pub fn transformation(&self) -> Transformation {
        Transformation::with_loc_and_orientation(self.loc, self.orientation)
    }

    /// Returns the location of the instance.
    #[inline]
    pub fn loc(&self) -> Point {
        self.loc
    }

    /// Sets the location of the instance.
    #[inline]
    pub fn set_loc(&mut self, p: impl Into<Point>) {
        self.loc = p.into();
    }

    /// Returns the orientation of the instance.
    #[inline]
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Returns a mutable reference to the orientation of the instance.
    #[inline]
    pub fn orientation_mut(&mut self) -> &mut Orientation {
        &mut self.orientation
    }

    /// Sets the orientation of the instance.
    #[inline]
    pub fn set_orientation(&mut self, o: impl Into<Orientation>) {
        self.orientation = o.into();
    }

    /// Returns a port with id `id`.
    pub fn port(
        &self,
        id: impl Into<PortId>,
    ) -> std::result::Result<TransformedPort<CellPort>, PortError> {
        let port = self.cell.port(id)?;
        Ok(TransformedPort {
            transformation: self.transformation(),
            inner: port,
        })
    }

    /// Returns ports with names starting with `name`.
    pub fn ports_starting_with<'a>(&'a self, prefix: &str) -> impl Iterator<Item = CellPort> + 'a {
        self.ports()
            .filter(|port| port.id.name.starts_with(prefix))
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// Returns a vector of [`CellPort`]s associated with the instance.
    pub fn ports(&self) -> impl Iterator<Item = CellPort> + '_ {
        self.cell
            .ports()
            .map(|port| self.port(port.id.clone()).unwrap().into_cell_port())
    }

    /// Reflects the instance vertically without modifying its bounding box.
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

    /// Reflects the instance horizontally without modifying its bounding box.
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

    pub fn shapes_on(&self, layer: LayerKey) -> impl Iterator<Item = Shape> + '_ {
        let tf = self.transformation();
        self.cell().shapes_on(layer).map(move |s| s.transform(tf))
    }

    #[inline]
    pub fn add_to(self, ctx: &mut LayoutCtx) -> crate::error::Result<()> {
        ctx.draw(self)
    }

    #[inline]
    pub fn add_to_ref(&self, ctx: &mut LayoutCtx) -> crate::error::Result<()> {
        ctx.draw_ref(self)
    }
}

impl BoundBox for Instance {
    fn bbox(&self) -> Bbox {
        let bbox = self.cell.bbox();
        if bbox.is_empty() {
            return bbox;
        }

        bbox.into_rect().transform(self.transformation()).bbox()
    }
}

impl LayerBoundBox for Instance {
    fn layer_bbox(&self, layer: LayerKey) -> Bbox {
        let bbox = self.cell.layer_bbox(layer);
        if bbox.is_empty() {
            return bbox;
        }

        bbox.into_rect().transform(self.transformation()).bbox()
    }
}

impl Translate for Instance {
    fn translate(&mut self, p: Point) {
        self.loc.translate(p);
    }
}

impl Transform for Instance {
    fn transform(&self, trans: Transformation) -> Self {
        let mut value = self.clone();
        let trans = Transformation::cascade(trans, self.transformation());
        value.orientation = trans.orientation();
        value.loc = trans.offset_point();
        value
    }
}

impl AlignRect for Instance {}

impl<'a> CellPort {
    /// Returns the shapes associated with layer `layer` in the port.
    fn _shapes(&'a self, layer: LayerKey) -> std::iter::Cloned<std::slice::Iter<'a, Shape>> {
        let v = self.shapes.get(&layer).unwrap();
        let i = v.iter().cloned();
        i
    }
}

impl<'a, P> From<TransformedPort<'a, P>> for CellPort
where
    P: Port,
{
    fn from(value: TransformedPort<'a, P>) -> Self {
        let shapes = value
            .layers()
            .map(|layer| (layer, value.shapes(layer).collect::<Vec<_>>()))
            .collect();
        Self {
            id: value.id().clone(),
            shapes,
            must_connect: Default::default(),
        }
    }
}

/// Trait for removing all hierarchy in an object.
pub trait Flatten {
    fn flatten(&mut self);
}

/// A helper function for flattening.
///
/// For each instance in the given list, computes the composition of
/// the given transformation and the instance's transformation.
/// This transformation is applied to each element in the instance,
/// and the resulting [`Element`] is added to `out`.
///
/// Finally, this recurses on any [`Instance`]s contained within each [`Instance`].
pub(crate) fn flatten_recur(
    elts: &mut Vec<Element>,
    annotations: &mut Vec<TextElement>,
    tx: Transformation,
    insts: &[Instance],
) {
    for inst in insts {
        let tx = Transformation::cascade(tx, inst.transformation());
        for elem in inst.cell.elems() {
            elts.push(elem.transform(tx));
        }
        for elem in inst.cell.annotations() {
            annotations.push(elem.transform(tx));
        }
        flatten_recur(elts, annotations, tx, inst.cell._insts());
    }
}

impl From<&Instance> for Instance {
    fn from(value: &Instance) -> Self {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use slotmap::SlotMap;

    use super::*;

    #[test]
    fn test_freeze_creates_cache() {
        let mut map: SlotMap<CellKey, ()> = SlotMap::with_key();
        let key = map.insert(());

        let mut cell = Cell::new(key);

        assert!(!cell.is_frozen());
        assert_eq!(cell.cache, None);

        cell.freeze();

        assert!(cell.is_frozen());
        assert_eq!(
            cell.cache,
            Some(Cache {
                bbox: Bbox::empty(),
            })
        );
    }
}
