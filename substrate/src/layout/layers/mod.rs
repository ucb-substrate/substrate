//! Utilities and types for managing layers in a PDK.

use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::str::FromStr;
use std::sync::{Arc, RwLock};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};
use subgeom::bbox::{Bbox, BoundBox};
use thiserror::Error;

use self::selector::Selector;
use crate::deps::arcstr::ArcStr;
use crate::error::{ErrorSource, Result as SubResult};

pub mod selector;

new_key_type! {
    /// A unique identifier for a layer in a PDK.
    pub struct LayerKey;
}

/// A GDS layer specification.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct GdsLayerSpec(pub i16, pub i16);

impl From<gds21::GdsLayerSpec> for GdsLayerSpec {
    fn from(other: gds21::GdsLayerSpec) -> Self {
        Self(other.layer, other.xtype)
    }
}

#[allow(clippy::from_over_into)]
impl Into<gds21::GdsLayerSpec> for GdsLayerSpec {
    fn into(self) -> gds21::GdsLayerSpec {
        gds21::GdsLayerSpec {
            layer: self.0,
            xtype: self.1,
        }
    }
}

#[derive(Deserialize)]
struct CsvLayerRecord {
    layernum: i16,
    datatype: i16,
    name: String,
    purpose: String,
}

/// An enumeration of layer purposes.
///
/// Includes the common use-cases for each shape,
/// and two "escape hatches", one named and one not.
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq, Hash)]
pub enum LayerPurpose {
    // First-class enumerated purposes
    Drawing,
    Pin,
    Label,
    Obstruction,
    Outline,
    /// Named purpose, not first-class supported
    Named(ArcStr),
    /// Other purpose, not first-class supported nor named
    Other(i16),
}

#[derive(Debug, Error)]
#[error("error converting string")]
pub struct FromStrError;

impl FromStr for LayerPurpose {
    type Err = FromStrError;
    fn from_str(purp: &str) -> Result<Self, Self::Err> {
        Ok(match purp {
            "drawing" => Self::Drawing,
            "pin" => Self::Pin,
            "label" => Self::Label,
            "obstruction" => Self::Obstruction,
            "outline" => Self::Outline,
            _ => match purp.parse::<i16>() {
                Ok(other) => Self::Other(other),
                Err(_) => Self::Named(ArcStr::from(purp)),
            },
        })
    }
}

/// A unique identifier for a specific GDS layer based on its definition in a PDK.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct LayerSpec(LayerKey, LayerPurpose);

impl LayerSpec {
    /// Creates a new [`LayerSpec`].
    #[inline]
    pub fn new(key: LayerKey, purpose: LayerPurpose) -> Self {
        Self(key, purpose)
    }

    /// Returns the spec for the drawing purpose of the layer associated with key `key`.
    pub fn drawing(key: LayerKey) -> Self {
        Self(key, LayerPurpose::Drawing)
    }

    /// Returns the spec for the pin purpose of the layer associated with key `key`.
    pub fn pin(key: LayerKey) -> Self {
        Self(key, LayerPurpose::Pin)
    }

    /// Returns the spec for the label purpose of the layer associated with key `key`.
    pub fn label(key: LayerKey) -> Self {
        Self(key, LayerPurpose::Label)
    }

    /// Returns the layer key of a [`LayerSpec`].
    #[inline]
    pub fn layer(&self) -> LayerKey {
        self.0
    }

    /// Returns the purpose of a [`LayerSpec`].
    #[inline]
    pub fn purpose(&self) -> &LayerPurpose {
        &self.1
    }
}

/// A manager for layers in a PDK.
///
/// Keeps track of active layers and indexes them by name.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Layers {
    slots: SlotMap<LayerKey, Layer>,
    names: HashMap<ArcStr, LayerKey>,
    gds_to_layout: HashMap<GdsLayerSpec, LayerSpec>,
    route_idxs: HashMap<usize, LayerKey>,
    metal_idxs: HashMap<usize, LayerKey>,
    via_idxs: HashMap<usize, LayerKey>,
}

impl Layers {
    /// Creates an empty [`Layers`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a [`Layers`] from a vector of [`LayerInfo`]s.
    pub fn from_layer_infos(layer_infos: Vec<LayerInfo>) -> Self {
        let mut layers = Self::new();
        for info in layer_infos {
            layers.add(info);
        }
        layers
    }

