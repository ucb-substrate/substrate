use std::collections::{BinaryHeap, HashMap};
use std::ops::{Deref, DerefMut};

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::new_key_type;
use sublut::{FloatLut1, FloatLut2};

use super::simulation::waveform::{EdgeDir, SharedWaveform, TimeWaveform};
use super::simulation::{Simulator, TranData};
use crate::log::Log;
use crate::pdk::corner::Pvt;
use crate::schematic::circuit::{InstanceKey, Reference};
use crate::schematic::context::ModuleKey;
use crate::schematic::netlist::preprocess::PreprocessedNetlist;
use crate::schematic::signal::{NamedSignalPathBuf, SignalPathBuf, SliceOne};
use crate::search::{search, SearchSide};
use crate::units::SiPrefix;

pub mod context;

new_key_type! {
    /// A key for referencing signals in the timing API.
    pub struct TimingSignalKey;
}

#[derive(Debug, Clone, PartialEq, Builder, Serialize, Deserialize)]
#[allow(clippy::needless_borrow)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct TimingConfig {
    /// The scale for PDK-provided timing information.
    ///
    /// For example, if times are specified in picoseconds, `time_unit`
    /// should be set to [`SiPrefix::Pico`]. If using nanoseconds,
    /// set `time_unit` to [`SiPrefix::Nano`].
    time_unit: SiPrefix,
    /// Thresholds for measuring transition times.
    ///
    /// The lower threshold comes first. For example, a 20%-80%
    /// threshold range would be specified as `[0.2, 0.8]`.
    ///
    /// We do not support separate thresholds for rise and fall transitions.
    slew_thresholds: [f64; 2],
}

impl TimingConfigBuilder {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref thresh) = self.slew_thresholds {
            if thresh[0] >= thresh[1] {
                return Err(format!(
                    "Upper slew threshold `{:.4}` must be larger than lower slew threshold `{:.4}`",
                    thresh[1], thresh[0]
                ));
            }
        }
        Ok(())
    }
}

impl TimingConfig {
    #[inline]
    pub fn builder() -> TimingConfigBuilder {
        TimingConfigBuilder::default()
    }

    #[inline]
    pub fn slew_lower_thresh(&self) -> f64 {
        self.slew_thresholds[0]
    }
    #[inline]
    pub fn slew_upper_thresh(&self) -> f64 {
        self.slew_thresholds[1]
    }
    #[inline]
    pub fn time_unit(&self) -> SiPrefix {
        self.time_unit
    }

    /// Converts a value in seconds to a value in units of `time_unit`.
    #[inline]
    pub fn to_time_unit(&self, value: f64) -> f64 {
        value / self.time_unit.multiplier()
    }

