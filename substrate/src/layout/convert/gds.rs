//! Utilities for GDS conversion.
//!
//! Converts between Substrate's layout data-model and [`gds21`] structures.

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use std::sync::{Arc, RwLock};

use derivative::Derivative;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use subgeom::bbox::BoundBox;
use subgeom::orientation::Orientation;
use subgeom::{Dir, Path, Point, Polygon, Rect, Shape, ShapeTrait};

use super::error::{ErrorContext, ErrorHelper};
use crate::data::{SubstrateCtx, SubstrateData};
use crate::deps::arcstr::ArcStr;
use crate::error::{
    with_err_context, ErrorContext as SubErrorContext, ErrorSource, Result as SubResult,
};
use crate::fmt::signal::BusFmt;
use crate::layout::cell::{BusPort, Cell, CellKey, CellPort, Element, Instance, TextElement};
use crate::layout::context::LayoutCtx;
use crate::layout::error::{LayoutError, LayoutResult};
use crate::layout::layers::{GdsLayerSpec, LayerInfo, LayerKey, LayerPurpose, LayerSpec, Layers};
use crate::units::SiPrefix;

new_key_type! {
    /// A unique identifier for imported [`Element`]s.
    pub struct ElementKey;
}

#[derive(Debug, Clone, Default)]
enum ExportSet {
    #[default]
    All,
    Set(HashSet<CellKey>),
}

/// A GDSII exporter.
///
/// Converts Substrate layout data to a GDSII library ([`gds21::GdsLibrary`]).
#[derive(Derivative)]
#[derivative(Debug)]
pub struct GdsExporter<'a> {
    #[derivative(Debug = "ignore")]
    data: &'a SubstrateData,
    layers: Arc<RwLock<Layers>>,
    backtrace: Vec<ErrorContext>,
    names_used: HashSet<ArcStr>,
    /// The top level cell.
    ///
    /// The name of this cell will be preserved.
    top: Option<Arc<Cell>>,
    export_set: ExportSet,
    names: SecondaryMap<CellKey, ArcStr>,
}

/// A GDSII importer.
///
/// Imports cells from a  [`gds21::GdsLibrary`] into Substrate.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct GdsImporter<'a> {
    #[derivative(Debug = "ignore")]
    data: &'a mut SubstrateData,
    layers: &'a mut Layers,
    backtrace: Vec<ErrorContext>,
    unsupported: Vec<gds21::GdsElement>,
    cell_map: HashMap<ArcStr, Arc<Cell>>,
}

