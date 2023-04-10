use itertools::Itertools;
use substrate::component::NoParams;

mod common;
use common::vdivider::array::VDividerArrayWrapper;
use common::{out_path, setup_ctx};
use substrate::schematic::circuit::Direction;

#[test]
fn test_bubble_layout_ports() {
    let ctx = setup_ctx();
    let inst = ctx
        .instantiate_layout::<VDividerArrayWrapper>(&NoParams)
        .expect("failed to generate layout");

    assert_eq!(inst.ports().count(), 0);

    ctx.write_layout::<VDividerArrayWrapper>(
        &NoParams,
        out_path("test_bubble_layout_ports", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_bubble_schematic_ports() {
    let ctx = setup_ctx();
    let inst = ctx
        .instantiate_schematic::<VDividerArrayWrapper>(&NoParams)
        .expect("failed to generate schematic");

    let ports = inst.ports().unwrap().collect_vec();
    assert_eq!(ports.len(), 3);

    assert_eq!(ports[0].name(), "vss");
    assert_eq!(ports[0].width(), 1);
    assert_eq!(ports[0].direction(), Direction::InOut);

    assert_eq!(ports[1].name(), "vdd");
    assert_eq!(ports[1].width(), 1);
    assert_eq!(ports[1].direction(), Direction::InOut);

    assert_eq!(ports[2].name(), "out");
    assert_eq!(ports[2].width(), 10);
    assert_eq!(ports[2].direction(), Direction::Output);

    ctx.write_schematic_to_file::<VDividerArrayWrapper>(
        &NoParams,
        out_path("test_bubble_schematic_ports", "schematic.spice"),
    )
    .expect("failed to write schematic");
}
