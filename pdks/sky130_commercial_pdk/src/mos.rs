use substrate::error::Result;
use substrate::pdk::mos::spec::{MosFlavor, MosId, MosKind, MosSpec};
use substrate::pdk::mos::MosParams;
use substrate::pdk::SupplyId;
use substrate::schematic::context::SchematicCtx;

use crate::Sky130CommercialPdk;

impl Sky130CommercialPdk {
    pub(crate) fn mos_devices() -> Vec<MosSpec> {
        let nmos = MosSpec {
            id: MosId::new(0),
            name: "nshort".to_string(),
            lmin: 150,
            wmin: 420,
            kind: MosKind::Nmos,
            flavor: MosFlavor::Svt,
            supply: SupplyId::Core,
            ..Default::default()
        };

        let pmos = MosSpec {
            id: MosId::new(1),
            name: "pshort".to_string(),
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
            "M0 d g s b {} w={:.3} l={:.3} nf={} mult={}",
            name,
            params.w as f64 / 1_000.0,
            params.l as f64 / 1_000.0,
            params.nf,
            params.m,
        ));
        Ok(())
    }
}
