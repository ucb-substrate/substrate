use substrate::error::Result;
use substrate::pdk::mos::spec::{MosFlavor, MosId, MosKind, MosSpec};
use substrate::pdk::mos::MosParams;
use substrate::pdk::SupplyId;
use substrate::schematic::context::SchematicCtx;

use crate::Sky130OpenPdk;

impl Sky130OpenPdk {
    pub(crate) fn mos_devices() -> Vec<MosSpec> {
        let nmos = MosSpec {
            id: MosId::new(0),
            name: "sky130_fd_pr__nfet_01v8".to_string(),
            lmin: 150,
            wmin: 420,
            kind: MosKind::Nmos,
            flavor: MosFlavor::Svt,
            supply: SupplyId::Core,
            ..Default::default()
        };

        let pmos = MosSpec {
            id: MosId::new(1),
            name: "sky130_fd_pr__pfet_01v8".to_string(),
            lmin: 150,
            wmin: 420,
            kind: MosKind::Pmos,
            flavor: MosFlavor::Svt,
            supply: SupplyId::Core,
            ..Default::default()
        };

        vec![nmos, pmos]
    }

    pub(crate) fn mos_schematic(ctx: &mut SchematicCtx, params: &MosParams) -> Result<()> {
        let mos_db = ctx.mos_db();
        let name = &mos_db.get_spec(params.id)?.name;
        // TODO correctly handle nf and m.
        // The sky130 pdk uses w and l in microns.
        // So we must divide by 1_000 to convert nanometers to microns.
        ctx.set_spice(format!(
            "X0 d g s b {} w={:.3} l={:.3}",
            name,
            (params.w as u64 * params.nf * params.m) as f64 / 1_000.0,
            params.l as f64 / 1_000.0
        ));
        Ok(())
    }
}
