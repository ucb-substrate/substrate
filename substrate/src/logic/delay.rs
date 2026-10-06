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

        let path = DensePath::new(self);
        let mut values: Vec<f64> = self.variables.values().map(|s| s.value).collect();
        let n = values.len();
        let mut grad = vec![0.0; n];
        let mut partials = Vec::new();

        let mut lr = opts.lr;
        // Backtracking: a step that increased the delay is undone and retried at
        // half the size. A fixed step overshoots on the `res / size` terms near
        // small sizes. `delay_grad` already returns the delay, so the check is free.
        let mut base = vec![0.0; n];
        let mut base_grad = vec![0.0; n];
        let mut base_delay = f64::INFINITY;
        let mut step = lr;
        let mut iter = 0;
        while iter < opts.max_iter {
            grad.fill(0.0);
            let delay = path.delay_grad(&values, &mut grad, &mut partials);
            // The tolerance ignores rounding noise once converged; without it, about
            // half of all steps near the optimum are rejected for nothing.
            let accepted = if delay > base_delay * (1.0 + 1e-9) {
                step *= 0.5;
                if step == 0.0 {
                    self.set_values(&base);
                    return;
                }
                false
            } else {
                base_delay = delay;
                base.copy_from_slice(&values);
                base_grad.copy_from_slice(&grad);
                step = lr;
                lr *= opts.lr_decay;
                iter += 1;
                true
            };
            let mut moved = false;
            for ((value, &b), &g) in values.iter_mut().zip(&base).zip(&base_grad) {
                // Project back onto the feasible region. Without this, a variable
                // pinned at `min_var_value` keeps drifting below it (`value()` hides
                // this) and cannot recover if its optimum later moves above the bound.
                let next = f64::max(b - step * g, self.min_var_value);
                moved |= next.to_bits() != value.to_bits();
                *value = next;
            }
            // Every later step starts from these same values, so it has the same
            // gradient and is accepted, and its step size is no larger: rounding
            // returns each value to the same bits again. The remaining iterations
            // would change nothing.
            if accepted && !moved {
                break;
            }
        }
        self.set_values(&values);
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

    fn total_output_cap(&self, segment: &Segment) -> f64 {
        let c = match &segment.element {
            Element::Resistor(_) => 0.0,
            Element::SizedGate(gate) => gate.cout,
            &Element::UnsizedGate(gate, v) => gate.cout * self.value(v),
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
}

/// A [`LogicPath`] with its variables numbered in creation order, so the optimizer
/// can work on flat vectors.
///
/// [`DensePath::delay_grad`] performs the same floating-point operations in the same
/// order as the test-only `LogicPath::delay_grad`, so it returns bit-identical results.
/// It differs only in skipping the zero partials of variables a segment does not touch.
struct DensePath {
    segments: Vec<DenseSegment>,
    min_var_value: f64,
}

struct DenseSegment {
    element: DenseElement,
    fixed_cap: f64,
    /// `(variable, multiplier)` pairs, in the iteration order of
    /// [`Segment::variable_cap`], which is the order their capacitances are summed in.
    variable_cap: Vec<(usize, f64)>,
}

enum DenseElement {
    SizedGate(GateModel),
    UnsizedGate(GateModel, usize),
    Resistor(f64),
}

impl DensePath {
    fn new(path: &LogicPath) -> Self {
        let mut index = SecondaryMap::with_capacity(path.variables.len());
        for (i, v) in path.variables.keys().enumerate() {
            index.insert(v, i);
        }
        let segments = path
            .segments
            .iter()
            .map(|seg| DenseSegment {
                element: match &seg.element {
                    Element::SizedGate(gate) => DenseElement::SizedGate(*gate),
                    &Element::UnsizedGate(gate, v) => DenseElement::UnsizedGate(gate, index[v]),
                    Element::Resistor(r) => DenseElement::Resistor(*r),
                },
                fixed_cap: seg.fixed_cap,
                variable_cap: seg
                    .variable_cap
                    .iter()
                    .map(|(v, &mult)| (index[v], mult))
                    .collect(),
            })
            .collect();
        Self {
            segments,
            min_var_value: path.min_var_value,
        }
    }

    #[inline]
    fn value(&self, values: &[f64], var: usize) -> f64 {
        f64::max(values[var], self.min_var_value)
    }

    /// Returns the delay and adds its gradient into `grad`.
    ///
    /// `partials` is scratch space, reused across calls to avoid allocating.
    fn delay_grad(
        &self,
        values: &[f64],
        grad: &mut [f64],
        partials: &mut Vec<(usize, f64)>,
    ) -> f64 {
        let mut tau = 0.0;
        for idx in 0..self.segments.len() {
            tau += self.segment_delay_grad(idx, values, grad, partials);
        }
        tau
    }

    fn segment_delay_grad(
        &self,
        idx: usize,
        values: &[f64],
        grad: &mut [f64],
        dcdv: &mut Vec<(usize, f64)>,
    ) -> f64 {
        let seg = &self.segments[idx];

        // The capacitance's nonzero partials, as `(variable, partial)`. Only the
        // element's own variable has a nonzero partial of the resistance.
        dcdv.clear();
        let mut drdv = None;

        let (r, mut c) = match seg.element {
            DenseElement::Resistor(r) => (r, 0.0),
            DenseElement::SizedGate(gate) => (gate.res, gate.cout),
            DenseElement::UnsizedGate(gate, v) => {
                drdv = Some((
                    v,
                    -gate.res / (self.value(values, v) * self.value(values, v)),
                ));
                *partial(dcdv, v) += gate.cout;
                (
                    gate.res / self.value(values, v),
                    gate.cout * self.value(values, v),
                )
            }
        };
        c += seg.fixed_cap;
        for &(v, mult) in &seg.variable_cap {
            c += self.value(values, v) * mult;
            *partial(dcdv, v) += mult;
        }
        c += self.elmore_input_capacitance_grad(idx + 1, values, dcdv);

        // Apply the product rule.
        for &(v, dc) in dcdv.iter() {
            let dr = match drdv {
                Some((u, dr)) if u == v => dr,
                _ => 0.0,
            };
            grad[v] += r * dc + c * dr;
        }
        // Every other variable gains `r * 0.0 + c * 0.0`. Gradient entries start at
        // `+0.0` and only have values added to them, so they are never `-0.0`, and
        // adding a zero leaves them unchanged. Only a NaN, from an infinite `r` or
        // `c`, changes them.
        let zero = r * 0.0 + c * 0.0;
        if zero.is_nan() {
            for (v, g) in grad.iter_mut().enumerate() {
                if !dcdv.iter().any(|&(u, _)| u == v) {
                    *g += zero;
                }
            }
        }

        r * c
    }

    fn elmore_input_capacitance_grad(
        &self,
        mut idx: usize,
        values: &[f64],
        dcdv: &mut Vec<(usize, f64)>,
    ) -> f64 {
        let mut c = 0.0;
        loop {
            if idx >= self.segments.len() {
                return c;
            }
            let seg = &self.segments[idx];
            match seg.element {
                // As in `LogicPath::total_output_cap_grad`.
                DenseElement::Resistor(_) => {
                    c += 0.0 + seg.fixed_cap;
                }
                DenseElement::SizedGate(gate) => {
                    c += gate.cin;
                    break;
                }
                DenseElement::UnsizedGate(gate, v) => {
                    c += gate.cin * self.value(values, v);
                    *partial(dcdv, v) += gate.cin;
                    break;
                }
            }
            idx += 1;
        }

        c
    }
}

/// Returns the partial for `var`, adding it as `0.0` if absent.
fn partial(partials: &mut Vec<(usize, f64)>, var: usize) -> &mut f64 {
    let i = match partials.iter().position(|&(v, _)| v == var) {
        Some(i) => i,
        None => {
            partials.push((var, 0.0));
            partials.len() - 1
        }
    };
    &mut partials[i].1
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

#[cfg(test)]
mod tests {
    use super::*;

    // The optimizer and gradient as they were before `DensePath`, kept so tests can
    // check that `size_with_opts` still returns bit-identical results.
    impl LogicPath {
        /// The raw variable values, in creation order.
        pub(crate) fn values(&self) -> Vec<f64> {
            self.variables.values().map(|s| s.value).collect()
        }

        /// [`LogicPath::size_with_opts`] without [`DensePath`] or the early exit.
        pub(crate) fn size_with_opts_reference(&mut self, opts: OptimizerOpts) {
            let mut lr = opts.lr;
            let n = self.variables.len();
            let mut base = vec![0.0; n];
            let mut base_grad = vec![0.0; n];
            let mut base_delay = f64::INFINITY;
            let mut step = lr;
            let mut iter = 0;
            while iter < opts.max_iter {
                let mut grad = self.zero_grad();
                let delay = self.delay_grad(&mut grad);
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
                    s.value = f64::max(base[i] - step * base_grad[i], self.min_var_value);
                }
            }
            if self.delay() > base_delay * (1.0 + 1e-9) {
                self.set_values(&base);
            }
        }

        pub(crate) fn delay_grad(&self, grad: &mut Gradient) -> f64 {
            let mut tau = 0.0;
            for idx in 0..self.segments.len() {
                tau += self.segment_delay_grad(idx, grad);
            }
            tau
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
}