/// Additional [`SubstrateCtx`] methods for GDSII conversion.
impl SubstrateCtx {
    /// Converts the context to a GDSII library.
    pub fn to_gds_lib(&self) -> SubResult<gds21::GdsLibrary> {
        let data = self.read();
        let inner = || -> SubResult<gds21::GdsLibrary> {
            let data = GdsExporter {
                data: &data,
                layers: data.layers(),
                backtrace: Vec::new(),
                names_used: HashSet::with_capacity(data.layouts().cells().count()),
                top: None,
                export_set: ExportSet::All,
                names: SecondaryMap::new(),
            }
            .export_lib()
            .map_err(ErrorSource::Layout)?;
            Ok(data)
        };
        with_err_context(inner(), || {
            SubErrorContext::Task(arcstr::literal!(
                "converting cells in context to GDS library"
            ))
        })
    }
    /// Converts the context to a GDSII library.
    pub(crate) fn to_gds_lib_with_top(&self, top: Arc<Cell>) -> SubResult<gds21::GdsLibrary> {
        let data = self.read();
        let inner = || -> SubResult<gds21::GdsLibrary> {
            Ok(GdsExporter {
                data: &data,
                layers: data.layers(),
                backtrace: Vec::new(),
                names_used: HashSet::with_capacity(data.layouts().cells().count()),
                export_set: ExportSet::for_top(&top),
                top: Some(top),
                names: SecondaryMap::new(),
            }
            .export_lib()
            .map_err(ErrorSource::Layout)?)
        };
        with_err_context(inner(), || {
            SubErrorContext::Task(arcstr::literal!("converting top cell to GDS library"))
        })
    }
    /// Saves the context to a GDS file.
    pub fn to_gds(&self, path: impl AsRef<std::path::Path>) -> SubResult<()> {
        let inner = || -> SubResult<()> {
            self.to_gds_lib()?
                .save(path)
                .map_err(LayoutError::from)
                .map_err(ErrorSource::Layout)?;
            Ok(())
        };
        with_err_context(inner(), || {
            SubErrorContext::Task(arcstr::literal!("converting cells in context to GDS"))
        })
    }
    /// Saves the context to a GDS file.
    pub(crate) fn to_gds_with_top(
        &self,
        top: Arc<Cell>,
        path: impl AsRef<std::path::Path>,
    ) -> SubResult<()> {
        let inner = || -> SubResult<()> {
            self.to_gds_lib_with_top(top)?
                .save(path)
                .map_err(LayoutError::from)
                .map_err(ErrorSource::Layout)?;
            Ok(())
        };
        with_err_context(inner(), || {
            SubErrorContext::Task(arcstr::literal!("converting top cell to GDS"))
        })
    }
    /// Adds cells from a GDSII library to the context.
    pub fn from_gds_lib(
        &self,
        gdslib: &gds21::GdsLibrary,
    ) -> SubResult<HashMap<ArcStr, Arc<Cell>>> {
        // Create the importer.
        let mut data = self.write();
        let layers = data.layers();
        let mut layers_guard = layers.write().unwrap();
        let mut importer = GdsImporter::new(&mut data, &mut layers_guard);
        // Run the main import method.
        importer.import_all(gdslib)?;
        // Destructure the result.
        let GdsImporter {
            unsupported,
            cell_map,
            ..
        } = importer;
        if !unsupported.is_empty() {
            println!(
                "Read {} Unsupported GDS Elements: {:?}",
                unsupported.len(),
                unsupported
            );
        }
        Ok(cell_map)
    }
    /// Adds cells from a GDS file to the context.
    pub fn from_gds(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> SubResult<HashMap<ArcStr, Arc<Cell>>> {
        let library = gds21::GdsLibrary::load(path)
            .map_err(LayoutError::from)
            .map_err(ErrorSource::Layout)?;
        self.from_gds_lib(&library)
    }

    /// Flat-import the cell named `cell_to_import` from a GDSII library into `cell`.
    pub fn from_gds_lib_flattened(
        &self,
        gdslib: &gds21::GdsLibrary,
        cell_to_import: &str,
        cell: &mut Cell,
    ) -> SubResult<()> {
        let mut data = self.write();
        let layers = data.layers();
        let mut layers_guard = layers.write().unwrap();
        let mut importer = GdsImporter::new(&mut data, &mut layers_guard);
        importer.import_cell_with_deps(gdslib, cell_to_import, cell)?;
        Ok(())
    }

    /// Flat-import the cell named `cell_to_import` from a GDS file into `cell`.
    pub fn from_gds_flattened(
        &mut self,
        path: impl AsRef<std::path::Path>,
        to_import: &str,
        cell: &mut Cell,
    ) -> SubResult<()> {
        let library = gds21::GdsLibrary::load(path)
            .map_err(LayoutError::from)
            .map_err(ErrorSource::Layout)?;
        self.from_gds_lib_flattened(&library, to_import, cell)
    }
}

/// Additional [`LayoutCtx`] methods for GDSII conversion.
impl LayoutCtx {
    /// Adds cells from a GDS file to the context.
    pub fn from_gds(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> SubResult<HashMap<ArcStr, Arc<Cell>>> {
        self.inner.from_gds(path)
    }

    /// Flat-import the cell named `cell_to_import` from a GDSII library into `cell`.
    pub fn from_gds_lib_flattened(
        &mut self,
        gdslib: &gds21::GdsLibrary,
        cell_to_import: &str,
    ) -> SubResult<()> {
        self.inner
            .from_gds_lib_flattened(gdslib, cell_to_import, &mut self.cell)
    }

    /// Flat-import the cell named `cell_to_import` from a GDS file into `cell`.
    pub fn from_gds_flattened(
        &mut self,
        path: impl AsRef<std::path::Path>,
        to_import: &str,
    ) -> SubResult<()> {
        self.inner
            .from_gds_flattened(path, to_import, &mut self.cell)
    }
}

impl<'a> GdsExporter<'a> {
    /// Runs basic preprocessing before export.
    fn prepare(&mut self) {
        if let Some(ref top) = self.top {
            self.names_used.insert(top.name().to_owned());
        }
        let layouts = self.data.layouts();
        for cell in layouts.cells() {
            let name = self.get_cell_name(cell);
            self.names.insert(cell.id(), name);
        }
    }
    /// Exports to a [`gds21::GdsLibrary`].
    fn export_lib(&mut self) -> LayoutResult<gds21::GdsLibrary> {
        self.prepare();

        self.backtrace.push(ErrorContext::Library);
        // Create a new GDS library.
        let mut gdslib = gds21::GdsLibrary::new("TOP".to_string());
        let layouts = self.data.layouts();

        // Set its distance units
        // In all cases the GDSII "user units" are set to 1µm.
        let units = layouts.units();
        gdslib.units = match units {
            SiPrefix::Micro => gds21::GdsUnits::new(1.0, 1e-6),
            SiPrefix::Nano => gds21::GdsUnits::new(1e-3, 1e-9),
            SiPrefix::Pico => gds21::GdsUnits::new(1e-6, 1e-12),
            _ => {
                return self.fail(format!("Invalid unit prefix for library: {units:?}"));
            }
        };
        // And convert each of our `cells` into its `structs`
        for cell in layouts.cells() {
            if !self.export_set.contains(&cell.id()) {
                continue;
            }
            let strukt = self.export_cell(cell.clone())?;
            gdslib.structs.push(strukt);
        }
        self.backtrace.pop();
        Ok(gdslib)
    }
    /// Converts a [`Cell`] to a [`gds21::GdsStruct`] cell definition.
    fn export_cell(&mut self, cell: Arc<Cell>) -> LayoutResult<gds21::GdsStruct> {
        self.backtrace.push(ErrorContext::Cell(cell.name().clone()));

        // Convert the primary implementation-data
        let mut elems = Vec::new();

        // Convert each [`Instance`]
        for inst in cell.insts() {
            elems.push(self.export_instance(inst)?.into());
        }

        // Convert each [`Element`]
        // Note each can produce more than one [GdsElement]
        self.backtrace.push(ErrorContext::Geometry);
        for elem in cell.elems() {
            elems.extend(self.export_element(elem)?);
        }
        self.backtrace.pop();

        // Convert each [`TextElement`]
        self.backtrace.push(ErrorContext::Annotations);
        for annotation in cell.annotations() {
            elems.push(self.export_annotation(annotation)?);
        }
        self.backtrace.pop();

        // Convert each [`CellPort`]
        self.backtrace.push(ErrorContext::Ports);
        for (_, bus) in cell.bus_ports() {
            elems.extend(self.export_bus(bus)?);
        }
        self.backtrace.pop();

        // Create and return a [`gds21::GdsStruct`]
        let mut strukt = gds21::GdsStruct::new(self.names[cell.id()].clone());
        strukt.elems = elems;

        self.backtrace.pop();
        Ok(strukt)
    }
    /// Converts an [`Instance`] to a GDS instance ([`gds21::GdsStructRef`]).
    fn export_instance(&mut self, inst: &Instance) -> LayoutResult<gds21::GdsStructRef> {
        self.backtrace
            .push(ErrorContext::Instance(inst.name().clone()));
        // Convert the orientation to a [gds21::GdsStrans] option
        let cell = inst.cell();
        let gdsinst = gds21::GdsStructRef {
            name: self.names[cell.id()].clone(),
            xy: self.export_point(&inst.loc())?,
            strans: inst.orientation().into(),
            ..Default::default()
        };
        self.backtrace.pop();
        Ok(gdsinst)
    }
    /// Converts a [`LayerSpec`] combination to a [`gds21::GdsLayerSpec`].
    pub fn export_layerspec(&mut self, spec: &LayerSpec) -> LayoutResult<gds21::GdsLayerSpec> {
        let layers = self.layers.read().unwrap();
        Ok(self
            .unwrap(
                layers.to_gds_spec(spec),
                format!("No GDS spec found for layer spec {spec:?}"),
            )?
            .into())
    }
    /// Converts a [`LayerKey`] to a [`gds21::GdsLayerSpec`] corresponding to labels for that layer.
    pub fn export_label_layerspec(&mut self, key: LayerKey) -> LayoutResult<gds21::GdsLayerSpec> {
        let layers = self.layers.read().unwrap();
        Ok(self
            .unwrap(
                layers.to_label_gds_spec(key),
                format!("No GDS spec found for layer spec {key:?}"),
            )?
            .into())
    }
    /// Converts an [`Element`] into one or more [`gds21::GdsElement`]s.
    ///
    pub fn export_element(&mut self, elem: &Element) -> LayoutResult<Vec<gds21::GdsElement>> {
        // Get the element's layer-numbers pair
        let layerspec = self.export_layerspec(&elem.layer)?;
        // Convert its core inner [Shape]
        let gds_elems = match self.export_shape(&elem.inner, &layerspec)? {
            Some(x) => vec![x],
            None => Vec::new(),
        };
        // FIXME: Find a way to store net labels in GDS
        Ok(gds_elems)
    }
    /// Converts a [`Shape`] to a [`gds21::GdsElement`].
    ///
    /// Layer and datatype must be previously converted to the [`gds21::GdsLayerSpec`] format.
    ///
    /// GDS shapes include an explicit repetition of their origin for closure.
    /// So an N-sided polygon is described by a (N+1)-point vector.
    pub fn export_shape(
        &mut self,
        shape: &Shape,
        layerspec: &gds21::GdsLayerSpec,
    ) -> LayoutResult<Option<gds21::GdsElement>> {
        let elem = match shape {
            Shape::Rect(r) => {
                let (p0, p1) = (&r.p0, &r.p1);
                let x0 = p0.x.try_into()?;
                let y0 = p0.y.try_into()?;
                let x1 = p1.x.try_into()?;
                let y1 = p1.y.try_into()?;
                let xy = gds21::GdsPoint::vec(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]);
                // Both rect and polygon map to [GdsBoundary], although [GdsBox] is also suitable here.
                gds21::GdsBoundary {
                    layer: layerspec.layer,
                    datatype: layerspec.xtype,
                    xy,
                    ..Default::default()
                }
                .into()
            }
            Shape::Polygon(poly) => {
                // Flatten our points-vec, converting to 32-bit along the way
                let mut xy = poly
                    .points
                    .iter()
                    .map(|p| self.export_point(p))
                    .collect::<Result<Vec<_>, _>>()?;
                // Add the origin a second time, to "close" the polygon
                xy.push(self.export_point(&poly.points[0])?);
                gds21::GdsBoundary {
                    layer: layerspec.layer,
                    datatype: layerspec.xtype,
                    xy,
                    ..Default::default()
                }
                .into()
            }
            Shape::Path(path) => {
                // Flatten our points-vec, converting to 32-bit along the way
                let mut xy = Vec::new();
                for p in path.points.iter() {
                    xy.push(self.export_point(p)?);
                }
                // Add the origin a second time, to "close" the polygon
                xy.push(self.export_point(&path.points[0])?);
                gds21::GdsPath {
                    layer: layerspec.layer,
                    datatype: layerspec.xtype,
                    width: Some(i32::try_from(path.width)?),
                    xy,
                    ..Default::default()
                }
                .into()
            }
            Shape::Point(_) => return Ok(None),
        };
        Ok(Some(elem))
    }
    /// Converts a [`TextElement`] to a [`gds21::GdsElement`].
    pub fn export_annotation(
        &mut self,
        text_elem: &TextElement,
    ) -> LayoutResult<gds21::GdsElement> {
        let layerspec = self.export_layerspec(&text_elem.layer)?;

        // And return a converted [GdsTextElem]
        Ok(gds21::GdsTextElem {
            string: text_elem.string.clone(),
            layer: layerspec.layer,
            texttype: layerspec.xtype,
            xy: self.export_point(&text_elem.loc)?,
            strans: None, // FIXME: Use text_elem's orientation.
            ..Default::default()
        }
        .into())
    }
    /// Converts a [`BusPort`] to its constituent [`gds21::GdsElement`]s.
    fn export_bus(&mut self, bus: &BusPort) -> LayoutResult<Vec<gds21::GdsElement>> {
        let width = bus.len();
        let mut elems = Vec::new();

        for port in bus.values() {
            for (key, shapes) in port.shapes.iter() {
                // FIXME: Add configurable layer purposes.
                let drawing_spec =
                    self.export_layerspec(&LayerSpec::new(*key, LayerPurpose::Drawing))?;
                let pin_spec = self.export_layerspec(&LayerSpec::new(*key, LayerPurpose::Pin))?;
                let label_spec = self.export_label_layerspec(*key)?;
                for shape in shapes {
                    if let Some(e) = self.export_shape(shape, &drawing_spec)? {
                        elems.push(e);
                    }
                    if let Some(e) = self.export_shape(shape, &pin_spec)? {
                        elems.push(e);
                    }
                    elems.push(
                        self.export_shape_label(
                            port.id
                                .format_signal(width, BusFmt::DoubleDelimiter('[', ']')),
                            shape,
                            &label_spec,
                        )?,
                    );
                }
            }
        }
        Ok(elems)
    }
    /// Creates a labeling [`gds21::GdsElement`] for [`Shape`] `shape`.
    pub fn export_shape_label(
        &mut self,
        net: ArcStr,
        shape: &Shape,
        layerspec: &gds21::GdsLayerSpec,
    ) -> LayoutResult<gds21::GdsElement> {
        // Sort out a location to place the text
        let loc = shape.label_location();

        // Rotate that text 90 degrees for mostly-vertical shapes
        let strans = match shape.orientation() {
            Dir::Horiz => None,
            Dir::Vert => Some(gds21::GdsStrans {
                angle: Some(90.0),
                ..Default::default()
            }),
        };
        // And return a converted [GdsTextElem]
        Ok(gds21::GdsTextElem {
            string: net,
            layer: layerspec.layer,
            texttype: layerspec.xtype,
            xy: self.export_point(&loc)?,
            strans,
            ..Default::default()
        }
        .into())
    }
    /// Convert a [`Point`] to a GDS21 [`gds21::GdsPoint`].
    pub fn export_point(&mut self, pt: &Point) -> LayoutResult<gds21::GdsPoint> {
        let x = pt.x.try_into()?;
        let y = pt.y.try_into()?;
        Ok(gds21::GdsPoint::new(x, y))
    }

