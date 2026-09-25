use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SecondaryMap, SlotMap, SparseSecondaryMap};

new_key_type! {
    pub struct VarKey;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicPath {
    segments: Vec<Segment>,
    variables: SlotMap<VarKey, VarState>,
    min_var_value: f64,
}

impl Default for LogicPath {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            variables: Default::default(),
            min_var_value: 1.,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct VarState {
    value: f64,
}

impl Default for VarState {
    fn default() -> Self {
        Self { value: 1f64 }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Gradient(SecondaryMap<VarKey, f64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Segment {
    /// Series element.
    element: Element,
    /// Fixed capacitance added to the output node of `element`.
    fixed_cap: f64,
    /// Size-dependent capacitance added to the output node of `element`.
    variable_cap: SparseSecondaryMap<VarKey, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Element {
    SizedGate(GateModel),
    UnsizedGate(GateModel, VarKey),
    Resistor(f64),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GateModel {
    /// Pull up/down resistance.
    pub res: f64,
    /// Input capacitance.
    pub cin: f64,
    /// Output capacitance.
    pub cout: f64,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct WireModel {
    /// Total series resistance.
    pub res: f64,
    /// Total parallel capacitance to ground.
    pub cap: f64,
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct OptimizerOpts {
    /// Initial learning rate.
    pub lr: f64,
    /// Learning rate decay. Should be between 0 and 1.
    pub lr_decay: f64,
    /// Maximum number of iterations.
    pub max_iter: usize,
}

impl Default for OptimizerOpts {
    fn default() -> Self {
        Self {
            lr: 0.2,
            lr_decay: 0.9999,
            max_iter: 10_000,
        }
    }
}

impl LogicPath {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_min_var_value(&mut self, min_var_value: f64) {
        self.min_var_value = min_var_value;
    }

    pub fn create_variable(&mut self) -> VarKey {
        self.variables.insert(VarState::default())
    }

    pub fn create_variable_with_initial(&mut self, value: f64) -> VarKey {
        self.variables.insert(VarState { value })
    }

    pub fn append_resistor(&mut self, r: f64) {
        self.segments.push(Segment::new(Element::Resistor(r)));
    }

    pub fn append_sized_gate(&mut self, gate: GateModel) {
        self.segments.push(Segment::new(Element::SizedGate(gate)));
    }

    pub fn append_unsized_gate(&mut self, gate: GateModel, var: VarKey) {
        self.segments
            .push(Segment::new(Element::UnsizedGate(gate, var)));
    }

    pub fn append_capacitor(&mut self, c: f64) {
        self.segments
            .last_mut()
            .expect("cannot place capacitances at the input of a `LogicPath`")
            .fixed_cap += c;
    }

    /// Appends a capacitance of value `mult * var`.
    ///
    /// Can be used to model branching effort in digital logic paths.
    pub fn append_variable_capacitor(&mut self, mult: f64, var: VarKey) {
        let entry = self
            .segments
            .last_mut()
            .expect("cannot place capacitances at the input of a `LogicPath`")
            .variable_cap
            .entry(var)
            .unwrap()
            .or_insert(0.0);
        *entry += mult;
    }

    /// Adds a pi model of the given wire.
    ///
    /// The pi model contains 2 capacitors to ground: one at the input,
    /// and one at the output. Each has capacitance `wire.cap/2`.
    ///
    /// The two capacitors are connected by one series resistor of value `wire.res`.
    pub fn append_wire(&mut self, wire: WireModel) {
        self.append_capacitor(wire.cap / 2.0);
        self.append_resistor(wire.res);
        self.append_capacitor(wire.cap / 2.0);
    }

    pub fn size(&mut self) {
        self.size_with_opts(OptimizerOpts::default());
    }

    pub fn size_with_opts(&mut self, opts: OptimizerOpts) {
        assert!(opts.lr_decay > 0.0);
        assert!(opts.lr_decay <= 1.0);
        assert!(opts.lr > 0.0);
        assert!(opts.max_iter > 0);

        let mut lr = opts.lr;
        // Backtracking: a step that increased the delay is undone and retried at
        // half the size. A fixed step overshoots on the `res / size` terms near
        // small sizes. `delay_grad` already returns the delay, so the check is free.
        let n = self.variables.len();
        let mut base = vec![0.0; n];
        let mut base_grad = vec![0.0; n];
        let mut base_delay = f64::INFINITY;
        let mut step = lr;
        let mut iter = 0;
        while iter < opts.max_iter {
            let mut grad = self.zero_grad();
            let delay = self.delay_grad(&mut grad);
            // The tolerance ignores rounding noise once converged; without it, about
            // half of all steps near the optimum are rejected for nothing.
            if delay > base_delay * (1.0 + 1e-9) {
                step *= 0.5;
                if step == 0.0 {
                    self.set_values(&base);
                    return;
                }
            } else {
                base_delay = delay;
                for (i, (v, s)) in self.variables.iter().enumerate() {
                    base[i] = s.value;
                    base_grad[i] = grad[v];
                }
                step = lr;
                lr *= opts.lr_decay;
                iter += 1;
            }
            for (i, s) in self.variables.values_mut().enumerate() {
                // Project back onto the feasible region. Without this, a variable
                // pinned at `min_var_value` keeps drifting below it (`value()` hides
                // this) and cannot recover if its optimum later moves above the bound.
                s.value = f64::max(base[i] - step * base_grad[i], self.min_var_value);
            }
        }
        // The last step has not been checked yet.
        if self.delay() > base_delay * (1.0 + 1e-9) {
            self.set_values(&base);
        }
    }

    fn set_values(&mut self, values: &[f64]) {
        for (s, &x) in self.variables.values_mut().zip(values) {
            s.value = x;
        }
    }

    pub fn delay(&self) -> f64 {
        let mut tau = 0.0;
        for idx in 0..self.segments.len() {
            tau += self.segment_delay(idx);
        }
        tau
    }

    pub(crate) fn delay_grad(&self, grad: &mut Gradient) -> f64 {
        let mut tau = 0.0;
        for idx in 0..self.segments.len() {
            tau += self.segment_delay_grad(idx, grad);
        }
        tau
    }

    fn segment_delay(&self, idx: usize) -> f64 {
        let seg = &self.segments[idx];

        let (r, mut c) = match &seg.element {
            Element::Resistor(r) => (*r, 0.0),
            Element::SizedGate(gate) => (gate.res, gate.cout),
            &Element::UnsizedGate(gate, v) => (gate.res / self.value(v), gate.cout * self.value(v)),
        };
        c += seg.fixed_cap;
        for (v, mult) in seg.variable_cap.iter() {
            c += self.value(v) * mult;
        }
        c += self.elmore_input_capacitance(idx + 1);

        r * c
    }

    #[inline]
    pub fn value(&self, var: VarKey) -> f64 {
        f64::max(self.variables[var].value, self.min_var_value)
    }

    fn segment_delay_grad(&self, idx: usize, grad: &mut Gradient) -> f64 {
        let seg = &self.segments[idx];

        // Gradient of resistance and capacitance, respectively.
        let mut drdv = self.zero_grad();
        let mut dcdv = self.zero_grad();

        let (r, mut c) = match &seg.element {
            Element::Resistor(r) => (*r, 0.0),
            Element::SizedGate(gate) => (gate.res, gate.cout),
            Element::UnsizedGate(gate, v) => {
                let v = *v;
                drdv[v] = -gate.res / (self.value(v) * self.value(v));
                dcdv[v] = dcdv.get(v) + gate.cout;
                (gate.res / self.value(v), gate.cout * self.value(v))
            }
        };
        c += seg.fixed_cap;
        for (v, mult) in seg.variable_cap.iter() {
            c += self.value(v) * mult;
            dcdv[v] += mult;
        }
        c += self.elmore_input_capacitance_grad(idx + 1, &mut dcdv);

        for v in self.variables.keys() {
            // Apply the product rule.
            grad[v] += r * dcdv[v] + c * drdv[v];
        }

        r * c
    }

    fn total_output_cap(&self, segment: &Segment) -> f64 {
        let c = match &segment.element {
            Element::Resistor(_) => 0.0,
            Element::SizedGate(gate) => gate.cout,
            &Element::UnsizedGate(gate, v) => gate.cout * self.value(v),
        };

        c + segment.fixed_cap
    }

    fn total_output_cap_grad(&self, segment: &Segment, grad: &mut Gradient) -> f64 {
        let c = match &segment.element {
            Element::Resistor(_) => 0.0,
            Element::SizedGate(gate) => gate.cout,
            &Element::UnsizedGate(gate, v) => {
                grad[v] = grad.get(v) + gate.cout;
                gate.cout * self.value(v)
            }
        };

        c + segment.fixed_cap
    }

    fn elmore_input_capacitance(&self, mut idx: usize) -> f64 {
        let mut c = 0.0;
        loop {
            if idx >= self.segments.len() {
                return c;
            }
            let seg = &self.segments[idx];
            match &seg.element {
                Element::Resistor(_) => {
                    c += self.total_output_cap(seg);
                }
                Element::SizedGate(gate) => {
                    c += gate.cin;
                    break;
                }
                &Element::UnsizedGate(gate, v) => {
                    c += gate.cin * self.value(v);
                    break;
                }
            }
            idx += 1;
        }

        c
    }

    fn elmore_input_capacitance_grad(&self, mut idx: usize, grad: &mut Gradient) -> f64 {
        let mut c = 0.0;
        loop {
            if idx >= self.segments.len() {
                return c;
            }
            let seg = &self.segments[idx];
            match &seg.element {
                Element::Resistor(_) => {
                    c += self.total_output_cap_grad(seg, grad);
                }
                Element::SizedGate(gate) => {
                    c += gate.cin;
                    break;
                }
                &Element::UnsizedGate(gate, v) => {
                    c += gate.cin * self.value(v);
                    grad[v] += gate.cin;
                    break;
                }
            }
            idx += 1;
        }

        c
    }

    pub(crate) fn zero_grad(&self) -> Gradient {
        let mut grad = Gradient(SecondaryMap::with_capacity(self.variables.len()));
        for v in self.variables.keys() {
            grad.0.insert(v, 0f64);
        }
        grad
    }
}

impl Segment {
    pub fn new(element: impl Into<Element>) -> Self {
        Self {
            element: element.into(),
            fixed_cap: 0.0,
            variable_cap: SparseSecondaryMap::new(),
        }
    }
}

impl std::ops::Mul<f64> for GateModel {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            res: self.res / rhs,
            cin: self.cin * rhs,
            cout: self.cout * rhs,
        }
    }
}

impl std::ops::Mul<GateModel> for f64 {
    type Output = GateModel;
    fn mul(self, rhs: GateModel) -> Self::Output {
        GateModel {
            res: rhs.res / self,
            cin: rhs.cin * self,
            cout: rhs.cout * self,
        }
    }
}

impl Gradient {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&self, key: VarKey) -> f64 {
        self.0.get(key).copied().unwrap_or_default()
    }
}

impl std::ops::Index<VarKey> for Gradient {
    type Output = f64;
    fn index(&self, index: VarKey) -> &Self::Output {
        self.0.index(index)
    }
}

impl std::ops::IndexMut<VarKey> for Gradient {
    fn index_mut(&mut self, index: VarKey) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}
