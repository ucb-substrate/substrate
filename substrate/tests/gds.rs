use std::path::PathBuf;

use arcstr::ArcStr;
use codegen::hard_macro;
use subgeom::Shape;
use substrate::component::{Component, NoParams, View};
use substrate::data::SubstrateCtx;
use substrate::layout::layers::{GdsLayerSpec, LayerPurpose};

mod common;
use common::vdivider::array::VDividerArray;
use common::{gds_path, out_path, setup_ctx};

#[hard_macro(
    name = "test_simple_macro",
    gds_cell_name = "B",
    pdk = "sky130-open",
    path_fn = "path"
)]
pub struct TestSimpleMacro;

fn path(_ctx: &SubstrateCtx, _name: &str, view: View) -> Option<PathBuf> {
    match view {
        View::Layout => Some(gds_path("test_sky130_simple")),
        _ => None,
    }
}

#[test]
fn test_gds_import() {
    let ctx = setup_ctx();
    let cell_map = ctx
        .from_gds(gds_path("test_sky130_simple"))
        .expect("GDS library should be imported successfully");

    let a = cell_map.get("A").unwrap();
    let b = cell_map.get("B").unwrap();
    let a_elems = a.elems().collect::<Vec<_>>();
    let b_insts = b.insts().collect::<Vec<_>>();
    let b_elems = b.elems().collect::<Vec<_>>();
    let b_annotations = b.annotations().collect::<Vec<_>>();
    let mut b_ports = b.ports();

    assert_eq!(a_elems.len(), 1, "expected 1 element in cell A");
    let a_elem_0 = a_elems[0];
    assert!(
        matches!(a_elem_0.inner, Shape::Rect(_)),
        "expected cell A to have a rectangle"
    );
    assert_eq!(
        a_elem_0.layer.purpose(),
        &LayerPurpose::Drawing,
        "expected rectangle in cell A to have purpose Drawing"
    );
    assert_eq!(
        ctx.raw_layers()
            .read()
            .unwrap()
            .get(a_elem_0.layer.layer())
            .unwrap()
            .info
            .name,
        ArcStr::from("met1"),
        "expected rectangle in cell A to be on metal 1"
    );

    assert_eq!(b_insts.len(), 4, "expected 4 instances in cell B");
    for inst in b_insts {
        assert_eq!(
            inst.cell().id(),
            a.id(),
            "expected all instances to be instances of cell A"
        );
    }

    // The pin rectangle should be imported as a port, not as an element.
    assert_eq!(b_elems.len(), 1, "expected 1 element in cell B");
    assert_eq!(b_annotations.len(), 0, "expected 0 annotations in cell B");
    let b_port_0 = b_ports.next().unwrap();
    assert_eq!(b_port_0.name(), "gnd", "expected a GND port in cell B");
    assert!(b_ports.next().is_none(), "expected only 1 port in cell B");
}

#[test]
fn test_gds_import_nonexistent_layer() {
    let ctx = setup_ctx();
    let cell_map = ctx
        .from_gds(gds_path("test_sky130_nonexistent_layer"))
        .expect("GDS library should be imported successfully");

    let layers_arc = ctx.raw_layers();
    let layers = layers_arc.read().unwrap();
    let new_layer = layers.get_from_spec(GdsLayerSpec(0, 0)).unwrap();

    let a = cell_map.get("A").unwrap();
    let a_elems = a.elems().collect::<Vec<_>>();
    assert_eq!(a_elems.len(), 1, "expected 1 element in cell A");
    let a_elem_0 = a_elems[0];
    assert_eq!(
        &a_elem_0.layer, new_layer,
        "expected element to be on GDS layer (0, 0)"
    );
}

#[test]
fn test_gds_import_invalid_units() {
    let ctx = setup_ctx();
    ctx.from_gds(gds_path("test_sky130_invalid_units"))
        .expect_err("should fail due to unit mismatch with PDK");
}

#[test]
fn test_gds_export() {
    let gds_path = out_path("test_gds_export", "layout.gds");
    let ctx_original = setup_ctx();
    ctx_original
        .write_layout::<VDividerArray>(&NoParams, &gds_path)
        .expect("failed to write layout");

    let name = VDividerArray::new(&NoParams, &ctx_original)
        .expect("failed to create VDividerArray struct")
        .name();

    let ctx_new = setup_ctx();
    let cell_map = ctx_new
        .from_gds(gds_path)
        .expect("failed to import GDS file");
    let array = cell_map.get(&name).unwrap();
    assert_eq!(array.insts().count(), 10);
}

#[test]
fn test_gds_reexport() {
    let gds_path = out_path("test_gds_reexport", "layout.gds");
    let ctx = setup_ctx();

    // Imports a hard macro from a GDS file.
    ctx.write_layout::<TestSimpleMacro>(&NoParams, &gds_path)
        .expect("failed to write layout");
    println!("finished writing layout");

    let ctx_new = setup_ctx();
    let cell_map = ctx_new
        .from_gds(gds_path)
        .expect("failed to import GDS file");
    let a = cell_map.get("A").unwrap();
    let b = cell_map.get("test_simple_macro").unwrap();
    let a_elems = a.elems().collect::<Vec<_>>();
    let b_insts = b.insts().collect::<Vec<_>>();
    let b_elems = b.elems().collect::<Vec<_>>();
    let b_annotations = b.annotations().collect::<Vec<_>>();
    let mut b_ports = b.ports();

    assert_eq!(a_elems.len(), 1, "expected 1 element in cell A");
    let a_elem_0 = a_elems[0];
    assert!(
        matches!(a_elem_0.inner, Shape::Rect(_)),
        "expected cell A to have a rectangle"
    );
    assert_eq!(
        a_elem_0.layer.purpose(),
        &LayerPurpose::Drawing,
        "expected rectangle in cell A to have purpose Drawing"
    );
    assert_eq!(
        ctx_new
            .raw_layers()
            .read()
            .unwrap()
            .get(a_elem_0.layer.layer())
            .unwrap()
            .info
            .name,
        ArcStr::from("met1"),
        "expected rectangle in cell A to be on metal 1"
    );

    assert_eq!(b_insts.len(), 4, "expected 4 instances in cell B");
    for inst in b_insts {
        assert_eq!(
            inst.cell().id(),
            a.id(),
            "expected all instances to be instances of cell A"
        );
    }

    // One element is the large drawn rectangle.
    // The other element is a small drawing rectangle that gets generated under the pin rectangle.
    assert_eq!(b_elems.len(), 2, "expected 2 element in cell B");
    assert_eq!(b_annotations.len(), 0, "expected 0 annotations in cell B");
    let b_port_0 = b_ports.next().unwrap();
    assert_eq!(b_port_0.name(), "gnd", "expected a GND port in cell B");
    assert!(b_ports.next().is_none(), "expected only 1 port in cell B");
}
