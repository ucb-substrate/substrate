mod common;
use common::{out_path, setup_ctx};
use subgates::*;

#[test]
fn test_and2() {
    let ctx = setup_ctx();
    let test_name = "test_and2";

    let params = AndParams {
        nand: PrimitiveGateParams {
            pwidth: 2_400,
            nwidth: 1_800,
            length: 150,
        },
        inv: PrimitiveGateParams {
            pwidth: 2_400,
            nwidth: 1_800,
            length: 150,
        },
    };
    ctx.write_layout::<And2>(&params, out_path(test_name, "layout.gds"))
        .expect("failed to write layout");
    ctx.write_schematic_to_file::<And2>(&params, out_path(test_name, "netlist.spice"))
        .expect("failed to write schematic");
}

#[test]
fn test_and3() {
    let ctx = setup_ctx();
    let test_name = "test_and3";

    let params = AndParams {
        nand: PrimitiveGateParams {
            pwidth: 2_400,
            nwidth: 4_000,
            length: 150,
        },
        inv: PrimitiveGateParams {
            pwidth: 2_400,
            nwidth: 1_800,
            length: 150,
        },
    };
    ctx.write_layout::<And3>(&params, out_path(test_name, "layout.gds"))
        .expect("failed to write layout");
    ctx.write_schematic_to_file::<And3>(&params, out_path(test_name, "netlist.spice"))
        .expect("failed to write schematic");
}

#[test]
fn test_inv_dec() {
    let ctx = setup_ctx();
    let test_name = "test_inv_dec";

    let params = Inv::dec_params();
    ctx.write_layout::<Inv>(&params, out_path(test_name, "layout.gds"))
        .expect("failed to write layout");
    ctx.write_schematic_to_file::<Inv>(&params, out_path(test_name, "netlist.spice"))
        .expect("failed to write schematic");
}

#[test]
fn test_nand2_dec() {
    let ctx = setup_ctx();
    let test_name = "test_nand2_dec";

    let params = Nand2::dec_params();
    ctx.write_layout::<Nand2>(&params, out_path(test_name, "layout.gds"))
        .expect("failed to write layout");
    ctx.write_schematic_to_file::<Nand2>(&params, out_path(test_name, "netlist.spice"))
        .expect("failed to write schematic");
}

#[test]
fn test_nand3() {
    let ctx = setup_ctx();
    let test_name = "test_nand3";

    let params = PrimitiveGateParams {
        nwidth: 1_600,
        pwidth: 2_400,
        length: 150,
    };
    ctx.write_layout::<Nand3>(&params, out_path(test_name, "layout.gds"))
        .expect("failed to write layout");
    ctx.write_schematic_to_file::<Nand3>(&params, out_path(test_name, "netlist.spice"))
        .expect("failed to write schematic");
}

#[test]
fn test_nor2() {
    let ctx = setup_ctx();
    let test_name = "test_nor2";

    let params = PrimitiveGateParams {
        nwidth: 1_200,
        pwidth: 3_000,
        length: 150,
    };
    ctx.write_layout::<Nor2>(&params, out_path(test_name, "layout.gds"))
        .expect("failed to write layout");
    ctx.write_schematic_to_file::<Nor2>(&params, out_path(test_name, "netlist.spice"))
        .expect("failed to write schematic");
}
