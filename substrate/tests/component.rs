use substrate::component::NoParams;

mod common;
use common::common_source::CommonSourceAmp;
use common::vdivider::array::VDividerArray;
use common::vdivider::tb::VDividerTb;
use common::{out_path, setup_ctx};

#[test]
#[ignore = "slow"]
fn test_vdivider() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<VDividerArray>(
        &NoParams,
        out_path("test_vdivider", "schematic.spice"),
    )
    .expect("failed to write schematic");

    ctx.write_layout::<VDividerArray>(&NoParams, out_path("test_vdivider", "layout.gds"))
        .expect("failed to write layout");

    let output = ctx
        .write_simulation::<VDividerTb>(&NoParams, out_path("test_vdivider", "sim"))
        .unwrap();
    // Check that the voltage divider ratio is 1/3
    assert_eq!(output.ratio, 1.0 / 3.0);
}

#[test]
fn test_common_source() {
    let ctx = setup_ctx();

    ctx.write_schematic_to_file::<CommonSourceAmp>(
        &NoParams,
        out_path("test_common_source", "schematic.spice"),
    )
    .expect("failed to write schematic");
}