    /// Creates a [`Layers`] from purposes specified in a CSV file.
    ///
    /// Uses the provided `base` closure to fill out additional Substrate-specific metadata for
    /// each layer.
    pub fn from_csv(
        csv: &str,
        mut base: impl FnMut(&str) -> LayerInfo,
    ) -> Result<Self, csv::Error> {
        let mut reader = csv::Reader::from_reader(csv.as_bytes());
        let mut layer_infos: HashMap<String, LayerInfo> = HashMap::new();

        for record in reader.deserialize() {
            let record: CsvLayerRecord = record?;
            let purp = LayerPurpose::from_str(&record.purpose).unwrap();
            let gds_spec = GdsLayerSpec(record.layernum, record.datatype);
            match layer_infos.entry(record.name.clone()) {
                Entry::Occupied(o) => {
                    o.into_mut().add_purpose(purp, gds_spec);
                }
                Entry::Vacant(v) => {
                    let mut layer_info = base(&record.name);
                    layer_info.name = ArcStr::from(record.name);
                    layer_info.add_purpose(purp, gds_spec);
                    v.insert(layer_info);
                }
            }
        }

        Ok(Self::from_layer_infos(layer_infos.into_values().collect()))
    }

    /// Adds a [`Layer`] to our slot-map and number-map, and name-map.
    pub fn add(&mut self, layer: LayerInfo) -> LayerKey {
        // Layer names should be unique
        let name = layer.name.clone();
        let key = self.slots.insert_with_key(|k| Layer::new(k, layer));
        for (purp, gds_spec) in self.slots[key].purps() {
            self.gds_to_layout
                .insert(*gds_spec, LayerSpec::new(key, purp.clone()));
        }
        self.names.insert(name, key);

        if let Some(via_idx) = self.slots[key].info.via_idx {
            self.via_idxs.insert(via_idx, key);
        }
        if let Some(route_idx) = self.slots[key].info.route_idx {
            self.route_idxs.insert(route_idx, key);
        }
        if let Some(metal_idx) = self.slots[key].info.metal_idx {
            self.metal_idxs.insert(metal_idx, key);
        }

        key
    }
    /// Gets a reference to the [`LayerKey`] with layer name `name`.
    pub fn get_key<Q>(&self, name: &Q) -> Option<LayerKey>
    where
        Q: Hash + Eq + ?Sized,
        ArcStr: Borrow<Q>,
    {
        self.names.get(name).cloned()
    }
    /// Gets a reference to the [`Layer`] with name `name`.
    pub fn get_layer<Q>(&self, name: &Q) -> Option<&Layer>
    where
        Q: Hash + Eq + ?Sized,
        ArcStr: Borrow<Q>,
    {
        let key = self.get_key(name)?;
        self.slots.get(key)
    }
    /// Gets the name of `key`.
    pub fn get_name(&self, key: LayerKey) -> SubResult<&ArcStr> {
        let layer = self
            .slots
            .get(key)
            .ok_or(ErrorSource::LayerNotFound(format!("{key:?}")))?;
        Ok(&layer.info.name)
    }
    /// Gets a reference to the [`Layer`] from [`LayerKey`] `key`.
    pub fn get(&self, key: LayerKey) -> Option<&Layer> {
        self.slots.get(key)
    }
    /// Gets a shared reference to the internal <[`LayerKey`], [`Layer`]> map.
    pub fn slots(&self) -> &SlotMap<LayerKey, Layer> {
        &self.slots
    }
    /// Gets a reference to [`Layer`] from [`GdsLayerSpec`] `spec`.
    pub fn get_layer_from_spec(&self, spec: GdsLayerSpec) -> Option<&Layer> {
        let &LayerSpec(key, _) = self.get_from_spec(spec)?;
        self.get(key)
    }
    /// Gets the [`LayerSpec`] corresponding to [`GdsLayerSpec`] `spec`.
    pub fn get_from_spec(&self, spec: GdsLayerSpec) -> Option<&LayerSpec> {
        self.gds_to_layout.get(&spec)
    }

    /// Converts a [`LayerSpec`] into its corresponding [`GdsLayerSpec`].
    pub fn to_gds_spec(&self, spec: &LayerSpec) -> Option<GdsLayerSpec> {
        self.get(spec.layer())
            .and_then(|layer| layer.spec(spec.purpose()))
    }

    /// Converts a [`LayerKey`] to a [`GdsLayerSpec`] corresponding to labels for that layer.
    pub fn to_label_gds_spec(&self, key: LayerKey) -> Option<GdsLayerSpec> {
        self.get(key).and_then(|layer| layer.label_spec())
    }