    /// Converts a value in units of `time_unit` to a value in seconds.
    #[inline]
    pub fn from_time_unit(&self, value: f64) -> f64 {
        value * self.time_unit.multiplier()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    Setup,
    Hold,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingTable(FloatLut2);

impl From<FloatLut2> for TimingTable {
    fn from(value: FloatLut2) -> Self {
        Self(value)
    }
}

impl Deref for TimingTable {
    type Target = FloatLut2;
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
    min_pulse_width: FloatLut1,
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

impl TimingCheck {
    #[inline]
    pub fn slack(&self) -> f64 {
        self.slack
    }

    /// The approximate simulation time at which this check was applied.
    ///
    /// Usually refers to the 50% transition point of the clock (or
    /// other related pin), though this is subject to change in future updates.
    #[inline]
    pub fn time(&self) -> f64 {
        self.time
    }

    /// The port that was constrained.
    ///
    /// For example, a setup time constraint on a flip flop constrains the D pin.
    #[inline]
    pub fn port(&self) -> &NamedSignalPathBuf {
        &self.port
    }

    /// The port related to the constraint.
    ///
    /// For example, the related port for a setup time constraint on a flip flop is CLK.
    #[inline]
    pub fn related_port(&self) -> &NamedSignalPathBuf {
        &self.related_port
    }
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
    pub(crate) fn builder() -> TimingReportBuilder {
        TimingReportBuilder::default()
    }

    pub fn is_failure(&self) -> bool {
        let setup_fail = self
            .setup_checks
            .get(0)
            .map(|c| c.slack < 0.0)
            .unwrap_or_default();
        let hold_fail = self
            .hold_checks
            .get(0)
            .map(|c| c.slack < 0.0)
            .unwrap_or_default();
        setup_fail || hold_fail
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

impl Log for TimingReport {
    fn log(&self) {
        use crate::log::*;

        if self.is_failure() {
            error!("Timing constraints not satisfied");
            for c in self.setup_checks.iter() {
                if c.slack < 0.0 {
                    error!("Setup check failed: {:?}", c);
                }
            }
            for c in self.hold_checks.iter() {
                if c.slack < 0.0 {
                    error!("Hold check failed: {:?}", c);
                }
            }
        } else {
            info!("All timing constraints satisfied");
            if let Some(c) = self.setup_checks.get(0) {
                info!("Minimum setup slack: {:?}", c);
            }
            if let Some(c) = self.hold_checks.get(0) {
                info!("Minimum hold slack: {:?}", c);
            }
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
        let mut stack = Vec::new();
        let mut out = Vec::new();
        self.timing_helper(self.top, pvt, &mut stack, &mut out);

        TopConstraintDb {
            constraints: out,
            named_constraints: None,
        }
    }

    fn timing_helper<'a, 'c>(
        &'a self,
        module: ModuleKey,
        pvt: &Pvt,
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
                    related_port: Some(
                        self.simplify_path(SignalPathBuf::new(stack.clone(), c.related_port)),
                    ),
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

    pub(crate) fn to_named_path(&self, path: &SignalPathBuf) -> NamedSignalPathBuf {
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
    fn compute_names(&mut self, netlist: &PreprocessedNetlist) {
        if self.named_constraints.is_some() {
            return;
        }
        let named_constraints = self
            .constraints
            .iter()
            .map(|c| {
                let port = netlist.to_named_path(&c.port);
                let related_port = c.related_port.as_ref().map(|p| netlist.to_named_path(p));
                NamedTopConstraint {
                    constraint: c.constraint,
                    port,
                    related_port,
                }
            })
            .collect();
        self.named_constraints = Some(named_constraints);
    }

    pub(crate) fn named_constraints(
        &mut self,
        netlist: &PreprocessedNetlist,
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
    config: &TimingConfig,
) {
    // if setup, check for data edges starting before `t`, then check that
    // edge's end time.
    // if hold, check for data edges ending after `t`, then check that
    // edge's start time.
    let vdd = constraint.pvt.voltage();
    let transitions = port
        .transitions(
            config.slew_lower_thresh() * vdd,
            config.slew_upper_thresh() * vdd,
        )
        .collect::<Vec<_>>();
    for clk_edge in related_port
        .transitions(
            config.slew_lower_thresh() * vdd,
            config.slew_upper_thresh() * vdd,
        )
        .filter(|e| e.dir == constraint.related_port_transition)
    {
        let t = clk_edge.center_time();
        match constraint.kind {
            ConstraintKind::Setup => {
                if let Some((_idx, tr)) = search(
                    &transitions,
                    |tr| tr.start_time().total_cmp(&t).into(),
                    SearchSide::Before,
                ) {
                    let idx1 = config.to_time_unit(tr.duration());
                    let idx2 = config.to_time_unit(clk_edge.duration());
                    // TODO handle extrapolation and add warning
                    let tsu = if tr.dir().is_rising() {
                        constraint.rise.getf(idx1, idx2).unwrap()
                    } else {
                        constraint.fall.getf(idx1, idx2).unwrap()
                    };

                    let tsu = config.from_time_unit(tsu);

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
                if let Some((_idx, tr)) = search(
                    &transitions,
                    |tr| tr.end_time().total_cmp(&t).into(),
                    SearchSide::After,
                ) {
                    let idx1 = config.to_time_unit(tr.duration());
                    let idx2 = config.to_time_unit(clk_edge.duration());
                    // TODO handle extrapolation and add warning
                    let t_hold = if tr.dir().is_rising() {
                        constraint.rise.getf(idx1, idx2).unwrap()
                    } else {
                        constraint.fall.getf(idx1, idx2).unwrap()
                    };

                    let t_hold = config.from_time_unit(t_hold);

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
    config: &'a TimingConfig,
) -> TimingReport {
    let mut report = TimingReport::builder();
    for constraint in constraints {
        match constraint.constraint {
            TimingConstraint::SetupHold(c) => {
                let port = &simulator.node_voltage_string(&constraint.port);
                let port = data
                    .waveform(port)
                    .unwrap_or_else(|| panic!("waveform not found: {port}"));

                let related_port_name = constraint.related_port.as_ref().unwrap();
                let related_port = &simulator.node_voltage_string(related_port_name);
                let related_port = data
                    .waveform(related_port)
                    .unwrap_or_else(|| panic!("waveform not found: {related_port}"));
                verify_setup_hold_constraint(
                    c,
                    port,
                    related_port,
                    &constraint.port,
                    related_port_name,
                    &mut report,
                    config,
                );
            }
            _ => todo!(),
        };
    }
    report.build()
}
