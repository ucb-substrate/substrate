use std::collections::{BinaryHeap, HashMap, HashSet};
use std::ops::{Deref, DerefMut};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use super::simulation::waveform::{EdgeDir, SharedWaveform, TimeWaveform};
use super::simulation::{Simulator, TranData};
use crate::pdk::corner::Pvt;
use crate::schematic::circuit::{InstanceKey, Reference};
use crate::schematic::context::ModuleKey;
use crate::schematic::netlist::preprocess::PreprocessedNetlist;
use crate::schematic::signal::{NamedSignalPathBuf, SignalInfo, SignalPathBuf, Slice, SliceOne};
use crate::search::{search, SearchSide};

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

impl<K1, K2, V> Lut2<K1, K2, V>
where
    K1: Ord,
    K2: Ord,
{
    pub fn get(&self, k1: &K1, k2: &K2) -> Option<&V> {
        let i1 = match self.k1.binary_search(k1) {
            Ok(x) => x,
            Err(x) => x,
        };
        let i2 = match self.k2.binary_search(k2) {
            Ok(x) => x,
            Err(x) => x,
        };
        Some(self.values.get(i1)?.get(i2)?)
    }
}

impl FloatLut2 {
    pub fn getf(&self, k1: &f64, k2: &f64) -> Option<&f64> {
        let i1 = match self.k1.binary_search(k1) {
            Ok(x) => x,
            Err(x) => x,
        };
        let i2 = match self.k2.binary_search(k2) {
            Ok(x) => x,
            Err(x) => x,
        };
        Some(self.values.get(i1)?.get(i2)?)
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
    pub(crate) pvt: Pvt,
    pub(crate) port: SliceOne,
    pub(crate) related_port: SliceOne,
    pub(crate) related_port_transition: EdgeDir,
    // TODO: decide how to specify conditions.
    // cond: Arc<dyn Fn(TimingInstance) -> bool>,
    pub(crate) kind: ConstraintKind,
    /// Timing for the falling edge of `port`
    #[builder(setter(into))]
    pub(crate) fall: TimingTable,
    /// Timing for the rising edge of `port`
    #[builder(setter(into))]
    pub(crate) rise: TimingTable,
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
    pvt: Pvt,
    port: Port,
    min_pulse_width: Lut1<f64, f64>,
}

#[derive(Clone, Debug)]
pub enum TimingConstraint {
    SetupHold(SetupHoldConstraint),
    MinPulseWidth(MinPulseWidthConstraint),
}

impl TimingConstraint {
    pub fn pvt(&self) -> &Pvt {
        match self {
            Self::SetupHold(c) => &c.pvt,
            Self::MinPulseWidth(c) => &c.pvt,
        }
    }
}

/// Timing constraints referenced to the top module of a netlist.
// TODO: the option should be removed.
pub(crate) struct TopConstraint<'a, T> {
    pub(crate) constraint: &'a TimingConstraint,
    pub(crate) port: T,
    pub(crate) related_port: Option<T>,
}

type IdTopConstraint<'a> = TopConstraint<'a, SignalPathBuf>;
type NamedTopConstraint<'a> = TopConstraint<'a, NamedSignalPathBuf>;

pub(crate) struct TopConstraintDb<'a> {
    pub(crate) constraints: Vec<IdTopConstraint<'a>>,
    pub(crate) named_constraints: Option<Vec<NamedTopConstraint<'a>>>,
}

#[derive(Default, Clone, Debug)]
pub struct TimingView {
    pub(crate) constraints: Vec<TimingConstraint>,
}

#[derive(Clone, Debug)]
pub struct TimingCheck {
    slack: f64,
    time: f64,
    port: NamedSignalPathBuf,
    related_port: NamedSignalPathBuf,
}

#[derive(Debug, Clone)]
struct MinSlack(TimingCheck);

impl Deref for MinSlack {
    type Target = TimingCheck;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MinSlack {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PartialEq for MinSlack {
    fn eq(&self, other: &Self) -> bool {
        self.0.slack.eq(&other.0.slack)
    }
}

impl Eq for MinSlack {}

impl Ord for MinSlack {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.0.slack.total_cmp(&self.0.slack)
    }
}

impl PartialOrd for MinSlack {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone)]
pub struct TimingReport {
    pub(crate) setup_checks: Vec<TimingCheck>,
    pub(crate) hold_checks: Vec<TimingCheck>,
}

