use std::collections::HashMap;
use std::sync::Arc;

use arcstr::ArcStr;

use super::module::{DigitalModule, DigitalModuleKey, Direction};
use super::types::HardwareType;
use super::wire::{Wire, WireInner};
use super::{DigitalComponent, Interface};
use crate::data::SubstrateCtx;
use crate::digital::wire::WireValue;
use crate::digital::{ModulePort, ParentModulePort};
use crate::generation::{GeneratedCheck, GenerationMap, ParamKey};

pub struct DigitalCtx {
    pub(crate) inner: SubstrateCtx,
    pub(crate) module: DigitalModule,
}

#[derive(Debug)]
pub struct DigitalData {
    pub(crate) modules: GenerationMap<ParamKey, DigitalModuleKey, DigitalModule>,
    // pub(crate) external_modules: HashMap<ArcStr, Arc<ExternalModule>>,
}

impl DigitalCtx {
    pub fn inner(&self) -> &SubstrateCtx {
        &self.inner
    }

    pub fn port<T>(&mut self, name: impl Into<ArcStr>, ty: T) -> Wire<T>
    where
        T: Into<HardwareType>,
    {
        let name = name.into();
        let wire = Wire::new(WireInner::new(ty.into(), WireValue::Port(name.clone())));
        let prev = self.module.port_wires.insert(name, wire._inner());
        assert!(
            prev.is_none(),
            "port names should be unique (module {})",
            self.module.name()
        );

        wire
    }

    pub fn instance_output<T>(&mut self, ty: T) -> Wire<T>
    where
        T: Into<HardwareType>,
    {
        Wire::new(WireInner::new(ty.into(), WireValue::InstanceOutput))
    }

    pub fn instantiate<T>(
        &mut self,
        params: &T::Params,
    ) -> crate::error::Result<<T::Interface as Interface>::Parent>
    where
        T: DigitalComponent,
    {
        let (inst, intf) = self.inner._instantiate_digital::<T>(params)?;
        Ok(intf.parent(inst, self))
    }

    pub fn add_instance<T>(&mut self, interface: T::Parent)
    where
        T: Interface,
    {
        let ports = T::ports();
        let mut connections = HashMap::with_capacity(ports.len());
        for port in ports {
            let wire = interface.port(&port.name);
            connections.insert(port.name, wire);
        }
        let mut inst = interface.into_instance();
        inst.connections = connections;
        self.module.instances.push(inst);
    }

    pub(crate) fn finish<T>(&mut self, interface: <T::Interface as Interface>::Output)
    where
        T: DigitalComponent,
    {
        for (name, _) in self
            .module
            .ports
            .iter()
            .filter(|(_, p)| p.direction == Direction::Output)
        {
            self.module
                .port_wires
                .insert(name.clone(), interface.port(name));
        }
    }
}

impl DigitalData {
    /// Creates a new [`DigitalData`].
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            modules: GenerationMap::new(),
        }
    }

    /// Returns the ID of a generated module if it already exists or generates a new ID.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn get_generated_id<T>(
        &mut self,
        params: &T::Params,
    ) -> GeneratedCheck<DigitalModuleKey, DigitalModuleKey>
    where
        T: DigitalComponent,
    {
        self.modules.get_id(ParamKey::from_params::<T>(params))
    }

    /// Returns the generated module if it already exists or generates a new ID.
    #[allow(dead_code)]
    pub(crate) fn get_generated_module<T>(
        &mut self,
        params: &T::Params,
    ) -> GeneratedCheck<Arc<DigitalModule>, DigitalModuleKey>
    where
        T: DigitalComponent,
    {
        self.modules.get(ParamKey::from_params::<T>(params))
    }

    /// Adds a module to the map based on its [`DigitalModuleKey`].
    pub(crate) fn set_module(&mut self, module: DigitalModule) -> Arc<DigitalModule> {
        self.modules.set(module.id(), module.name().clone(), module)
    }

    /// Gets a module from the map based on its [`DigitalModuleKey`].
    #[allow(dead_code)]
    pub(crate) fn get_by_id(
        &self,
        id: DigitalModuleKey,
    ) -> crate::error::Result<&Arc<DigitalModule>> {
        self.modules.get_by_id(id)
    }

    /// Generates a new [`DigitalModuleKey`] to allow for a new digital module to be created.
    #[allow(dead_code)]
    pub(crate) fn gen_id(&mut self) -> DigitalModuleKey {
        self.modules.gen_id()
    }

    /// Allocates an unused name derived from the given base name.
    #[allow(dead_code)]
    pub(crate) fn alloc_name(&self, base_name: impl Into<ArcStr>) -> ArcStr {
        self.modules.alloc_name(base_name)
    }

    /// Returns an iterator over the modules in the map.
    #[allow(dead_code)]
    pub(crate) fn module(&self) -> impl Iterator<Item = &Arc<DigitalModule>> {
        self.modules.values()
    }
}
