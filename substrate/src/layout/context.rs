//! Context methods for instantiating component layouts.

use std::sync::Arc;

use subgeom::bbox::{Bbox, BoundBox};
use subgeom::trim::Trim;
use subgeom::{Point, Rect, Shape};

use super::cell::{
    Cell, CellKey, CellPort, Element, Flatten, Instance, PortConflictStrategy, PortError,
    TextElement,
};
use super::group::Group;
use super::layers::{LayerPurpose, LayersRef, UserLayer};
use super::{Draw, DrawRef};
use crate::component::Component;
use crate::data::SubstrateCtx;
use crate::deps::arcstr::ArcStr;
use crate::error::Result as SubResult;
use crate::generation::{GeneratedCheck, GenerationMap, ParamKey};
use crate::pdk::mos::db::MosDb;
use crate::pdk::Pdk;
use crate::units::SiPrefix;

/// Context for creating the layout view of a [`Component`].
pub struct LayoutCtx {
    /// The global [`SubstrateCtx`].
    pub(crate) inner: SubstrateCtx,
    /// The layout view of the cell being created.
    pub(crate) cell: Cell,
}

/// Cell data for a Substrate layout.
#[derive(Debug)]
pub(crate) struct LayoutData {
    /// The units used for lengths.
    units: SiPrefix,
    /// A map of generated and imported cells.
    cells: GenerationMap<ParamKey, CellKey, Cell>,
}

impl LayoutData {
    /// Creates a new [`LayoutData`] from a [`Pdk`].
    #[inline]
    pub(crate) fn new(pdk: Arc<dyn Pdk>) -> Self {
        Self {
            units: pdk.lengths().layout,
            cells: GenerationMap::new(),
        }
    }

    /// Returns the units associated with lengths in the layout.
    #[inline]
    pub(crate) fn units(&self) -> SiPrefix {
        self.units
    }

    /// Returns the ID of a generated cell if it already exists or generates a new ID.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn get_generated_id<T>(
        &mut self,
        params: &T::Params,
    ) -> GeneratedCheck<CellKey, CellKey>
    where
        T: Component,
    {
        self.cells.get_id(ParamKey::from_params::<T>(params))
    }

    /// Returns the generated cell if it already exists or generates a new ID.
    #[allow(dead_code)]
    pub(crate) fn get_generated_cell<T>(
        &mut self,
        params: &T::Params,
    ) -> GeneratedCheck<Arc<Cell>, CellKey>
    where
        T: Component,
    {
        self.cells.get(ParamKey::from_params::<T>(params))
    }

    /// Adds a cell to the map based on its [`CellKey`].
    pub(crate) fn set_cell(&mut self, cell: Cell) -> Arc<Cell> {
        self.cells.set(cell.id(), cell.name().clone(), cell)
    }

    /// Gets a cell from the map based on its [`CellKey`].
    pub(crate) fn get_by_id(&self, id: CellKey) -> SubResult<&Arc<Cell>> {
        self.cells.get_by_id(id)
    }

    /// Generates a new [`CellKey`] to allow for a new cell to be created.
    pub(crate) fn gen_id(&mut self) -> CellKey {
        self.cells.gen_id()
    }

    /// Allocates an unused name derived from the given base name.
    pub(crate) fn alloc_name(&self, base_name: impl Into<ArcStr>) -> ArcStr {
        self.cells.alloc_name(base_name)
    }

    /// Returns an iterator over the cells in the map.
    pub(crate) fn cells(&self) -> impl Iterator<Item = &Arc<Cell>> {
        self.cells.values()
    }
}

impl LayoutCtx {
    /// Returns a reference to the global [`SubstrateCtx`].
    #[inline]
    pub fn inner(&self) -> &SubstrateCtx {
        &self.inner
    }

    /// Returns an iterator over the [`Element`]s in the current cell.
    pub fn elems(&self) -> impl Iterator<Item = &Element> {
        self.cell.elems()
    }

    /// Returns a reference to the underlying [`Pdk`].
    #[inline]
    pub fn pdk(&self) -> Arc<dyn Pdk> {
        self.inner.pdk()
    }

    /// Returns a reference to the underlying PDK's [`MosDb`].
    #[inline]
    pub fn mos_db(&self) -> Arc<MosDb> {
        self.inner.mos_db()
    }

    /// Returns a reference to the underlying PDK's layer manager as a [`LayersRef`].
    #[inline]
    pub fn layers(&self) -> LayersRef {
        LayersRef::new(self.inner.read().layers())
    }

    /// Instantiates a layout instance of component `T` with params `params`.
    #[inline]
    pub fn instantiate<T>(&mut self, params: &T::Params) -> SubResult<Instance>
    where
        T: Component,
    {
        self.inner.instantiate_layout::<T>(params)
    }