#[derive(Debug, Clone)]
pub(crate) struct TimingReportBuilder {
    setup_checks: BinaryHeap<MinSlack>,
    hold_checks: BinaryHeap<MinSlack>,
    capacity: usize,
}

impl TimingView {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl TimingReport {
    #[inline]
    pub fn builder() -> TimingReportBuilder {
        TimingReportBuilder::default()
    }
}

impl Default for TimingReportBuilder {
    fn default() -> Self {
        Self::with_capacity(4)
    }
}

impl TimingReportBuilder {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            setup_checks: BinaryHeap::with_capacity(capacity),
            hold_checks: BinaryHeap::with_capacity(capacity),
        }
    }

    pub fn build(self) -> TimingReport {
        TimingReport {
            setup_checks: self.setup_checks.into_iter().map(|m| m.0).collect(),
            hold_checks: self.hold_checks.into_iter().map(|m| m.0).collect(),
        }
    }
}

impl TimingReportBuilder {
    pub fn add_setup_check(&mut self, slack: f64, check: impl FnOnce() -> TimingCheck) {
        debug_assert!(self.capacity > 0);
        if self.setup_checks.len() < self.capacity {
            self.setup_checks.push(MinSlack(check()));
        } else {
            let max_slack = self.setup_checks.peek().unwrap().0.slack;
            if slack < max_slack {
                self.setup_checks.pop();
                self.setup_checks.push(MinSlack(check()));
            }
        }
    }

    pub fn add_hold_check(&mut self, slack: f64, check: impl FnOnce() -> TimingCheck) {
        debug_assert!(self.capacity > 0);
        if self.hold_checks.len() < self.capacity {
            self.hold_checks.push(MinSlack(check()));
        } else {
            let max_slack = self.hold_checks.peek().unwrap().0.slack;
            if slack < max_slack {
                self.setup_checks.pop();
                self.hold_checks.push(MinSlack(check()));
            }
        }
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

impl PreprocessedNetlist {
    /// Returns a list of the nodes that need to be captured by the simulator.
    pub(crate) fn timing_constraint_db(&self, pvt: &Pvt) -> TopConstraintDb {
        let module = &self.modules[self.top];
        let mut stack = Vec::new();
        let mut out = Vec::new();
        self.timing_helper(self.top, pvt, &mut stack, &mut out);

        TopConstraintDb {
            constraints: out,
            named_constraints: None,
        }
    }

    fn timing_helper<'a, 'b, 'c>(
        &'a self,
        module: ModuleKey,
        pvt: &'b Pvt,
        stack: &'c mut Vec<InstanceKey>,
        out: &'c mut Vec<IdTopConstraint<'a>>,
    ) {
        let module = &self.modules[module];
        for constraint in module.timing().constraints.iter() {
            if constraint.pvt() != pvt {
                continue;
            }

            let constraint = match constraint {
                TimingConstraint::SetupHold(c) => TopConstraint {
                    constraint,
                    port: self.simplify_path(SignalPathBuf::new(stack.clone(), c.port)),
                    related_port: None,
                },
                _ => todo!(),
            };

            out.push(constraint);
        }

        for (key, inst) in module.instances_iter() {
            if let Reference::Local(m) = inst.module() {
                stack.push(key);
                self.timing_helper(m.id(), pvt, stack, out);
                stack.pop().expect("stack should not be empty");
            }
        }
    }

    fn to_named_path(&self, path: &SignalPathBuf) -> NamedSignalPathBuf {
        let mut module = self.top;
        let mut insts = Vec::with_capacity(path.insts.len());
        for inst in path.insts.iter().copied() {
            let inst = &self.modules[module].instance_map()[inst];
            insts.push(inst.name().clone());
            module = inst.module().local_id().unwrap();
        }

        let sig = &self.modules[module].signals()[path.slice.signal];
        let idx = if sig.width() > 1 {
            Some(path.slice.idx)
        } else {
            None
        };

        NamedSignalPathBuf {
            insts,
            signal: sig.name().clone(),
            idx,
        }
    }
}

impl<'a> TopConstraintDb<'a> {
    fn compute_names<'b>(&mut self, netlist: &'b PreprocessedNetlist) {
        if self.named_constraints.is_some() {
            return;
        }
        let named_constraints = self
            .constraints
            .iter()
            .map(|c| NamedTopConstraint {
                constraint: c.constraint,
                port: netlist.to_named_path(&c.port),
                related_port: c.related_port.as_ref().map(|p| netlist.to_named_path(p)),
            })
            .collect();
        self.named_constraints = Some(named_constraints);
    }

