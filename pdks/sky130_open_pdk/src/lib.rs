use sky130_common_pdk::Sky130Pdk;
use substrate::error::Result;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::ViaParams;
use substrate::layout::layers::Layers;
use substrate::pdk::mos::spec::MosSpec;
use substrate::pdk::{Pdk, Units};
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::netlist::{IncludeBundle, NetlistPurpose};
use substrate::units::SiPrefix;

pub mod mos;

pub struct Sky130OpenPdk {
    inner: Sky130Pdk,
}

impl Pdk for Sky130OpenPdk {
    fn new(params: &substrate::pdk::PdkParams) -> substrate::error::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self {
            inner: Sky130Pdk::new(params)?,
        })
    }

    fn name(&self) -> &'static str {
        "sky130-open"
    }

    fn process(&self) -> &'static str {
        self.inner.process()
    }

    fn lengths(&self) -> Units {
        self.inner.lengths()
    }

    fn voltages(&self) -> SiPrefix {
        self.inner.voltages()
    }

    fn layers(&self) -> Layers {
        Sky130Pdk::layers()
    }

    fn supplies(&self) -> substrate::pdk::Supplies {
        self.inner.supplies()
    }

    fn mos_devices(&self) -> Vec<MosSpec> {
        Sky130OpenPdk::mos_devices()
    }

    fn mos_schematic(
        &self,
        ctx: &mut SchematicCtx,
        params: &substrate::pdk::mos::MosParams,
    ) -> Result<()> {
        Sky130OpenPdk::mos_schematic(ctx, params)
    }

    fn mos_layout(
        &self,
        ctx: &mut LayoutCtx,
        params: &substrate::pdk::mos::LayoutMosParams,
    ) -> Result<()> {
        Sky130Pdk::mos_layout(ctx, params)
    }

    fn via_layout(&self, ctx: &mut LayoutCtx, params: &ViaParams) -> Result<()> {
        Sky130Pdk::via_layout(ctx, params)
    }

    fn layout_grid(&self) -> i64 {
        self.inner.layout_grid()
    }

    fn includes(
        &self,
        purpose: substrate::schematic::netlist::NetlistPurpose,
    ) -> Result<substrate::schematic::netlist::IncludeBundle> {
        if let NetlistPurpose::Simulation { corner } = purpose {
            Ok(IncludeBundle {
                lib_includes: vec![(
                    self.inner
                        .pdk_root
                        .join("libraries/sky130_fd_pr/latest/models/sky130.lib.spice"),
                    corner.name().clone(),
                )],
                ..Default::default()
            })
        } else {
            Ok(Default::default())
        }
    }

    #[inline]
    fn standard_cells(&self) -> Result<substrate::pdk::stdcell::StdCellDb> {
        self.inner.std_cells()
    }

    fn corners(&self) -> Result<substrate::pdk::corner::CornerDb> {
        Ok(self.inner.corners())
    }
}