    /// Draws a rectangle on layer `layer` of the layout.
    pub fn draw_rect<L>(&mut self, layer: L, rect: Rect)
    where
        L: Into<UserLayer>,
    {
        self.cell
            .draw_rect(layer.into().to_spec(LayerPurpose::Drawing), rect)
    }

    pub fn bbox(&self) -> Bbox {
        self.cell.bbox()
    }

    /// The bounding [`Rect`] of the current cell.
    ///
    /// # Panics
    ///
    /// This function may panic if the bounding box is empty.
    pub fn brect(&self) -> Rect {
        self.cell.brect()
    }

    pub fn draw<T>(&mut self, value: T) -> SubResult<()>
    where
        T: Draw,
    {
        let group = value.draw()?;
        self.add_group(group);
        Ok(())
    }

    pub fn draw_ref<T>(&mut self, value: &T) -> SubResult<()>
    where
        T: DrawRef,
    {
        let group = value.draw_ref()?;
        self.add_group(group);
        Ok(())
    }

    pub fn draw_all<T>(&mut self, values: impl IntoIterator<Item = T>) -> SubResult<()>
    where
        T: Draw,
    {
        for value in values.into_iter() {
            self.draw(value)?;
        }
        Ok(())
    }

    pub(crate) fn add_group(&mut self, group: Group) {
        self.add_elements(group.elements());
        self.add_instances(group.instances());
        self.add_annotations(group.annotations());
    }

    /// Flattens the current cell.
    ///
    /// See [`Flatten::flatten`] for more information.
    #[inline]
    pub fn flatten(&mut self) {
        self.cell.flatten();
    }

    /// Trims the current cell.
    #[inline]
    pub fn trim<T>(&mut self, bounds: &T)
    where
        T: ?Sized,
        Element: Trim<T, Output = Element>,
        Shape: Trim<T, Output = Shape>,
        CellPort: Trim<T, Output = CellPort>,
        TextElement: Trim<T, Output = TextElement>,
    {
        self.cell.trim::<T>(bounds);
    }

    /// Sets the origin of the current cell to the given [`Point`].
    ///
    /// This is implemented by translating the contents of the cell appropriately.
    #[inline]
    pub fn set_origin(&mut self, origin: Point) {
        self.cell.set_origin(origin);
    }

    /// Adds all elements from the given iterator to this cell.
    pub(crate) fn add_elements(&mut self, elements: impl IntoIterator<Item = Element>) {
        self.cell.add_elements(elements);
    }

    /// Adds all instances from the given iterator to this cell.
    pub(crate) fn add_instances(&mut self, instances: impl IntoIterator<Item = Instance>) {
        self.cell.add_instances(instances);
    }

    /// Adds all annotations from the given iterator to this cell.
    pub(crate) fn add_annotations(&mut self, annotations: impl IntoIterator<Item = TextElement>) {
        self.cell.add_annotations(annotations);
    }

    /// Adds a [`CellPort`] to the cell.
    pub fn add_port(&mut self, port: impl Into<CellPort>) -> Result<(), PortError> {
        self.cell.add_port(port)
    }

    /// Adds a [`CellPort`] to the cell, merging if a port with the same name already exists.
    pub fn merge_port(&mut self, port: impl Into<CellPort>) {
        self.cell.merge_port(port)
    }

    /// Adds a [`CellPort`] to the cell, resolving conflicts with the provided strategy.
    pub fn add_port_with_strategy(
        &mut self,
        port: impl Into<CellPort>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        self.cell
            .add_port_with_strategy(port, port_conflict_strategy)
    }

    /// Adds several [`CellPort`]s to the cell.
    pub fn add_ports(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
    ) -> Result<(), PortError> {
        self.cell.add_ports(ports)
    }

    /// Adds several [`CellPort`]s to the cell, resolving conflicts with the provided strategy.
    pub fn add_ports_with_strategy(
        &mut self,
        ports: impl IntoIterator<Item = impl Into<CellPort>>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> Result<(), PortError> {
        self.cell
            .add_ports_with_strategy(ports, port_conflict_strategy)
    }

    /// Adds elements, instances, and annotations from cell.
    pub fn add_cell_flattened(&mut self, cell: Arc<Cell>) -> crate::error::Result<()> {
        self.cell.add_cell_flattened(cell)
    }

    /// Adds elements, instances, and annotations from cell, resolving port conflicts with the
    /// provided strategy.
    pub fn add_cell_flattened_with_strategy(
        &mut self,
        cell: Arc<Cell>,
        port_conflict_strategy: PortConflictStrategy,
    ) -> crate::error::Result<()> {
        self.cell
            .add_cell_flattened_with_strategy(cell, port_conflict_strategy)
    }

    pub fn set_metadata<T: Send + Sync + 'static>(&mut self, data: T) -> bool {
        self.cell.set_metadata(data)
    }

    pub fn get_metadata<T: Send + Sync + 'static>(&self) -> &T {
        self.cell.get_metadata::<T>()
    }
}
