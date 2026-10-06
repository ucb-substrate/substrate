use float_eq::float_eq;

use super::delay::*;

pub const INV_MODEL: GateModel = GateModel {
    res: 1.0,
    cin: 3.0,
    cout: 3.0,
};
pub const NAND2_MODEL: GateModel = GateModel {
    res: 1.0,
    cin: 4.0,
    cout: 6.0,
};
pub const NAND3_MODEL: GateModel = GateModel {
    res: 1.0,
    cin: 5.0,
    cout: 9.0,
};

#[test]
fn test_elmore_delay_1() {
    let mut path = LogicPath::new();
    path.append_resistor(1.0);
    path.append_capacitor(2.0);
    path.append_resistor(8.0);
    path.append_capacitor(2.0);
    assert_eq!(path.delay(), 20.0);
}

#[test]
fn test_elmore_delay_2() {
    let mut path = LogicPath::new();
    path.append_resistor(7.0);
    path.append_capacitor(5.0);
    path.append_resistor(4.0);
    path.append_capacitor(3.0);
    path.append_resistor(2.0);
    path.append_capacitor(6.0);
    assert_eq!(path.delay(), 146.0);
}

#[test]
fn test_inv_chain_fo1_delay() {
    for n in 1..20 {
        let mut path = LogicPath::new();
        for _ in 0..n {
            path.append_sized_gate(INV_MODEL);
        }
        path.append_capacitor(INV_MODEL.cin);

        // All inverters have fanout f = 1 and gamma = 1.
        //
        // So the total delay is n * tp * (1 + f/gamma) = 2 * 3 * n.
        let analytical_delay = 6.0 * (n as f64);
        assert_eq!(
            path.delay(),
            analytical_delay,
            "incorrect delay for chain of {n} FO1 inverter(s)"
        );
    }
}

#[test]
fn test_inv_chain_fo4_delay() {
    for n in 1..8 {
        let mut path = LogicPath::new();
        for i in 0..n {
            let mul = 4f64.powi(i);
            path.append_sized_gate(mul * INV_MODEL);
        }
        path.append_capacitor(4f64.powi(n) * INV_MODEL.cin);

        // All inverters have fanout f = 4 and gamma = 1.
        //
        // So the total delay is n * tp * (1 + f/gamma) = 5 * 3 * n.
        let analytical_delay = 15.0 * (n as f64);
        assert_eq!(
            path.delay(),
            analytical_delay,
            "incorrect delay for chain of {n} FO4 inverter(s)"
        );
    }
}

#[test]
fn test_inv_chain_3_sizing() {
    let mut path = LogicPath::new();
    path.append_sized_gate(INV_MODEL);
    let a0 = 2.0;
    let b0 = 4.0;
    let cl = 64.0 * INV_MODEL.cin;
    let a = path.create_variable_with_initial(a0);
    let b = path.create_variable_with_initial(b0);
    path.append_unsized_gate(INV_MODEL, a);
    path.append_unsized_gate(INV_MODEL, b);
    path.append_capacitor(cl);

    let mut grad = path.zero_grad();
    let delay = path.delay_grad(&mut grad);
    assert_eq!(
        delay,
        3.0 * (1.0 + a0 + 1.0 + b0 / a0 + 1.0 + cl / (3.0 * b0))
    );

    assert_eq!(grad[a], 1.0 - b0 / (a0 * a0));
    assert_eq!(grad[b], 3.0 / a0 - cl / (b0 * b0));

    let opts = OptimizerOpts::default();
    path.size_with_opts(opts);
    assert!(float_eq!(path.value(a), 4.0, r2nd <= 1e-8));
    assert!(float_eq!(path.value(b), 16.0, r2nd <= 1e-8));
}

#[test]
fn test_inv_chain_4_sizing() {
    let mut path = LogicPath::new();
    path.append_sized_gate(INV_MODEL);
    let cl = 64.0 * INV_MODEL.cin;
    let a = path.create_variable();
    let b = path.create_variable();
    let c = path.create_variable();
    path.append_unsized_gate(INV_MODEL, a);
    path.append_unsized_gate(INV_MODEL, b);
    path.append_unsized_gate(INV_MODEL, c);
    path.append_capacitor(cl);

    path.size();
    assert!(
        float_eq!(path.value(a), 2.828, abs <= 0.001),
        "incorrect value: {}",
        path.value(a)
    );
    assert!(
        float_eq!(path.value(b), 8.0, abs <= 0.001),
        "incorrect value: {}",
        path.value(b)
    );
    assert!(
        float_eq!(path.value(c), 22.627, abs <= 0.001),
        "incorrect value: {}",
        path.value(c)
    );
}

