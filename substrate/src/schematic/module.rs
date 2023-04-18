use std::collections::HashMap;
use std::path::PathBuf;

use slotmap::SlotMap;
use subspice::parser::SubcktLine;

use super::circuit::{Direction, Instance, Param, Port, PortError, PortInfo};
use super::context::ModuleKey;
use super::signal::{SignalInfo, SignalKey, Slice};
use crate::deps::arcstr::ArcStr;
use crate::error::{ErrorSource, Result};
use crate::verification::timing::TimingView;

#[derive(Clone, Debug)]
pub struct Module {
    pub(crate) id: ModuleKey,
    name: ArcStr,
    ports: Vec<Port>,
    instances: Vec<Instance>,
    parameters: HashMap<ArcStr, Param>,
    signals: SlotMap<SignalKey, SignalInfo>,
    raw_spice: Option<ArcStr>,
    timing: TimingView,
}

impl Module {
    pub(crate) fn new(id: ModuleKey) -> Self {
        Self {
            id,
            name: arcstr::literal!("unnamed"),
            ports: Vec::new(),
            instances: Vec::new(),
            parameters: HashMap::new(),
            signals: SlotMap::with_key(),
            raw_spice: None,
            timing: Default::default(),
        }
    }

    #[inline]
    pub(crate) fn add_port(
        &mut self,
        name: impl Into<ArcStr>,
        width: usize,
        direction: Direction,
    ) -> Slice {
        let key = self.signals.insert(SignalInfo::new(name, width));
        let port = Port::new(key, direction);
        self.ports.push(port);
        Slice::with_width(key, width)
    }

    #[inline]
    pub(crate) fn add_signal(&mut self, name: impl Into<ArcStr>, width: usize) -> Slice {
        let key = self.signals.insert(SignalInfo::new(name, width));
        Slice::with_width(key, width)
    }

    #[inline]
    pub(crate) fn set_raw_spice(&mut self, s: impl Into<ArcStr>) {
        self.raw_spice = Some(s.into())
    }

    #[inline]
    pub(crate) fn add_instance(&mut self, inst: Instance) {
        self.instances.push(inst);
    }

    #[inline]
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    #[inline]
    pub(crate) fn instances_mut(&mut self) -> &mut [Instance] {
        &mut self.instances
    }

    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    #[inline]
    pub(crate) fn signals(&self) -> &SlotMap<SignalKey, SignalInfo> {
        &self.signals
    }

    #[inline]
    pub(crate) fn signals_mut(&mut self) -> &mut SlotMap<SignalKey, SignalInfo> {
        &mut self.signals
    }

    #[inline]
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = name.into();
    }

    #[inline]
    pub fn params(&self) -> &HashMap<ArcStr, Param> {
        &self.parameters
    }

    pub fn signal_width(&self, key: SignalKey) -> Option<usize> {
        self.signals.get(key).map(|r| r.width())
    }

    #[inline]
    pub(crate) fn raw_spice(&self) -> Option<&str> {
        self.raw_spice.as_deref()
    }

    #[inline]
    pub(crate) fn timing(&self) -> &TimingView {
        &self.timing
    }

    #[inline]
    pub(crate) fn timing_mut(&mut self) -> &mut TimingView {
        &mut self.timing
    }

    #[inline]
    pub fn id(&self) -> ModuleKey {
        self.id
    }

    pub fn port(&self, name: &str) -> std::result::Result<PortInfo, PortError> {
        let p = self
            .ports
            .iter()
            .find(|p| self.signals[p.signal].name() == name)
            .ok_or_else(|| PortError::PortNotFound(name.into()))?;
        Ok(self.convert_to_port_info(*p))
    }

    fn convert_to_port_info(&self, p: Port) -> PortInfo {
        let info = &self.signals[p.signal];
        PortInfo {
            direction: p.direction,
            name: info.name().clone(),
            width: info.width(),
            signal: p.signal,
        }
    }

    pub fn ports(&self) -> impl Iterator<Item = PortInfo> + '_ {
        self.ports.iter().map(|p| self.convert_to_port_info(*p))
    }
}

#[derive(Clone, Debug)]
pub struct ExternalModule {
    pub(crate) name: ArcStr,
    pub(crate) ports: Vec<Port>,
    pub(crate) parameters: HashMap<ArcStr, Param>,
    pub(crate) signals: SlotMap<SignalKey, SignalInfo>,
    pub(crate) source: RawSource,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum RawSource {
    /// Include a literal spice string in generated netlists.
    Literal(ArcStr),
    /// Include a spice file in generated netlists.
    File(PathBuf),
    /// Do not emit any information for the [`ExternalModule`].
    ///
    /// Users will have to manually include or link to
    /// the file that defines this module.
    #[default]
    ManualInclude,
}

impl RawSource {
    #[inline]
    pub fn with_literal<T>(literal: T) -> Self
    where
        T: Into<ArcStr>,
    {
        Self::Literal(literal.into())
    }

