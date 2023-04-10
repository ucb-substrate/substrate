use common::{out_path, setup_ctx};
use substrate::pdk::stdcell::StdCell;

mod common;

#[test]
fn test_sky130_standard_cells() {
    let ctx = setup_ctx();
    let stdcells = ctx.std_cell_db();
    let lib = stdcells.lib_named("sky130_fd_sc_hd").unwrap();

    for cell in lib.cells() {
        ctx.write_schematic_to_file::<StdCell>(
            &cell.id(),
            out_path(
                "test_sky130_standard_cells",
                &format!("{}.spice", cell.name()),
            ),
        )
        .expect("failed to write schematic");
        ctx.write_layout::<StdCell>(
            &cell.id(),
            out_path(
                "test_sky130_standard_cells",
                &format!("{}.gds", cell.name()),
            ),
        )
        .expect("failed to write layout");
    }
}