    /// Renames the cell with the given name to avoid duplicate cell names.
    ///
    /// Does not rename the top cell. However, to ensure no other cell takes
    /// the top cell's name, the top cell name must be inserted into the
    /// `names_used` map prior to calling this function.
    fn get_cell_name(&mut self, cell: &Arc<Cell>) -> ArcStr {
        let name = cell.name();
        let name = if self.names_used.contains(name) && !self.is_top(cell) {
            let mut i = 1;
            loop {
                let newname = arcstr::format!("{}_{}", name, i);
                if !self.names_used.contains(&newname) {
                    break newname;
                }
                i += 1;
            }
        } else {
            name.clone()
        };

        self.names_used.insert(name.clone());
        name
    }

    /// Checks if `cell`'s ID matches the top cell's ID.
    fn is_top(&self, cell: &Arc<Cell>) -> bool {
        if let Some(ref top) = self.top {
            top.id() == cell.id()
        } else {
            false
        }
    }
}

impl ErrorHelper for GdsExporter<'_> {
    type Error = LayoutError;
    fn err(&self, msg: impl Into<String>) -> LayoutError {
        LayoutError::Export {
            message: msg.into(),
            stack: self.backtrace.clone(),
        }
    }
}

/// A trait for calculating the location of text-labels, generally per [`Shape`].
///
/// The sole function `label_location` calculates an appropriate location.
///
/// While Layout21 formats do not include "placed text", GDSII relies on it for connectivity annotations.
/// How to place these labels varies by shape type.
trait PlaceLabels {
    fn label_location(&self) -> Point;
}
impl PlaceLabels for Shape {
    fn label_location(&self) -> Point {
        // Dispatch based on shape-type
        match self {
            Shape::Rect(ref r) => r.label_location(),
            Shape::Polygon(ref p) => p.label_location(),
            Shape::Path(ref p) => p.label_location(),
            Shape::Point(ref p) => p.label_location(),
        }
    }
}
impl PlaceLabels for Point {
    fn label_location(&self) -> Point {
        *self
    }
}
impl PlaceLabels for Rect {
    fn label_location(&self) -> Point {
        // Place rectangle-labels in the center of the rectangle.
        self.center()
    }
}
impl PlaceLabels for Path {
    fn label_location(&self) -> Point {
        // Place on the center of the first segment.
        let p0 = &self.points[0];
        let p1 = &self.points[1];
        Point::new((p0.x + p1.x) / 2, (p0.y + p1.y) / 2)
    }
}
impl PlaceLabels for Polygon {
    fn label_location(&self) -> Point {
        // Priority 1: if the center of our bounding box lies within the polygon, use that.
        // In simple-polygon cases, this is most likely our best choice.
        // In many other cases, e.g. for "U-shaped" polygons, this will fall outside the polygon and be invalid.
        let bbox_center = self.points.bbox().center();
        if self.contains(bbox_center) {
            return bbox_center;
        }

        // Priority 2: try the four coordinates immediately above, below, left, and right of the polygon's first point.
        // If none work, just use one of the polygon's vertices.
        let pt0 = self.point0();
        let candidates = vec![
            Point::new(pt0.x, pt0.y - 1),
            Point::new(pt0.x - 1, pt0.y),
            Point::new(pt0.x, pt0.y + 1),
            Point::new(pt0.x + 1, pt0.y),
        ];
        for pt in candidates {
            if self.contains(pt) {
                return pt;
            }
        }
        pt0
    }
}