/// INV -> NAND3 (size `a`, branching 4 more NAND3 loads) -> NAND2 (size `b`) -> 18 INV loads.
///
/// Delay is `18 + 25a + 4b/a + 54/b`. The stationary point is `b = 5.4^(2/3)`,
/// `a = 0.4 * sqrt(b)`, i.e. `a = 0.7018`, `b = 3.0780`.
fn inv_nand3_nand2(min_var_value: f64) -> (LogicPath, VarKey, VarKey) {
    let mut path = LogicPath::new();
    path.set_min_var_value(min_var_value);
    path.append_sized_gate(INV_MODEL);
    let cl = 18.0 * INV_MODEL.cin;
    let a = path.create_variable();
    let b = path.create_variable();
    path.append_variable_capacitor(4.0 * NAND3_MODEL.cin, a);
    path.append_unsized_gate(NAND3_MODEL, a);
    path.append_unsized_gate(NAND2_MODEL, b);
    path.append_capacitor(cl);
    path.size();
    (path, a, b)
}

#[test]
fn test_inv_nand3_nand2() {
    // The unconstrained optimum has `a < 1`, so `a` sits on the default bound
    // and `b` minimizes `4b + 54/b`, giving `b = sqrt(13.5)`.
    let (path, a, b) = inv_nand3_nand2(1.0);
    assert!(
        float_eq!(path.value(a), 1.0, abs <= 0.001),
        "incorrect value: {}",
        path.value(a)
    );
    assert!(
        float_eq!(path.value(b), 13.5f64.sqrt(), abs <= 0.001),
        "incorrect value: {}",
        path.value(b)
    );
}

#[test]
fn test_inv_nand3_nand2_interior() {
    let (path, a, b) = inv_nand3_nand2(0.1);
    let b_opt = 5.4f64.powf(2.0 / 3.0);
    let a_opt = 0.4 * b_opt.sqrt();
    assert!(
        float_eq!(path.value(a), a_opt, abs <= 0.001),
        "incorrect value: {}",
        path.value(a)
    );
    assert!(
        float_eq!(path.value(b), b_opt, abs <= 0.001),
        "incorrect value: {}",
        path.value(b)
    );
}

/// A xorshift generator, so the randomized test needs no extra dependencies.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn gate(&mut self) -> GateModel {
        GateModel {
            res: self.uniform(0.5, 5.0),
            cin: self.uniform(0.5, 5.0),
            cout: self.uniform(0.0, 5.0),
        }
    }
}

/// A random path using every kind of element and capacitance `LogicPath` supports.
fn random_path(rng: &mut Rng) -> LogicPath {
    let mut path = LogicPath::new();
    // A zero bound lets sizes reach zero, making resistances infinite.
    let min_var_value = [1.0, 0.5, 0.1, 0.0][rng.below(4)];
    path.set_min_var_value(min_var_value);
    path.append_sized_gate(rng.gate());
    let mut vars = Vec::new();
    for _ in 0..1 + rng.below(8) {
        match rng.below(6) {
            0 => path.append_resistor(rng.uniform(0.0, 2.0)),
            1 => path.append_wire(WireModel {
                res: rng.uniform(0.0, 2.0),
                cap: rng.uniform(0.0, 4.0),
            }),
            2 => path.append_sized_gate(rng.gate()),
            _ => {
                let var = if !vars.is_empty() && rng.below(4) == 0 {
                    vars[rng.below(vars.len())]
                } else {
                    let initial = if min_var_value == 0.0 && rng.below(4) == 0 {
                        0.0
                    } else {
                        rng.uniform(0.5, 4.0)
                    };
                    let var = path.create_variable_with_initial(initial);
                    vars.push(var);
                    var
                };
                // Branching loads, possibly several per segment and on other sizes.
                for _ in 0..rng.below(4) {
                    let load = if rng.below(2) == 0 {
                        var
                    } else {
                        vars[rng.below(vars.len())]
                    };
                    path.append_variable_capacitor(rng.uniform(0.0, 20.0), load);
                }
                path.append_unsized_gate(rng.gate(), var);
            }
        }
        if rng.below(2) == 0 {
            path.append_capacitor(rng.uniform(0.0, 10.0));
        }
    }
    path.append_capacitor(rng.uniform(1.0, 200.0));
    path
}

#[test]
fn size_with_opts_matches_reference_bit_for_bit() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for case in 0..300 {
        let path = random_path(&mut rng);
        let opts = OptimizerOpts {
            lr: 10f64.powf(rng.uniform(-3.0, 1.0)),
            // Fast decays converge within `max_iter`, exercising the early exit.
            lr_decay: [1.0, 0.9999, 0.999, 0.99][rng.below(4)],
            max_iter: 1 + rng.below(3_000),
        };
        let mut sized = path.clone();
        sized.size_with_opts(opts);
        let mut reference = path;
        reference.size_with_opts_reference(opts);
        let bits = |path: &LogicPath| {
            path.values()
                .into_iter()
                .map(f64::to_bits)
                .collect::<Vec<_>>()
        };
        assert_eq!(bits(&sized), bits(&reference), "case {case}: {opts:?}");
    }
}
