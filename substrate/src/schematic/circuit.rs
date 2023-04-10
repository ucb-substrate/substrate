//! Circuit primitives for schematic generation.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::context::{ModuleKey, SchematicCtx};
use super::module::Module;
use super::signal::{Signal, SignalKey};
use crate::deps::arcstr::ArcStr;

/// A signal exposed by a [`Module`](super::module::Module).
#[derive(Copy, Eq, PartialEq, Clone, Debug)]
pub struct Port {
    pub(crate) signal: SignalKey,
    pub(crate) direction: Direction,
}

impl Port {
    /// Create a new port exposing the given signal with the given [`Direction`].
    #[inline]
    pub(crate) fn new(signal: SignalKey, direction: Direction) -> Self {
        Self { signal, direction }
    }

    /// Get the [`Direction`] of this [`Port`].
    #[inline]
    pub fn direction(&self) -> Direction {
        self.direction
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct PortInfo {
    pub direction: Direction,
    pub name: ArcStr,
    pub width: usize,
    pub(crate) signal: SignalKey,
}

impl PortInfo {
    #[inline]
    pub fn direction(&self) -> Direction {
        self.direction
    }

    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }
}

/// An instance of a module in a schematic.
#[derive(Clone, Debug)]
pub struct Instance {
    /// The name of the instance.
    name: ArcStr,
    /// The instance's reference module.
    module: Reference,
    /// An unstructured map of parameters.
    params: HashMap<ArcStr, Value>,
    /// A map of connections to the instance's ports.
    connections: HashMap<ArcStr, Signal>,
}

impl Instance {
    /// Creates a new [`Instance`].
    pub fn new(module: Reference) -> Self {
        Self {
            name: arcstr::literal!("0"),
            module,
            params: HashMap::new(),
            connections: HashMap::new(),
        }
    }

    /// Connects a port of the instance to the provided signal.
    #[inline]
    pub fn connect(&mut self, port: impl Into<ArcStr>, signal: impl Into<Signal>) {
        self.connections.insert(port.into(), signal.into());
    }

    /// Connects port-signal tuples provided as an iterator.
    pub fn connect_all<I, P, S>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (P, S)>,
        P: Into<ArcStr>,
        S: Into<Signal>,
    {
        for (p, s) in iter.into_iter() {
            self.connect(p, s);
        }
    }

    /// Returns a reference to the instance's module.
    pub fn module(&self) -> Reference {
        self.module.clone()
    }

    /// Returns the name of the instance.
    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    /// Returns the parameters associated with the instance.
    #[inline]
    pub fn params(&self) -> &HashMap<ArcStr, Value> {
        &self.params
    }

    /// Returns the instance's connection map.
    #[inline]
    pub fn connections(&self) -> &HashMap<ArcStr, Signal> {
        &self.connections
    }

    /// Sets the name of the instance.
    #[inline]
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = name.into()
    }

    /// A consuming method to set the name of the instance.
    #[inline]
    pub fn named(mut self, name: impl Into<ArcStr>) -> Self {
        self.set_name(name);
        self
    }

    /// A consuming method to connect a set of ports.
    pub fn with_connections<I, P, S>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = (P, S)>,
        P: Into<ArcStr>,
        S: Into<Signal>,
    {
        self.connect_all(iter);
        self
    }

    /// A consuming method to connect a single port.
    pub fn with_connection(mut self, port: impl Into<ArcStr>, signal: impl Into<Signal>) -> Self {
        self.connect(port, signal);
        self
    }

    /// Adds this instance to the given schematic context.
    ///
    /// Equivalent to calling `ctx.add_instance(instance)`.
    ///
    /// See [`SchematicCtx::add_instance`] for more information.
    pub fn add_to(self, ctx: &mut SchematicCtx) {
        ctx.add_instance(self);
    }

    /// Lists the exposed ports of this instance's [`Module`].
    ///
    /// # Panics
    ///
    /// This function panics if called on an instance representing
    /// an external module.
    pub fn ports(&self) -> Result<impl Iterator<Item = PortInfo> + '_, PortError> {
        let module = self.module.local_ref().ok_or(PortError::ExternalModule)?;
        Ok(module.ports())
    }

    pub fn port(&self, name: &str) -> Result<PortInfo, PortError> {
        let module = self.module.local_ref().ok_or(PortError::ExternalModule)?;
        module.port(name)
    }
}

/// An enumeration of port-related errors.
#[derive(Debug, Error)]
pub enum PortError {
    /// The desired port was not found.
    #[error("port not found: {0}")]
    PortNotFound(ArcStr),

    /// Cannot list the ports of an external module.
    #[error("cannot list the ports of an external module")]
    ExternalModule,
}

/// An enumeration of reference types.
#[derive(Clone, Debug)]
pub enum Reference {
    /// A reference to a module generated locally.
    Local(Arc<Module>),
    /// A reference to a module included as an external spice file.
    External(ArcStr),
}

impl Reference {
    /// Returns a pointer to the [`Module`] referred to by a [`Reference`] if it is local,
    /// otherwise returns `None`.
    #[inline]
    pub fn local(self) -> Option<Arc<Module>> {
        match self {
            Self::Local(m) => Some(m),
            Self::External(_) => None,
        }
    }

    #[inline]
    pub fn local_ref(&self) -> Option<&Arc<Module>> {
        match self {
            Self::Local(m) => Some(m),
            Self::External(_) => None,
        }
    }
    /// Returns the [`ModuleKey`] referred to by a [`Reference`] if it is local, otherwise returns
    /// `None`.
    #[inline]
    pub fn local_id(self) -> Option<ModuleKey> {
        match self {
            Self::Local(m) => Some(m.id),
            Self::External(_) => None,
        }
    }

    /// Returns the name of a external module referred to by a [`Reference`].
    ///
    /// Returns `None` if the reference is local.
    #[inline]
    pub fn external(self) -> Option<ArcStr> {
        match self {
            Self::External(key) => Some(key),
            Self::Local(_) => None,
        }
    }
}

/// A general-purpose parameter type for schematic objects.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Param {
    name: ArcStr,
    desc: Option<ArcStr>,
    /// Value or default
    value: Value,
}

/// An enumeration of possible datatypes for a schematic parameter value.
#[derive(Clone, Debug)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Expr(String),
}

/// An enumeration of port directions.
#[derive(
    Clone, Copy, Eq, PartialEq, Hash, Default, Debug, Ord, PartialOrd, Serialize, Deserialize,
)]
pub enum Direction {
    Input,
    Output,
    #[default]
    InOut,
}
