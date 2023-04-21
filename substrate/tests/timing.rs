use common::{out_path, setup_ctx};
use sublut::FloatLut2;
use substrate::component::{Component, NoParams};
use substrate::pdk::corner::Pvt;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::verification::simulation::waveform::EdgeDir;
use substrate::verification::timing::{ConstraintKind, SetupHoldConstraint};

mod common;

pub struct Register;

impl Component for Register {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("register")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let [clk, d] = ctx.ports(["clk", "d"], Direction::Input);
        let [vdd, vss] = ctx.ports(["vdd", "vss"], Direction::InOut);
        let q = ctx.port("q", Direction::Output);

        let stdcells = ctx.inner().std_cell_db();
        let lib = stdcells.lib_named("sky130_fd_sc_hd").unwrap();
        let cell = lib.try_cell_named("sky130_fd_sc_hd__dfxtp_2")?;

        ctx.instantiate::<StdCell>(&cell.id())?
            .with_connections([
                ("CLK", clk),
                ("D", d),
                ("VGND", vss),
                ("VNB", vss),
                ("VPB", vdd),
                ("VPWR", vdd),
                ("Q", q),
            ])
            .named("Xinner")
            .add_to(ctx);

        Ok(())
    }

    fn timing(
        &self,
        ctx: &mut substrate::verification::timing::context::TimingCtx,
    ) -> substrate::error::Result<()> {
        let d = ctx.port("d");
        let clk = ctx.port("clk");
        let rise = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.0508281, 0.1649557, 0.2434876],
                vec![-0.0181335, 0.0837871, 0.1476706],
                vec![-0.0453958, 0.0504212, 0.1106426],
            ])
            .build()
            .unwrap();
        let fall = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.1033184, 0.3187643, 0.6206849],
                vec![-0.0108092, 0.1997539, 0.4980123],
                vec![-0.0966654, 0.109015, 0.4036113],
            ])
            .build()
            .unwrap();
        let corners = ctx.inner().corner_db();
        let tt = corners.try_default_corner()?;
        let pvt = Pvt::new(tt.clone(), 1.8, 27.0);
        ctx.add_constraint(
            SetupHoldConstraint::builder()
                .pvt(pvt)
                .port(d.into_single())
                .related_port(clk.into_single())
                .related_port_transition(EdgeDir::Rising)
                .kind(ConstraintKind::Setup)
                .rise(rise)
                .fall(fall)
                .build()
                .unwrap(),
        );
        Ok(())
    }
}

#[test]
fn test_register_timing_constraints() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<Register>(
        &NoParams,
        out_path("test_register_timing_constraints", "schematic.spice"),
    )
    .expect("failed to write schematic");
}
