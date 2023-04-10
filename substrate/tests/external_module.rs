use std::path::PathBuf;

use arcstr::ArcStr;
use substrate::component::NoParams;
use substrate::data::SubstrateCtx;
use substrate::schematic::circuit::Direction;
use substrate::schematic::module::{ExternalModule, RawSource};

mod common;
use common::common_source::CommonSourceAmp;
use common::{out_path, setup_ctx, DATA_DIR};

const SPICE_RESISTOR: ArcStr = arcstr::literal!(
    r#"
.subckt my_resistor p n
R1 p n 100
.ends
"#
);

fn add_external_modules(ctx: &SubstrateCtx) {
    let my_resistor = ExternalModule::from_spice_literal("my_resistor", SPICE_RESISTOR).unwrap();

    let my_transistor = ExternalModule::builder()
        .name("my_transistor")
        .add_port("d", 1, Direction::InOut)
        .add_port("g", 1, Direction::Input)
        .add_port("s", 1, Direction::InOut)
        .add_port("b", 1, Direction::Output)
        .source(RawSource::with_file(
            PathBuf::from(DATA_DIR).join("schematics/my_transistor.spice"),
        ))
        .build();

    ctx.add_external_module(my_resistor).unwrap();
    ctx.add_external_module(my_transistor).unwrap();
}

#[test]
fn test_external_modules() {
    let ctx = setup_ctx();
    add_external_modules(&ctx);

    ctx.write_schematic_to_file::<CommonSourceAmp>(
        &NoParams,
        out_path("test_external_modules", "schematic.spice"),
    )
    .expect("failed to write schematic");
}
