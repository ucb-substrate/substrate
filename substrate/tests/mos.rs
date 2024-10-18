use common::{out_path, setup_ctx};
use substrate::layout::elements::mos::LayoutMos;
use substrate::pdk::mos::spec::MosId;
use substrate::pdk::mos::{LayoutMosParams, MosParams};

mod common;

#[test]
fn test_sky130_mos_nand2() {
    let ctx = setup_ctx();
    ctx.write_layout::<LayoutMos>(
        &LayoutMosParams {
            skip_sd_metal: vec![vec![]; 2],
            deep_nwell: false,
            contact_strategy: substrate::pdk::mos::GateContactStrategy::BothSides,
            devices: vec![
                MosParams {
                    w: 2000,
                    l: 150,
                    m: 1,
                    nf: 2,
                    id: MosId::new(0),
                },
                MosParams {
                    w: 4000,
                    l: 150,
                    m: 1,
                    nf: 2,
                    id: MosId::new(1),
                },
            ],
        },
        out_path("test_sky130_mos_nand2", "layout.gds"),
    )
    .expect("failed to write layout");
}