    #[inline]
    pub fn with_file<T>(path: T) -> Self
    where
        T: Into<PathBuf>,
    {
        Self::File(path.into())
    }
}

impl From<ArcStr> for RawSource {
    fn from(value: ArcStr) -> Self {
        Self::Literal(value)
    }
}

impl From<PathBuf> for RawSource {
    fn from(value: PathBuf) -> Self {
        Self::File(value)
    }
}

#[derive(Clone, Default)]
pub struct ExternalModuleBuilder {
    name: Option<ArcStr>,
    ports: Vec<Port>,
    signals: SlotMap<SignalKey, SignalInfo>,
    // TODO: support parameters
    source: RawSource,
}

impl ExternalModule {
    pub fn builder() -> ExternalModuleBuilder {
        ExternalModuleBuilder::new()
    }

    pub fn from_spice_literal(name: impl Into<ArcStr>, spice: impl Into<ArcStr>) -> Result<Self> {
        let spice = spice.into();
        let name = name.into();

        let parsed_spice = subspice::parse(&spice)?;
        let subckt = parsed_spice
            .subcircuit_named(&name)
            .ok_or_else(|| ErrorSource::ModuleNotFound(name.to_string()))?;

        let module = ExternalModuleBuilder::from_subckt(name, subckt)
            .source(spice)
            .build();

        Ok(module)
    }

    pub fn from_spice_file(name: impl Into<ArcStr>, path: impl Into<PathBuf>) -> Result<Self> {
        let name = name.into();
        let path = path.into();
        let spice = crate::io::read_to_string(&path)?;

        let parsed_spice = subspice::parse(&spice)?;
        let subckt = parsed_spice
            .subcircuit_named(&name)
            .ok_or_else(|| ErrorSource::ModuleNotFound(name.to_string()))?;

        let module = ExternalModuleBuilder::from_subckt(name, subckt)
            .source(path)
            .build();

        Ok(module)
    }

    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    #[inline]
    pub(crate) fn signals(&self) -> &SlotMap<SignalKey, SignalInfo> {
        &self.signals
    }

    #[inline]
    pub fn params(&self) -> &HashMap<ArcStr, Param> {
        &self.parameters
    }

    #[inline]
    pub fn signal_width(&self, key: SignalKey) -> Option<usize> {
        self.signals.get(key).map(|r| r.width())
    }

    #[inline]
    pub fn source(&self) -> &RawSource {
        &self.source
    }
}

pub(crate) trait AbstractModule {
    fn port_info(&self, port: &Port) -> &SignalInfo;
    fn raw_ports(&self) -> &Vec<Port>;
}

impl AbstractModule for ExternalModule {
    #[inline]
    fn raw_ports(&self) -> &Vec<Port> {
        &self.ports
    }

    #[inline]
    fn port_info(&self, port: &Port) -> &SignalInfo {
        &self.signals[port.signal]
    }
}

impl AbstractModule for Module {
    #[inline]
    fn raw_ports(&self) -> &Vec<Port> {
        &self.ports
    }

    #[inline]
    fn port_info(&self, port: &Port) -> &SignalInfo {
        &self.signals[port.signal]
    }
}

impl ExternalModuleBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn add_port(mut self, name: impl Into<ArcStr>, width: usize, direction: Direction) -> Self {
        let key = self.signals.insert(SignalInfo::new(name, width));
        let port = Port::new(key, direction);
        self.ports.push(port);
        self
    }

    #[inline]
    pub fn name(mut self, name: impl Into<ArcStr>) -> Self {
        self.name = Some(name.into());
        self
    }

    #[inline]
    pub fn source(mut self, source: impl Into<RawSource>) -> Self {
        self.source = source.into();
        self
    }

    #[inline]
    pub fn build(self) -> ExternalModule {
        ExternalModule {
            name: self.name.unwrap(),
            ports: self.ports,
            parameters: HashMap::new(),
            signals: self.signals,
            source: self.source,
        }
    }

    pub(crate) fn from_subckt(name: impl Into<ArcStr>, subckt: &SubcktLine) -> Self {
        let mut builder = Self::new().name(name);

        for port in subckt.ports.iter() {
            builder = builder.add_port(*port, 1, Direction::InOut);
        }

        builder
    }
}
