use substrate::component::{Component, NoParams};

mod common;
use common::sp_cell::{
    SpCell, SpCellArray, SpCellArrayCenter, SpCellArrayCornerTop, SpCellArrayLeft,
    SpCellArrayParams, SpCellArrayTop,
};
use common::{hm_toml_path, out_path, setup_ctx};
use substrate::hard_macro::Config;

pub struct ManualSchematicImport;

impl Component for ManualSchematicImport {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("manual_schematic_import")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let cfg = Config::from_toml_file(hm_toml_path("example_01"))?;
        ctx.import_hard_macro_config(cfg)?;
        Ok(())
    }
}

#[test]
fn test_sp_cell() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<SpCell>(&NoParams, out_path("test_sp_cell", "schematic.spice"))
        .expect("failed to write schematic");
    ctx.write_layout::<SpCell>(&NoParams, out_path("test_sp_cell", "layout.gds"))
        .expect("failed to write layout");
}

#[test]
fn test_sp_cell_array_corner_top() {
    let ctx = setup_ctx();
    ctx.write_layout::<SpCellArrayCornerTop>(
        &NoParams,
        out_path("test_sp_cell_array_corner_top", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_sp_cell_array_left() {
    let ctx = setup_ctx();
    ctx.write_layout::<SpCellArrayLeft>(
        &NoParams,
        out_path("test_sp_cell_array_left", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_sp_cell_array_top() {
    let ctx = setup_ctx();
    ctx.write_layout::<SpCellArrayTop>(&NoParams, out_path("test_sp_cell_array_top", "layout.gds"))
        .expect("failed to write layout");
}

#[test]
fn test_sp_cell_array_center() {
    let ctx = setup_ctx();
    ctx.write_layout::<SpCellArrayCenter>(
        &NoParams,
        out_path("test_sp_cell_array_center", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_sp_cell_array() {
    let ctx = setup_ctx();
    ctx.write_layout::<SpCellArray>(
        &SpCellArrayParams { rows: 32, cols: 32 },
        out_path("test_sp_cell_array", "layout.gds"),
    )
    .expect("failed to write layout");
}

#[test]
fn test_manual_schematic_import_from_toml() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<ManualSchematicImport>(
        &NoParams,
        out_path("test_manual_schematic_import_from_toml", "schematic.spice"),
    )
    .expect("failed to write schematic");
}
