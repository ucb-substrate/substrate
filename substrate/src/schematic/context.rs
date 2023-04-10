//! Context methods for instantiating component schematics.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::sync::Arc;

use itertools::Itertools;
use slotmap::new_key_type;

use super::circuit::{Direction, Instance, PortInfo};
use super::module::{ExternalModule, Module};
use super::signal::Slice;
use crate::component::Component;
use crate::data::SubstrateCtx;
use crate::deps::arcstr::ArcStr;
use crate::error::{with_err_context, ErrorContext, ErrorSource, Result as SubResult};
use crate::fmt::signal::{parse_bus, ParsedBus};
use crate::generation::{GeneratedCheck, GenerationMap, ParamKey};
use crate::hard_macro::Config as HardMacroConfig;
use crate::index::IndexOwned;
use crate::pdk::mos::db::MosDb;
use crate::pdk::Pdk;
use crate::units::SiPrefix;

/// Context for creating the schematic view of a [`Component`].
pub struct SchematicCtx {
    pub(crate) inner: SubstrateCtx,
    pub(crate) module: Module,
}

#[derive(Debug)]
pub(crate) struct SchematicData {
    #[allow(dead_code)]
    pub(crate) units: SiPrefix,
    pub(crate) modules: GenerationMap<ParamKey, ModuleKey, Module>,
    pub(crate) external_modules: HashMap<ArcStr, Arc<ExternalModule>>,
}

new_key_type! {
    /// A key identifying a schematic-level [`Module`].
    pub struct ModuleKey;
}

impl SchematicData {
    #[inline]
    pub fn new(pdk: Arc<dyn Pdk>) -> Self {
        Self {
            units: pdk.lengths().schematic,
            modules: GenerationMap::new(),
            external_modules: HashMap::new(),
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(crate) fn get_id<T>(&mut self, params: &T::Params) -> GeneratedCheck<ModuleKey, ModuleKey>
    where
        T: Component,
    {
        self.modules.get_id(ParamKey::from_params::<T>(params))
    }

    pub(crate) fn get_module<T>(
        &mut self,
        params: &T::Params,
    ) -> GeneratedCheck<Arc<Module>, ModuleKey>
    where
        T: Component,
    {
        self.modules.get(ParamKey::from_params::<T>(params))
    }

    pub(crate) fn set_module(&mut self, module: Module) -> Arc<Module> {
        self.modules.set(module.id, module.name().clone(), module)
    }

    pub(crate) fn get_by_id(&self, id: ModuleKey) -> SubResult<&Arc<Module>> {
        self.modules.get_by_id(id)
    }

    pub(crate) fn get_external<Q>(&self, key: &Q) -> SubResult<&Arc<ExternalModule>>
    where
        Q: AsRef<str>,
    {
        let key = key.as_ref();

        self.external_modules
            .get(key)
            .ok_or_else(|| ErrorSource::ModuleNotFound(key.to_string()).into())
    }

    pub(crate) fn add_external(&mut self, module: ExternalModule) -> SubResult<()> {
        let entry = self.external_modules.entry(module.name().to_owned());
        match entry {
            Entry::Occupied(_) => {
                return Err(ErrorSource::AlreadyExists(module.name().to_owned()).into())
            }
            Entry::Vacant(v) => v.insert(Arc::new(module)),
        };
        Ok(())
    }

    pub(crate) fn external_modules(&self) -> impl Iterator<Item = &Arc<ExternalModule>> {
        self.external_modules.values()
    }
}

#[allow(dead_code)]
struct HardMacroImport<'a> {
    path: &'a Path,
    subckt: &'a str,
    config: Option<&'a HardMacroConfig>,
}

impl SchematicCtx {
    #[inline]
    pub fn inner(&self) -> &SubstrateCtx {
        &self.inner
    }
    #[inline]
    pub fn pdk(&self) -> Arc<dyn Pdk> {
        self.inner.pdk()
    }

    #[inline]
    pub fn mos_db(&self) -> Arc<MosDb> {
        self.inner.mos_db()
    }