    /// Returns a [`Vec`] consisting of all layer names in the layer manager.
    pub fn get_layer_names(&self) -> Vec<&ArcStr> {
        self.names.keys().collect()
    }
}

/// A layer in a PDK.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Layer {
    /// A unique identifier.
    pub id: LayerKey,
    /// Information associated with the layer.
    pub info: LayerInfo,
}

/// Metadata associated with a layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Builder)]
#[builder(pattern = "owned")]
pub struct LayerInfo {
    /// The layer name.
    #[builder(setter(into), default)]
    pub name: ArcStr,
    /// A layer purpose to GDS spec lookup table.
    #[builder(setter(into), default)]
    pub purps: HashMap<LayerPurpose, GdsLayerSpec>,
    /// Indicates if this layer should be used for routing.
    #[builder(setter(strip_option), default)]
    pub route_idx: Option<usize>,
    /// Indicates if this layer should be used as a metal layer.
    #[builder(setter(strip_option), default)]
    pub metal_idx: Option<usize>,
    /// Indicates if this layer should be used as a via layer.
    ///
    /// See [`Selector::Via`] for more information.
    #[builder(setter(strip_option), default)]
    pub via_idx: Option<usize>,
    /// The type of the layer.
    #[builder(default)]
    pub layer_type: LayerType,
    /// The purpose with which labels should be emitted.
    #[builder(default = "LayerPurpose::Label")]
    pub label_purpose: LayerPurpose,
}

impl Default for LayerInfo {
    fn default() -> Self {
        Self {
            name: Default::default(),
            purps: Default::default(),
            route_idx: Default::default(),
            metal_idx: Default::default(),
            via_idx: Default::default(),
            layer_type: Default::default(),
            label_purpose: LayerPurpose::Label,
        }
    }
}

/// An enumeraton of layer types.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Debug, Serialize, Deserialize)]
pub enum LayerType {
    Metal,
    Via,
    Diffusion,
    Gate,
    Well,
    Tap,
    Implant,
    #[default]
    Other,
}

impl Layer {
    pub(crate) fn new(id: LayerKey, info: LayerInfo) -> Self {
        Self { id, info }
    }

    /// Adds a new [`LayerPurpose`].
    #[inline]
    pub fn add_purpose(&mut self, purp: LayerPurpose, spec: GdsLayerSpec) {
        self.info.add_purpose(purp, spec);
    }

    /// Adds purpose-spec `pairs`.
    ///
    /// Consumes and returns `self` for chainability.
    pub fn add_pairs(mut self, pairs: &[(LayerPurpose, GdsLayerSpec)]) -> Self {
        for (purp, spec) in pairs {
            self.add_purpose(purp.clone(), *spec);
        }
        self
    }

    /// Retrieves the spec for this layer and [`purpose`](LayerPurpose).
    pub fn spec(&self, purpose: &LayerPurpose) -> Option<GdsLayerSpec> {
        self.info.spec(purpose)
    }

    /// Retrieves the label spec for this layer.
    pub fn label_spec(&self) -> Option<GdsLayerSpec> {
        self.info.label_spec()
    }

    /// Retrieves a list of [`LayerPurpose`]-[`GdsLayerSpec`] tuples.
    pub fn purps(&self) -> Vec<(&LayerPurpose, &GdsLayerSpec)> {
        self.info.purps()
    }
}

impl LayerInfo {
    /// Creates a new [`LayerInfoBuilder`].
    #[inline]
    pub fn builder() -> LayerInfoBuilder {
        LayerInfoBuilder::default()
    }

    /// Adds a new [`LayerPurpose`].
    #[inline]
    pub fn add_purpose(&mut self, purp: LayerPurpose, spec: GdsLayerSpec) {
        self.purps.insert(purp, spec);
    }

    /// Adds purpose-spec `pairs`.
    ///
    /// Consumes and returns `self` for chainability.
    pub fn add_pairs(mut self, pairs: &[(LayerPurpose, GdsLayerSpec)]) -> Self {
        for (purp, spec) in pairs {
            self.add_purpose(purp.clone(), *spec);
        }
        self
    }

    /// Retrieves the spec for this layer and [`purpose`](LayerPurpose).
    pub fn spec(&self, purpose: &LayerPurpose) -> Option<GdsLayerSpec> {
        self.purps.get(purpose).copied()
    }

    /// Retrieves the label spec for this layer.
    pub fn label_spec(&self) -> Option<GdsLayerSpec> {
        self.purps.get(&self.label_purpose).copied()
    }