/// A helper for retrieving GDS dependencies in reverse topological order.
///
/// Creates a vector of references Gds structs, ordered by their instance dependencies.
/// Each item in the ordered return value is guaranteed *not* to instantiate any item which comes later.
#[derive(Debug)]
pub struct GdsDepOrder<'a> {
    strukts: HashMap<ArcStr, &'a gds21::GdsStruct>,
    stack: Vec<&'a gds21::GdsStruct>,
    seen: HashSet<ArcStr>,
}
impl<'a> GdsDepOrder<'a> {
    /// Creates a new [`GdsDepOrder`] for a [`gds21::GdsLibrary`].
    fn new(gdslib: &'a gds21::GdsLibrary) -> Self {
        // First create a map from names to structs
        let mut strukts = HashMap::new();
        for s in &gdslib.structs {
            strukts.insert(s.name.clone(), s);
        }
        Self {
            strukts,
            stack: Vec::new(),
            seen: HashSet::new(),
        }
    }
    /// Returns a reverse topological sort of all structs in `gdslib`.
    fn total_order(mut self) -> Vec<&'a gds21::GdsStruct> {
        let strukts = self
            .strukts
            .values()
            .copied()
            .collect::<Vec<&gds21::GdsStruct>>();
        for s in strukts {
            self.push(s)
        }
        self.stack
    }
    /// Returns all dependencies of a given cell in reverse topological order.
    fn cell_order(mut self, cell: impl Into<ArcStr>) -> Vec<&'a gds21::GdsStruct> {
        if let Some(strukt) = self.strukts.get(&cell.into()) {
            self.push(strukt);
        }
        self.stack
    }
    /// Adds all of `strukt`'s dependencies, and then `strukt` itself, to the stack.
    fn push(&mut self, strukt: &'a gds21::GdsStruct) {
        if !self.seen.contains(&strukt.name) {
            for elem in &strukt.elems {
                use gds21::GdsElement::*;
                match elem {
                    GdsStructRef(ref x) => self.push(self.strukts.get(&x.name).unwrap()),
                    GdsArrayRef(ref x) => self.push(self.strukts.get(&x.name).unwrap()),
                    _ => (),
                };
            }
            self.seen.insert(strukt.name.clone());
            self.stack.push(strukt);
        }
    }
}

