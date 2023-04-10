//! An empty PDK.
//!
//! Has no layers, device information, via layouts, etc.
//! Intended for use when you need to do PDK-independent operations,
//! such as listing the cells in a GDS file.
use std::collections::HashMap;

use substrate::error::Result;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::ViaParams;
use substrate::layout::layers::Layers;
use substrate::pdk::mos::spec::MosSpec;
use substrate::pdk::{Pdk, Supplies, Supply, SupplyId, Units};
use substrate::schematic::context::SchematicCtx;
use substrate::units::SiPrefix;

pub struct EmptyPdk {}

impl Pdk for EmptyPdk {
    fn new(_params: &substrate::pdk::PdkParams) -> substrate::error::Result<Self> {
        Ok(Self {})
    }

    fn name(&self) -> &'static str {
        "empty"
    }

    fn process(&self) -> &'static str {
        "empty"
    }

    fn lengths(&self) -> Units {
        Units::new(SiPrefix::Nano, SiPrefix::Nano)
    }

    fn voltages(&self) -> SiPrefix {
        SiPrefix::None
    }

    fn layers(&self) -> Layers {
        Layers::new()
    }

    fn supplies(&self) -> substrate::pdk::Supplies {
        let values = HashMap::from_iter([(
            SupplyId::Core,
            Supply {
                typ: 1.0f64,
                ..Default::default()
            },
        )]);
        Supplies { values }
    }

    fn mos_devices(&self) -> Vec<MosSpec> {
        vec![]
    }

    fn mos_schematic(
        &self,
        _ctx: &mut SchematicCtx,
        _params: &substrate::pdk::mos::MosParams,
    ) -> Result<()> {
        Ok(())
    }

    fn mos_layout(
        &self,
        _ctx: &mut LayoutCtx,
        _params: &substrate::pdk::mos::LayoutMosParams,
    ) -> Result<()> {
        Ok(())
    }

    fn via_layout(&self, _ctx: &mut LayoutCtx, _params: &ViaParams) -> Result<()> {
        Ok(())
    }

    fn layout_grid(&self) -> i64 {
        1
    }

    fn pre_sim(
        &self,
        _ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> Result<()> {
        Ok(())
    }
}