    #[inline]
    pub fn instantiate<T>(&mut self, params: &T::Params) -> SubResult<Instance>
    where
        T: Component,
    {
        self.inner.instantiate_schematic::<T>(params)
    }

    #[inline]
    pub fn instantiate_external<Q>(&mut self, name: &Q) -> SubResult<Instance>
    where
        Q: AsRef<str>,
    {
        self.inner.instantiate_external_schematic::<Q>(name)
    }

    pub fn add_instance(&mut self, inst: Instance) {
        self.module.add_instance(inst);
    }

    pub fn port(&mut self, name: impl Into<ArcStr>, direction: Direction) -> Slice {
        self.module.add_port(name, 1, direction)
    }

    pub fn ports<T, const N: usize>(&mut self, names: [T; N], direction: Direction) -> [Slice; N]
    where
        T: Into<ArcStr>,
    {
        names.map(move |name| self.port(name, direction))
    }

    pub fn bus_port(
        &mut self,
        name: impl Into<ArcStr>,
        width: usize,
        direction: Direction,
    ) -> Slice {
        self.module.add_port(name, width, direction)
    }

    pub fn signal(&mut self, name: impl Into<ArcStr>) -> Slice {
        self.module.add_signal(name, 1)
    }

    pub fn signals<T, const N: usize>(&mut self, names: [T; N]) -> [Slice; N]
    where
        T: Into<ArcStr>,
    {
        names.map(|name| self.signal(name))
    }

    /// Creates a single bus with the given name and width.
    pub fn bus(&mut self, name: impl Into<ArcStr>, width: usize) -> Slice {
        self.module.add_signal(name, width)
    }

    /// Creates a set of buses, all with the same width (ie. number of bits).
    pub fn buses<T, const N: usize>(&mut self, names: [T; N], width: usize) -> [Slice; N]
    where
        T: Into<ArcStr>,
    {
        names.map(|name| self.bus(name, width))
    }

    pub fn set_spice(&mut self, spice: impl Into<ArcStr>) {
        self.module.set_raw_spice(spice)
    }

    /// Bubbles up the port with the given name.
    ///
    /// The instance must still be added to the schematic context.
    pub fn bubble_port(&mut self, instance: &mut Instance, port: impl Into<ArcStr>) {
        let port = port.into();
        let iport = instance.port(&port).unwrap();
        let slice = self.bus_port(port.clone(), iport.width(), iport.direction());
        instance.connect(port, slice);
    }

    /// Bubbles up the port with the given name, and renames it to `new_name`.
    ///
    /// The instance must still be added to the schematic context.
    pub fn bubble_renamed_port(
        &mut self,
        instance: &mut Instance,
        port: impl Into<ArcStr>,
        new_name: impl Into<ArcStr>,
    ) {
        let port = port.into();
        let iport = instance.port(&port).unwrap();
        let slice = self.bus_port(new_name.into(), iport.width(), iport.direction());
        instance.connect(port, slice);
    }

    /// Bubbles up all ports on the given [`Instance`].
    ///
    /// The instance must still be added to the schematic context.
    pub fn bubble_all_ports(&mut self, instance: &mut Instance) {
        let ports = instance.ports().unwrap().collect_vec();
        for info in ports {
            let slice = self.bus_port(info.name().clone(), info.width(), info.direction());
            instance.connect(info.name(), slice);
        }
    }

    /// Uses `f` to filter and rename ports, then bubbles up ports that passed through the filter.
    ///
    /// The instance must still be added to the schematic context.
    pub fn bubble_filter_map<F>(&mut self, instance: &mut Instance, mut f: F)
    where
        F: FnMut(PortInfo) -> Option<ArcStr>,
    {
        let ports = instance.ports().unwrap().collect_vec();
        for info in ports {
            if let Some(name) = f(info.clone()) {
                let slice = self.bus_port(name, info.width(), info.direction());
                instance.connect(info.name(), slice);
            }
        }
    }