impl<'a> GdsImporter<'a> {
    /// Creates a new [`GdsImporter`].
    fn new(data: &'a mut SubstrateData, layers: &'a mut Layers) -> Self {
        GdsImporter {
            data,
            layers,
            backtrace: Vec::new(),
            unsupported: Vec::new(),
            cell_map: HashMap::new(),
        }
    }
    /// Imports a [gds21::GdsLibrary].
    fn import_all(&mut self, gdslib: &gds21::GdsLibrary) -> LayoutResult<()> {
        self.backtrace.push(ErrorContext::Library);
        self.run_preimport_checks(gdslib)?;
        for strukt in &GdsDepOrder::new(gdslib).total_order() {
            self.import_and_add(strukt)?;
        }
        Ok(())
    }
    /// Imports a single cell and all of its dependencies into the provided cell.
    fn import_cell_with_deps(
        &mut self,
        gdslib: &gds21::GdsLibrary,
        to_import: impl Into<ArcStr>,
        cell: &mut Cell,
    ) -> LayoutResult<()> {
        let to_import = to_import.into();
        self.backtrace.push(ErrorContext::Library);
        self.run_preimport_checks(gdslib)?;

        let mut found = false;
        for strukt in &GdsDepOrder::new(gdslib).cell_order(to_import.clone()) {
            if strukt.name == to_import {
                self.import_cell(strukt, cell)?;
                found = true;
            } else {
                self.import_and_add(strukt)?;
            }
        }

        if !found {
            self.fail(format!("cell {to_import} not found in GDS library"))?;
        }
        Ok(())
    }
    /// Runs relevant checks before importing from a GDS library.
    fn run_preimport_checks(&mut self, gdslib: &gds21::GdsLibrary) -> LayoutResult<()> {
        // Unsupported GDSII features, if ever added, shall be imported here:
        // if gdslib.libdirsize.is_some()
        //     || gdslib.srfname.is_some()
        //     || gdslib.libsecur.is_some()
        //     || gdslib.reflibs.is_some()
        //     || gdslib.fonts.is_some()
        //     || gdslib.attrtable.is_some()
        //     || gdslib.generations.is_some()
        //     || gdslib.format_type.is_some()
        // {
        //     return self.fail("Unsupported GDSII Feature");
        // }
        // And convert each of its `structs` into our `cells`

        self.check_units(&gdslib.units)
    }
    /// Checks that the database units match up with the units specified by the PDK.
    fn check_units(&mut self, units: &gds21::GdsUnits) -> LayoutResult<()> {
        self.backtrace.push(ErrorContext::Units);
        // Peel out the GDS "database unit", the one of its numbers that really matters
        let gdsunit = units.db_unit();
        let rv = if (gdsunit - 1e-9).abs() < 1e-12 {
            SiPrefix::Nano
        } else if (gdsunit - 1e-6).abs() < 1e-9 {
            SiPrefix::Micro
        } else {
            return self.fail(format!("Unsupported GDSII Units {gdsunit:10.3e}"));
        };

        let pdk_units = self.data.layouts().units();
        if rv != pdk_units {
            return self.fail(format!(
                "Units do not match PDK units: {gdsunit:?} != {pdk_units:?}"
            ));
        }
        self.backtrace.pop();
        Ok(())
    }
    /// Imports and adds a cell if not already defined
    fn import_and_add(&mut self, strukt: &gds21::GdsStruct) -> LayoutResult<()> {
        let name = &strukt.name;
        // Check whether we're already defined, and bail if so
        if self.cell_map.get(name).is_some() {
            return self.fail(format!("Cell {name} defined multiple times in GDS file"));
        }

        let new_name = self.data.layouts().alloc_name(name);
        let id = self.data.layouts_mut().gen_id();

        // Add it to our library
        let mut cell = Cell::new(id);
        cell.set_name(new_name);
        self.import_cell(strukt, &mut cell)?;
        self.data.layouts_mut().set_cell(cell);
        let cell = self.data.layouts().get_by_id(id).unwrap();
        // And add the cell to our name-map
        self.cell_map.insert(name.clone(), cell.clone());
        Ok(())
    }
    /// Imports a GDS Cell ([gds21::GdsStruct]) into a [Cell]
    fn import_cell(&mut self, strukt: &gds21::GdsStruct, cell: &mut Cell) -> LayoutResult<()> {
        self.backtrace.push(ErrorContext::Cell(strukt.name.clone()));
        // Importing each layout requires at least two passes over its elements.
        // In the first pass we add each [Instance] and geometric element,
        // And keep a list of [gds21::GdsTextElem] on the side.
        let mut texts: Vec<&gds21::GdsTextElem> = Vec::new();
        let mut elems: SlotMap<ElementKey, Element> = SlotMap::with_key();
        // Also keep a hash of by-layer elements, to aid in text-assignment in our second pass
        let mut layers: HashMap<LayerKey, Vec<ElementKey>> = HashMap::new();
        for elem in &strukt.elems {
            use gds21::GdsElement::*;
            let e = match elem {
                GdsBoundary(ref x) => Some(self.import_boundary(x)?),
                GdsPath(ref x) => Some(self.import_path(x)?),
                GdsBox(ref x) => Some(self.import_box(x)?),
                GdsArrayRef(ref x) => {
                    cell.add_insts(self.import_instance_array(x)?);
                    None
                }
                GdsStructRef(ref x) => {
                    cell.add_inst(self.import_instance(x)?);
                    None
                }
                GdsTextElem(ref x) => {
                    texts.push(x);
                    None
                }
                // GDSII "Node" elements are fairly rare, and are not supported.
                // (Maybe some day we'll even learn what they are.)
                GdsNode(ref x) => {
                    self.unsupported.push(x.clone().into());
                    None
                }
            };
            // If we got a new element, add it to our per-layer hash
            if let Some(e) = e {
                let layer = e.layer.layer();
                let ekey = elems.insert(e);
                if let Some(ref mut bucket) = layers.get_mut(&layer) {
                    bucket.push(ekey);
                } else {
                    layers.insert(layer, vec![ekey]);
                }
            }
        }
        // Pass two: sort out whether each [gds21::GdsTextElem] is a net-label,
        // And if so, assign it as a net-name on each intersecting [Element].
        // Text elements which do not overlap a geometric element on the same layer
        // are converted to annotations.
        for textelem in &texts {
            // Import the GDS text element into a Substrate text element, creating missing layers
            // as necessary.
            let text_elem = self.import_text_elem(textelem)?;

            let net_name = textelem.string.to_lowercase().to_string();
            let text_spec = self
                .layers
                .get_from_spec(GdsLayerSpec(textelem.layer, textelem.texttype))
                .unwrap();
            let loc = self.import_point(&textelem.xy)?;

            let pin_spec = self
                .layers
                .to_gds_spec(text_spec)
                .map(|_| LayerSpec::new(text_spec.layer(), LayerPurpose::Pin));

            let purp = text_spec.purpose();
            if purp == &LayerPurpose::Label || purp == &LayerPurpose::Pin {
                if let Some(pin_spec) = pin_spec {
                    let mut port = CellPort::new(&net_name);
                    let mut has_geometry = false;
                    if let Some(layer) = layers.get_mut(&text_spec.layer()) {
                        // Layer exists in geometry; see which elements intersect with this text
                        for ekey in layer.iter() {
                            let elem = elems.get_mut(*ekey);
                            if elem.is_none() {
                                continue;
                            }
                            let elem = elem.unwrap();

                            if elem.inner.contains(loc) && elem.layer == pin_spec {
                                // Label lands inside this element.
                                // Check whether we have an existing label.
                                // If so, it better be the same net name!
                                if let Some(pname) = &elem.net {
                                    if *pname != net_name {
                                        println!(
                                            "Warning: GDSII label shorts nets {} and {} on layer {} in cell {}, skipping label",
                                            pname,
                                            textelem.string.clone(),
                                            textelem.layer,
                                            &strukt.name,
                                        );
                                    }
                                }
                                elem.net = Some(ArcStr::from(&net_name));
                                port.add(pin_spec.layer(), elem.inner.clone());
                                has_geometry = true;

                                // This pin shape is stored in a port.
                                // No need to also include it as a regular element.
                                elems.remove(*ekey);
                            }
                        }
                    }
                    cell.merge_port(port);
                }
            } else {
                // Import the text element as is
                cell.add_annotation(text_elem);
            }
        }
        // Pull the elements out of the local slot-map, into the vector that [Layout] wants
        cell.set_elems(elems.drain().map(|(_k, v)| v).collect());
        self.backtrace.pop();
        Ok(())
    }
    /// Imports a [gds21::GdsBoundary] into an [Element]
    fn import_boundary(&mut self, x: &gds21::GdsBoundary) -> LayoutResult<Element> {
        self.backtrace.push(ErrorContext::Geometry);
        let mut pts: Vec<Point> = self.import_point_vec(&x.xy)?;
        if pts[0] != *pts.last().unwrap() {
            return self.fail("GDS Boundary must start and end at the same point");
        }
        // Pop the redundant last entry
        pts.pop();
        // Check for Rectangles; they help
        let inner = if pts.len() == 4
            && ((pts[0].x == pts[1].x // Clockwise
                && pts[1].y == pts[2].y
                && pts[2].x == pts[3].x
                && pts[3].y == pts[0].y)
                || (pts[0].y == pts[1].y // Counter-clockwise
                    && pts[1].x == pts[2].x
                    && pts[2].y == pts[3].y
                    && pts[3].x == pts[0].x))
        {
            // That makes this a Rectangle.
            Shape::Rect(Rect {
                p0: pts[0],
                p1: pts[2],
            })
        } else {
            // Otherwise, it's a polygon
            Shape::Polygon(Polygon { points: pts })
        };

        // Grab (or create) its [Layer]
        let layer = self.import_element_layer(x)?;
        // Create the Element, and insert it in our slotmap
        let e = Element {
            net: None,
            layer,
            inner,
        };
        self.backtrace.pop();
        Ok(e)
    }
    /// Imports a [gds21::GdsBox] into an [Element]
    fn import_box(&mut self, x: &gds21::GdsBox) -> LayoutResult<Element> {
        self.backtrace.push(ErrorContext::Geometry);

        // GDS stores *five* coordinates per box (for whatever reason).
        // This does not check fox "box validity", and imports the
        // first and third of those five coordinates,
        // which are by necessity for a valid [GdsBox] located at opposite corners.
        let inner = Shape::Rect(Rect {
            p0: self.import_point(&x.xy[0])?,
            p1: self.import_point(&x.xy[2])?,
        });

        // Grab (or create) its [Layer]
        let layer = self.import_element_layer(x)?;
        // Create the Element, and insert it in our slotmap
        let e = Element {
            net: None,
            layer,
            inner,
        };
        self.backtrace.pop();
        Ok(e)
    }
    /// Import a [gds21::GdsPath] into an [Element]
    fn import_path(&mut self, x: &gds21::GdsPath) -> LayoutResult<Element> {
        self.backtrace.push(ErrorContext::Geometry);

        let pts = self.import_point_vec(&x.xy)?;
        let width = if let Some(w) = x.width {
            w as usize
        } else {
            return self.fail("Invalid nonspecifed GDS Path width ");
        };
        // Create the shape
        let inner = Shape::Path(Path { width, points: pts });

        // Grab (or create) its [Layer]
        let layer = self.import_element_layer(x)?;
        // Create the Element, and insert it in our slotmap
        let e = Element {
            net: None,
            layer,
            inner,
        };
        self.backtrace.pop();
        Ok(e)
    }
    /// Import a [gds21::GdsTextElem] cell/struct-instance into an [TextElement].
    fn import_text_elem(&mut self, sref: &gds21::GdsTextElem) -> LayoutResult<TextElement> {
        let string = ArcStr::from(sref.string.to_lowercase());
        self.backtrace.push(ErrorContext::Instance(string.clone()));
        // Convert its location
        let loc = self.import_point(&sref.xy)?;
        let layer = self.import_element_layer(sref)?;
        self.backtrace.pop();
        Ok(TextElement { string, loc, layer })
    }
    /// Import a [gds21::GdsStructRef] cell/struct-instance into an [Instance]
    fn import_instance(&mut self, sref: &gds21::GdsStructRef) -> LayoutResult<Instance> {
        let cname = sref.name.clone();
        self.backtrace.push(ErrorContext::Instance(cname.clone()));
        // Look up the cell-key, which must be imported by now
        let cell = self.unwrap(
            self.cell_map.get(&sref.name),
            format!("Instance of invalid cell {cname}"),
        )?;
        let cell = cell.clone();
        // Convert its location
        let loc = self.import_point(&sref.xy)?;
        let mut inst = Instance::builder().cell(cell).loc(loc).build().unwrap();
        // If defined, convert orientation settings
        if let Some(strans) = &sref.strans {
            inst.set_orientation(self.import_orientation(strans)?);
        }
        self.backtrace.pop();
        Ok(inst)
    }
    /// Imports a (two-dimensional) [`gds21::GdsArrayRef`] into [`Instance`]s.
    ///
    /// Returns the newly-created [`Instance`]s as a vector.
    /// Instance names are of the form `{array.name}[{col}][{row}]`.
    ///
    /// GDSII arrays are described by three spatial points:
    /// The origin, extent in "rows", and extent in "columns".
    /// In principle these need not be the same as "x" and "y" spacing,
    /// i.e. there might be "diamond-shaped" array specifications.
    ///
    /// Here, arrays are supported if they are "specified rectangular",
    /// i.e. that (a) the first two points align in `y`, and (b) the second two points align in `x`.
    ///
    /// Further support for such "non-rectangular-specified" arrays may (or may not) become a future addition,
    /// based on observed GDSII usage.
    fn import_instance_array(&mut self, aref: &gds21::GdsArrayRef) -> LayoutResult<Vec<Instance>> {
        let cname = aref.name.clone();
        self.backtrace.push(ErrorContext::Array(cname.clone()));

        // Look up the cell, which must be imported by now
        let cell = self.unwrap(
            self.cell_map.get(&aref.name),
            format!("Instance Array of invalid cell {cname}"),
        )?;
        let cell = Arc::clone(cell);

        // Convert its three (x,y) coordinates
        let p0 = self.import_point(&aref.xy[0])?;
        let p1 = self.import_point(&aref.xy[1])?;
        let p2 = self.import_point(&aref.xy[2])?;
        // Check for (thus far) unsupported non-rectangular arrays
        if p0.y != p1.y || p0.x != p2.x {
            self.fail("Unsupported Non-Rectangular GDS Array")?;
        }
        // Sort out the inter-element spacing
        let mut xstep = (p1.x - p0.x) / i64::from(aref.cols);
        let mut ystep = (p2.y - p0.y) / i64::from(aref.rows);

        // Incorporate the reflection/ rotation settings
        let mut orientation = Orientation::default();
        if let Some(strans) = &aref.strans {
            orientation = self.import_orientation(strans)?;
        }
        // The angle-setting rotates the *entire* array lattice together.
        // Update the (x,y) steps via a rotation-matrix multiplication:
        // x = x * cos(a) - y * sin(a)
        // y = x * sin(a) + y * cos(a)
        let prev_xy = (i32::try_from(xstep)?, i32::try_from(ystep)?);
        let prev_xy = (f64::from(prev_xy.0), f64::from(prev_xy.1));
        let a = orientation.angle().to_radians(); // Rust `sin` and `cos` take radians, convert first
        xstep = (prev_xy.0 * a.cos() - prev_xy.1 * a.sin()) as i64;
        ystep = (prev_xy.0 * a.sin() + prev_xy.1 * a.cos()) as i64;

        // Create the Instances
        let mut insts = Vec::with_capacity((aref.rows * aref.cols) as usize);
        for ix in 0..i64::from(aref.cols) {
            let x = p0.x + ix * xstep;
            for iy in 0..i64::from(aref.rows) {
                let y = p0.y + iy * ystep;
                insts.push(
                    Instance::builder()
                        .name(ArcStr::from(format!("{cname}[{ix}][{iy}]")))
                        .cell(cell.clone())
                        .loc(Point::new(x, y))
                        .orientation(orientation)
                        .build()
                        .unwrap(),
                );
            }
        }
        self.backtrace.pop();
        Ok(insts)
    }
    /// Imports a [`Point`].
    fn import_point(&self, pt: &gds21::GdsPoint) -> LayoutResult<Point> {
        let x = pt.x.into();
        let y = pt.y.into();
        Ok(Point::new(x, y))
    }
    /// Imports a vector of [`Point`]s.
    fn import_point_vec(&mut self, pts: &[gds21::GdsPoint]) -> LayoutResult<Vec<Point>> {
        pts.iter()
            .map(|p| self.import_point(p))
            .collect::<Result<Vec<_>, _>>()
    }
    /// Imports an orientation.
    fn import_orientation(&mut self, strans: &gds21::GdsStrans) -> LayoutResult<Orientation> {
        if strans.abs_mag || strans.abs_angle {
            return self.fail("Unsupported GDSII Instance Feature: Absolute Magnitude/ Angle");
        }
        if strans.mag.is_some() {
            self.fail("Unsupported GDSII Setting: Magnitude")?;
        }
        Ok(strans.clone().into())
    }
    /// Gets the [`LayerSpec`] for a GDS element implementing its [`gds21::HasLayer`] trait.
    /// Layers are created if they do not already exist,
    /// although this may eventually be a per-importer setting.
    fn import_element_layer(&mut self, elem: &impl gds21::HasLayer) -> LayoutResult<LayerSpec> {
        let spec = elem.layerspec().into();
        let layers = &mut self.layers;
        Ok(if let Some(layer_spec) = layers.get_from_spec(spec) {
            layer_spec
        } else {
            layers.add(
                LayerInfo::builder()
                    .purps(HashMap::from_iter([(LayerPurpose::Other(spec.1), spec)]))
                    .build()
                    .unwrap(),
            );

            layers.get_from_spec(spec).unwrap()
        }
        .clone())
    }
}
impl<'a> ErrorHelper for GdsImporter<'a> {
    type Error = LayoutError;
    fn err(&self, msg: impl Into<String>) -> LayoutError {
        LayoutError::Import {
            message: msg.into(),
            stack: self.backtrace.clone(),
        }
    }
}

impl ExportSet {
    #[inline]
    pub fn contains(&self, key: &CellKey) -> bool {
        match self {
            Self::All => true,
            Self::Set(s) => s.contains(key),
        }
    }

    pub fn for_top(top: &Arc<Cell>) -> Self {
        let mut set = HashSet::new();
        let mut stack = VecDeque::new();
        stack.push_front(top);
        set.insert(top.id());

        while let Some(cell) = stack.pop_front() {
            for inst in cell.insts() {
                let cell = inst.cell();
                let id = cell.id();
                if !set.contains(&id) {
                    set.insert(id);
                    stack.push_front(cell);
                }
            }
        }

        Self::Set(set)
    }
}
