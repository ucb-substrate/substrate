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
        use std::fmt::Write;
        let mos_db = ctx.mos_db();
        let name = &mos_db.get_spec(params.id)?.name;
        // TODO correctly handle nf and m.
        // The sky130 pdk uses w and l in microns.
        // So we must divide by 1_000 to convert nanometers to microns.
        // The max allowed width of a single transistor is 100um, so we fold
        // larger transistors into several 100um segments plus one smaller segment containing
        // the leftover width.
        let mut spice = String::new();
        const MAX_WIDTH: i64 = 90_000;
        let n_extra = params.w / MAX_WIDTH;
        let l = params.l as f64 / 1_000.0;
        let nf = params.nf;
        let m = params.m;

        for i in 1..=n_extra {
            writeln!(
                &mut spice,
                "M{i} d g s b {name} w=90.0 l={l:.3} nf={nf} mult={m}"
            )
            .expect("failed to write to string");
        }

        let w = (params.w % MAX_WIDTH) as f64 / 1_000.0;
        writeln!(
            &mut spice,
            "M0 d g s b {name} w={w:.3} l={l:.3} nf={nf} mult={m}"
        )
        .expect("failed to write to string");

        ctx.set_spice(spice);
        Ok(())
    }
}
