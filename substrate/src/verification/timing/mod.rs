use std::collections::{HashMap, HashSet};
use std::ops::Deref;

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};

use super::simulation::waveform::EdgeDir;
use crate::pdk::corner::Pvt;
use crate::schematic::circuit::{InstanceKey, Reference};
use crate::schematic::context::ModuleKey;
use crate::schematic::netlist::preprocess::PreprocessedNetlist;
use crate::schematic::signal::{NamedSignalPathBuf, SignalInfo, SignalPathBuf, Slice, SliceOne};

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
    pvt: Pvt,
    port: SliceOne,
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
