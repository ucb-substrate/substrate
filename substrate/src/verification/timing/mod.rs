use std::collections::HashMap;
use std::ops::Deref;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use super::simulation::waveform::EdgeDir;
use crate::schematic::signal::{SignalInfo, Slice, SliceOne};

pub mod context;

new_key_type! {
    /// A key for referencing signals in the timing API.
    pub struct TimingSignalKey;
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    Setup,
    Hold,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Builder, Serialize, Deserialize)]
#[builder(pattern = "owned")]
pub struct Lut1<K1, V> {
    k1: Vec<K1>,
    values: Vec<V>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Builder, Serialize, Deserialize)]
#[builder(pattern = "owned")]
pub struct Lut2<K1, K2, V> {
    k1: Vec<K1>,
    k2: Vec<K2>,
    // row major order
    values: Vec<Vec<V>>,
}

impl<K1, K2, V> Lut2<K1, K2, V> {
    pub fn builder() -> Lut2Builder<K1, K2, V> {
        Default::default()
    }
}

type FloatLut1 = Lut1<f64, f64>;
type FloatLut2 = Lut2<f64, f64, f64>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingTable(FloatLut2);

impl From<FloatLut2> for TimingTable {
    fn from(value: FloatLut2) -> Self {
        Self(value)
    }
}

impl Deref for TimingTable {
    type Target = Lut2<f64, f64, f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, Builder)]
pub struct SetupHoldConstraint {
    port: Slice,
    related_port: SliceOne,
    related_port_transition: EdgeDir,
    // TODO: decide how to specify conditions.
    // cond: Arc<dyn Fn(TimingInstance) -> bool>,
    kind: ConstraintKind,
    /// Timing for the falling edge of `port`
    #[builder(setter(into))]
    fall: TimingTable,
    /// Timing for the rising edge of `port`
    #[builder(setter(into))]
    rise: TimingTable,
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Port {}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TimingInstance {
    port_states: HashMap<Port, PortState>,
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortState {
    value: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MinPulseWidthConstraint {
    port: Port,
    min_pulse_width: Lut1<f64, f64>,
}

#[derive(Clone, Debug)]
pub enum TimingConstraint {
    SetupHold(SetupHoldConstraint),
    MinPulseWidth(MinPulseWidthConstraint),
}

#[derive(Default, Clone, Debug)]
pub struct TimingView {
    pub(crate) constraints: Vec<TimingConstraint>,
}

impl TimingView {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<SetupHoldConstraint> for TimingConstraint {
    fn from(value: SetupHoldConstraint) -> Self {
        Self::SetupHold(value)
    }
}

impl From<MinPulseWidthConstraint> for TimingConstraint {
    fn from(value: MinPulseWidthConstraint) -> Self {
        Self::MinPulseWidth(value)
    }
}

impl SetupHoldConstraint {
    #[inline]
    pub fn builder() -> SetupHoldConstraintBuilder {
        SetupHoldConstraintBuilder::default()
    }
}