    /// Imports a SPICE file as the contents of the current [`Component`].
    ///
    /// If you use this, you should not use any other [`SchematicCtx`]
    /// methods when creating your schematic.
    // FIXME: make sure users don't do anything else with the context.
    // See issue #56: https://github.com/rahulk29/substrate/issues/56.
    pub fn import_spice(
        &mut self,
        subckt_name: impl Into<ArcStr>,
        path: impl AsRef<Path>,
    ) -> SubResult<()> {
        let subckt = subckt_name.into();
        let path = path.as_ref();

        let mut inner = || -> Result<(), crate::error::SubstrateError> {
            // Rename this module to avoid conflicting with the external module.
            self.module.set_name(arcstr::format!("{}_wrapper", subckt));

            let ext = ExternalModule::from_spice_file(&subckt, path)?;
            let conns = ext
                .ports
                .iter()
                .map(|port| {
                    let name = ext.signals()[port.signal].name().clone();
                    let slice = self.port(&name, Direction::InOut);
                    (name, slice)
                })
                .collect::<Vec<_>>();

            self.inner.add_external_module(ext)?;

            let mut inst = self.instantiate_external(&subckt)?;
            inst.connect_all(conns);
            self.add_instance(inst);

            Ok(())
        };

        with_err_context(inner(), || {
            ErrorContext::Task(arcstr::format!(
                "importing SPICE netlist for subcircuit `{subckt}` from {path:?}"
            ))
        })
    }

    pub fn import_hard_macro_config(&mut self, config: HardMacroConfig) -> SubResult<()> {
        let subckt = config.spice_subckt_name.ok_or_else(|| {
            ErrorSource::InvalidArgs(
                "subcircuit name must be specified when importing hard macro".to_string(),
            )
        })?;
        let path = config.spice_path.ok_or_else(|| {
            ErrorSource::InvalidArgs(
                "spice file path must be specified when importing hard macro".to_string(),
            )
        })?;

        // Rename this module to avoid conflicting with the external module.
        self.module.set_name(arcstr::format!("{}_wrapper", subckt));

        let ext = ExternalModule::from_spice_file(&subckt, path)?;

        struct PortStatus {
            slice: Slice,
            connected: Vec<bool>,
        }

        let mut pub_ports = config
            .ports
            .into_iter()
            .map(|(name, info)| {
                (
                    name.clone(),
                    PortStatus {
                        slice: self.bus_port(name, info.width, info.direction),
                        connected: vec![false; info.width],
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut conns = Vec::new();

        for port in ext.ports.iter() {
            let raw_name = ext.signals()[port.signal].name();

            match parse_bus(raw_name, config.bus_format) {
                Ok(ParsedBus { name, idx }) => {
                    let status = pub_ports
                        .get_mut(name)
                        .ok_or_else(|| ErrorSource::PortNotFound(name.into()))?;
                    if idx >= status.connected.len() {
                        return Err(ErrorSource::PortIndexOutOfBounds {
                            width: status.connected.len(),
                            index: idx,
                        }
                        .into());
                    }
                    assert!(!status.connected[idx]);
                    status.connected[idx] = true;
                    conns.push((raw_name.clone(), status.slice.index(idx)));
                }
                Err(_) => {
                    let status = pub_ports
                        .get_mut(raw_name)
                        .ok_or_else(|| ErrorSource::PortNotFound(raw_name.clone()))?;
                    if status.connected.len() != 1 {
                        return Err(ErrorSource::InvalidArgs(format!(
                            "bus indices not found for bus port {raw_name}"
                        ))
                        .into());
                    }
                    assert!(!status.connected[0]);
                    status.connected[0] = true;
                    conns.push((raw_name.clone(), status.slice));
                }
            }
        }

        if !pub_ports
            .iter()
            .all(|(_, s)| s.connected.iter().all(|v| *v))
        {
            return Err(
                ErrorSource::InvalidArgs("not all subcircuit ports connected".to_string()).into(),
            );
        }

        self.inner.add_external_module(ext)?;

        let mut inst = self.instantiate_external(&subckt)?;
        inst.connect_all(conns);
        self.add_instance(inst);

        Ok(())
    }
}