    /// Retrieves a list of [`LayerPurpose`]-[`GdsLayerSpec`] tuples.
    pub fn purps(&self) -> Vec<(&LayerPurpose, &GdsLayerSpec)> {
        self.purps.iter().collect()
    }
}

/// A cheaply-clonable reference to [`Layers`].
pub struct LayersRef {
    inner: Arc<RwLock<Layers>>,
}

impl LayersRef {
    /// Creates a new [`LayersRef`].
    #[inline]
    pub(crate) fn new(inner: Arc<RwLock<Layers>>) -> Self {
        Self { inner }
    }

    /// Gets a [`LayerKey`] based on the provided [`Selector`].
    pub fn get(&self, sel: Selector) -> SubResult<LayerKey> {
        let inner = self.inner.read().unwrap();
        let key = match sel {
            Selector::Metal(n) => inner.metal_idxs.get(&n).copied(),
            Selector::Routing(n) => inner.route_idxs.get(&n).copied(),
            Selector::Via(n) => inner.via_idxs.get(&n).copied(),
            Selector::Name(n) => inner.names.get(n).copied(),
            Selector::Gds(spec) => inner.get_layer_from_spec(spec).map(|l| l.id),
        };
        key.ok_or(ErrorSource::LayerNotFound(format!("{sel:?}")).into())
    }

    /// Gets the [`LayerInfo`] associated with [`LayerKey`] `layer`.
    pub fn info(&self, layer: LayerKey) -> SubResult<LayerInfo> {
        let inner = self.inner.read().unwrap();
        let info = inner
            .slots
            .get(layer)
            .map(|l| l.info.clone())
            .ok_or(ErrorSource::LayerNotFound(format!("{layer:?}")))?;
        Ok(info)
    }

    /// Returns the name associated with [`LayerKey`] `layer`.
    pub fn name(&self, layer: LayerKey) -> SubResult<ArcStr> {
        let inner = self.inner.read().unwrap();
        let name = inner
            .slots
            .get(layer)
            .map(|l| l.info.name.clone())
            .ok_or(ErrorSource::LayerNotFound(format!("{layer:?}")))?;
        Ok(name)
    }

    /// Gets the metal index corresponding to this layer.
    ///
    /// Returns an error if this layer is not a metal layer.
    pub fn which_metal(&self, layer: LayerKey) -> SubResult<usize> {
        let inner = self.inner.read().unwrap();
        let layer = inner
            .slots
            .get(layer)
            .ok_or(ErrorSource::LayerNotFound(format!("{layer:?}")))?;
        let idx = layer
            .info
            .metal_idx
            .ok_or(ErrorSource::LayerNotFound(format!("{layer:?}")))?;
        Ok(idx)
    }

    /// Gets the via index corresponding to this layer.
    ///
    /// Returns an error if this layer is not a via layer.
    pub fn which_via(&self, layer: LayerKey) -> SubResult<usize> {
        let inner = self.inner.read().unwrap();
        let layer = inner
            .slots
            .get(layer)
            .ok_or(ErrorSource::LayerNotFound(format!("{layer:?}")))?;
        let idx = layer
            .info
            .via_idx
            .ok_or(ErrorSource::LayerNotFound(format!("{layer:?}")))?;
        Ok(idx)
    }
}

/// Options by which Substrate users can name a layer.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum UserLayer {
    /// Use the given [`LayerKey`], with a purpose selected
    /// by Substrate.
    Key(LayerKey),
    /// Use the given [`LayerSpec`].
    ///
    /// Substrate will not try to infer the correct purpose.
    Spec(LayerSpec),
}

impl From<LayerKey> for UserLayer {
    #[inline]
    fn from(value: LayerKey) -> Self {
        Self::Key(value)
    }
}

impl From<LayerSpec> for UserLayer {
    #[inline]
    fn from(value: LayerSpec) -> Self {
        Self::Spec(value)
    }
}

impl UserLayer {
    /// Converts the user's layer selection to a [`LayerSpec`].
    ///
    /// If the user did not specify a purpose, the `default_purpose` will be used.
    pub fn to_spec(self, default_purpose: LayerPurpose) -> LayerSpec {
        match self {
            Self::Key(key) => LayerSpec::new(key, default_purpose),
            Self::Spec(spec) => spec,
        }
    }
}

/// A trait representing functions available for multi-layered objects with bounding boxes.
pub trait LayerBoundBox: BoundBox {
    fn layer_bbox(&self, layer: LayerKey) -> Bbox;
}