    pub(crate) fn named_constraints<'b>(
        &mut self,
        netlist: &'b PreprocessedNetlist,
    ) -> impl Iterator<Item = &NamedTopConstraint> {
        self.compute_names(netlist);
        self.named_constraints.as_ref().unwrap().iter()
    }
}

pub(crate) fn verify_setup_hold_constraint(
    constraint: &SetupHoldConstraint,
    port: SharedWaveform,
    related_port: SharedWaveform,
    port_name: &NamedSignalPathBuf,
    related_port_name: &NamedSignalPathBuf,
    report: &mut TimingReportBuilder,
) {
    // if setup, check for data edges starting before `t`, then check that
    // edge's end time.
    // if hold, check for data edges ending after `t`, then check that
    // edge's start time.
    let vdd = constraint.pvt.voltage();
    let transitions = port.transitions(0.2 * vdd, 0.8 * vdd).collect::<Vec<_>>();
    for clk_edge in related_port
        .transitions(0.2 * vdd, 0.8 * vdd)
        .filter(|e| e.dir == constraint.related_port_transition)
    {
        let t = clk_edge.center_time();
        match constraint.kind {
            ConstraintKind::Setup => {
                if let Some((idx, tr)) = search(
                    &transitions,
                    |tr| tr.start_time().total_cmp(&t).into(),
                    SearchSide::Before,
                ) {
                    // convert to nanoseconds
                    let idx1 = tr.duration() * 1e9;
                    let idx2 = clk_edge.duration() * 1e9;
                    let tsu = if tr.dir().is_rising() {
                        constraint.rise.get(idx1, idx2)
                    } else {
                        constraint.fall.get(idx1, idx2)
                    };

                    let slack = t - tr.end_time() - tsu;
                    report.add_setup_check(slack, || TimingCheck {
                        slack,
                        time: t,
                        port: port_name.clone(),
                        related_port: related_port_name.clone(),
                    });
                }
            }
            ConstraintKind::Hold => {
                if let Some((idx, tr)) = search(
                    &transitions,
                    |tr| tr.end_time().total_cmp(&t).into(),
                    SearchSide::After,
                ) {
                    // check edge.t() - t > t_hold
                    // convert to nanoseconds
                    let idx1 = tr.duration() * 1e9;
                    let idx2 = clk_edge.duration() * 1e9;
                    let t_hold = if tr.dir().is_rising() {
                        constraint.rise.get(idx1, idx2)
                    } else {
                        constraint.fall.get(idx1, idx2)
                    };
                    let slack = tr.start_time() - t - t_hold;
                    report.add_hold_check(slack, || TimingCheck {
                        slack,
                        time: t,
                        port: port_name.clone(),
                        related_port: related_port_name.clone(),
                    });
                }
            }
        }
    }
}

pub(crate) fn generate_timing_report<'a>(
    constraints: impl Iterator<Item = &'a NamedTopConstraint<'a>>,
    data: &'a TranData,
    simulator: &'a dyn Simulator,
) -> TimingReport {
    let mut report = TimingReport::builder();
    for constraint in constraints {
        match constraint.constraint {
            TimingConstraint::SetupHold(c) => {
                let related_port_name = constraint.related_port.as_ref().unwrap();
                let port = data
                    .waveform(&simulator.node_voltage_string(&constraint.port))
                    .unwrap();
                let related_port = data
                    .waveform(&simulator.node_voltage_string(&related_port_name))
                    .unwrap();
                verify_setup_hold_constraint(
                    c,
                    port,
                    related_port,
                    &constraint.port,
                    related_port_name,
                    &mut report,
                );
            }
            _ => todo!(),
        };
    }
    report.build()
}
