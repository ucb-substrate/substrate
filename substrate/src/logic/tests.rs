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

#[test]
fn test_inv_nand3_nand2() {
    let mut path = LogicPath::new();
    path.append_sized_gate(INV_MODEL);
    let cl = 18.0 * INV_MODEL.cin;
    let a = path.create_variable();
    let b = path.create_variable();
    path.append_variable_capacitor(4.0 * NAND3_MODEL.cin, a);
    path.append_unsized_gate(NAND3_MODEL, a);
    path.append_unsized_gate(NAND2_MODEL, b);
    path.append_capacitor(cl);

    path.size();
    assert!(
        float_eq!(path.value(a), 2.052, abs <= 0.001),
        "incorrect value: {}",
        path.value(a)
    );
    assert!(
        float_eq!(path.value(b), 5.263, abs <= 0.001),
        "incorrect value: {}",
        path.value(b)
    );
}
