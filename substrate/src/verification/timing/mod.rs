use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use super::simulation::waveform::EdgeDir;

pub mod context;

pub enum ConstraintKind {
    Setup,
    Hold,
}

pub struct Lut1<K1, V> {
    k1: Vec<K1>,
    values: Vec<V>,
}

pub struct Lut2<K1, K2, V> {
    k1: Vec<K1>,
    k2: Vec<K2>,
    // row major order
    values: Vec<V>,
}

pub struct TimingTable(Lut2<f64, f64, f64>);

impl Deref for TimingTable {
    type Target = Lut2<f64, f64, f64>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct SetupHoldConstraint {
    port: Port,
    related_port: Port,
    related_port_transition: EdgeDir,
    cond: Arc<dyn FnMut(TimingInstance) -> bool>,
    kind: ConstraintKind,
    // Timing for the falling edge of `port`
    fall: TimingTable,
    // Timing for the rising edge of `port`
    rise: TimingTable,
}

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
pub struct Port {}

pub struct TimingInstance {
    port_states: HashMap<Port, PortState>,
}

pub struct PortState {
    value: bool,
}

pub struct MinPulseWidthConstraint {
    port: Port,
    min_pulse_width: Lut1<f64, f64>,
}

pub enum TimingConstraint {
    SetupHold(SetupHoldConstraint),
    MinPulseWidth(MinPulseWidthConstraint),
}
