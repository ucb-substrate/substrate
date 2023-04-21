use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use self::lut::{FloatLut1, FloatLut2};

use super::simulation::waveform::{EdgeDir, SharedWaveform, TimeWaveform};
use crate::pdk::corner::Pvt;
use crate::schematic::circuit::{InstanceKey, Reference};
use crate::schematic::context::ModuleKey;
use crate::schematic::netlist::preprocess::PreprocessedNetlist;
use crate::schematic::signal::{NamedSignalPathBuf, SignalInfo, SignalPathBuf, Slice, SliceOne};
use crate::search::{search, SearchSide};

pub mod context;
pub mod lut;

new_key_type! {
    /// A key for referencing signals in the timing API.
    pub struct TimingSignalKey;
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
                    // TODO handle extrapolation and add warning
                    let tsu = if tr.dir().is_rising() {
                        constraint.rise.getf(idx1, idx2).unwrap()
                    } else {
                        constraint.fall.getf(idx1, idx2).unwrap()
                    };
                    // TODO return an Error instead of panicking
                    assert!(t - tr.end_time() > tsu);
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
                    // TODO handle extrapolation and add warning
                    let t_hold = if tr.dir().is_rising() {
                        constraint.rise.getf(idx1, idx2).unwrap()
                    } else {
                        constraint.fall.getf(idx1, idx2).unwrap()
                    };
                    // TODO return an Error instead of panicking
                    assert!(tr.start_time() - t > t_hold);
                }
            }
        }
    }
}
