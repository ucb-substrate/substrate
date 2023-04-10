use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

use super::wire::WireKey;
use crate::deps::arcstr::ArcStr;

new_key_type! {
    /// A key identifying an RTL-level [`DigitalModule`].
    pub struct DigitalModuleKey;
}

#[derive(Debug)]
pub struct DigitalModule {
    pub(crate) id: DigitalModuleKey,
    pub(crate) name: ArcStr,
    pub(crate) ports: HashMap<ArcStr, Port>,
    pub(crate) port_wires: HashMap<ArcStr, WireKey>,
    pub(crate) instances: Vec<Instance>,
}

#[derive(Debug)]
pub struct Port {
    pub direction: Direction,
    pub name: ArcStr,
}

#[derive(Debug)]
pub struct Instance {
    pub(crate) name: ArcStr,
    pub(crate) connections: HashMap<ArcStr, WireKey>,
    /// A pointer to the reference module.
    pub(crate) module: Arc<DigitalModule>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum Direction {
    Input,
    Output,
}

impl DigitalModule {
    #[inline]
    pub fn id(&self) -> DigitalModuleKey {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    #[inline]
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = name.into();
    }

    pub fn new(id: DigitalModuleKey) -> Self {
        Self {
            id,
            name: arcstr::literal!("unnamed"),
            ports: HashMap::new(),
            port_wires: HashMap::new(),
            instances: Vec::new(),
        }
    }
}

impl Instance {
    pub fn new(module: impl Into<Arc<DigitalModule>>) -> Self {
        let module = module.into();
        Self {
            name: module.name().clone(),
            connections: HashMap::new(),
            module,
        }
    }

    #[inline]
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = name.into();
    }

    #[inline]
    pub fn module(&self) -> &Arc<DigitalModule> {
        &self.module
    }
}
