use std::path::PathBuf;

use arcstr::ArcStr;
use sky130_common_pdk::Sky130Pdk;
use substrate::error::Result;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::ViaParams;
use substrate::layout::layers::Layers;
use substrate::pdk::mos::spec::MosSpec;
use substrate::pdk::{Pdk, PdkParams, Units};
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::netlist::{IncludeBundle, NetlistPurpose};
use substrate::units::SiPrefix;

pub mod mos;

pub struct Sky130CommercialPdk {
    inner: Sky130Pdk,
    commercial_root: PathBuf,
}

impl Sky130CommercialPdk {
    pub fn new(commercial_root: PathBuf, open_root: PathBuf) -> substrate::error::Result<Self> {
        Ok(Self {
            commercial_root,
            inner: Sky130Pdk::new(&PdkParams {
                pdk_root: open_root,
            })?,
        })
    }
}

impl Pdk for Sky130CommercialPdk {
    fn name(&self) -> &'static str {
        "sky130-commercial"
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
        Sky130CommercialPdk::mos_devices()
    }

    fn mos_schematic(
        &self,
        ctx: &mut SchematicCtx,
        params: &substrate::pdk::mos::MosParams,
    ) -> Result<()> {
        Sky130CommercialPdk::mos_schematic(ctx, params)
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
        let (raw_spice, includes) = match purpose {
            NetlistPurpose::Lvs | NetlistPurpose::Pex | NetlistPurpose::Timing => {
                (CAL_PRELUDE, vec![])
            }
            NetlistPurpose::Simulation { corner } => (
                SIM_PRELUDE,
                vec![
                    "MODELS/SPECTRE/s8x/Models/models.all".to_string(),
                    format!("MODELS/SPECTRE/s8x/Models/{}.cor", corner.name()),
                    format!("MODELS/SPECTRE/s8x/Models/{}cell.cor", corner.name()),
                    "MODELS/SPECTRE/s8x/Models/npass.pm3".to_string(),
                    "MODELS/SPECTRE/s8x/Models/npd.pm3".to_string(),
                    "MODELS/SPECTRE/s8x/Models/ppu.pm3".to_string(),
                ],
            ),
            NetlistPurpose::Library => (EMPTY, vec![]),
        };

        let includes = includes
            .iter()
            .map(|p| self.commercial_root.join(p))
            .collect();

        Ok(IncludeBundle {
            raw_spice,
            includes,
            ..Default::default()
        })
    }

    fn standard_cells(&self) -> Result<substrate::pdk::stdcell::StdCellDb> {
        self.inner.std_cells()
    }

    fn corners(&self) -> Result<substrate::pdk::corner::CornerDb> {
        Ok(self.inner.corners())
    }
}

const EMPTY: ArcStr = arcstr::literal!("");

/// Simulation prelude.
const SIM_PRELUDE: ArcStr = arcstr::literal!(
    "*SPICE NETLIST
* OPEN SOURCE CONVERSION PRELUDE (SPECTRE)

.SUBCKT sky130_fd_pr__special_nfet_pass d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b npass l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_nfet_latch d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b npd l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__nfet_01v8 d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b nshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8 d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b pshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_pfet_pass d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b ppu l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8_hvt d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b phighvt l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__nfet_01v8_lvt d g s b
.PARAM w=1.0 l=1.0 mult=1
M0 d g s b nlowvt l='l' w='w' mult='mult'
.ENDS
"
);

/// Calibre prelude.
const CAL_PRELUDE: ArcStr = arcstr::literal!(
    "*SPICE NETLIST
* OPEN SOURCE CONVERSION PRELUDE (SPICE)

.SUBCKT sky130_fd_pr__special_nfet_pass d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b npass l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_nfet_latch d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b npd l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__nfet_01v8 d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b nshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8 d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b pshort l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__special_pfet_pass d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b ppu l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__pfet_01v8_hvt d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b phighvt l='l' w='w' mult='mult'
.ENDS

.SUBCKT sky130_fd_pr__nfet_01v8_lvt d g s b PARAMS: w=1.0 l=1.0 mult=1
M0 d g s b nlowvt l='l' w='w' mult='mult'
.ENDS
"
);
