use std::sync::Arc;

use common::{out_path, setup_ctx};
use serde::{Deserialize, Serialize};
use sublut::FloatLut2;
use substrate::component::{Component, NoParams};
use substrate::data::VerifyTiming;
use substrate::error::ErrorSource;
use substrate::pdk::corner::Pvt;
use substrate::pdk::stdcell::StdCell;
use substrate::schematic::circuit::Direction;
use substrate::schematic::elements::vdc::Vdc;
use substrate::schematic::elements::vpwl::Vpwl;
use substrate::units::{SiPrefix, SiValue};
use substrate::verification::simulation::testbench::Testbench;
use substrate::verification::simulation::waveform::{EdgeDir, Waveform};
use substrate::verification::simulation::TranAnalysis;
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
        arcstr::literal!("register_wrapper")
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
        let corners = ctx.inner().corner_db();
        let tt = corners.try_default_corner()?;
        let pvt = Pvt::new(tt.clone(), 1.8, 25.0);

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
        ctx.add_constraint(
            SetupHoldConstraint::builder()
                .pvt(pvt.clone())
                .port(d.into_single())
                .related_port(clk.into_single())
                .related_port_transition(EdgeDir::Rising)
                .kind(ConstraintKind::Setup)
                .rise(rise)
                .fall(fall)
                .build()
                .unwrap(),
        );

        let rise = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![-0.0285176, -0.1316588, -0.1857767],
                vec![0.0355612, -0.0602559, -0.1143737],
                vec![0.0579408, -0.0354349, -0.0895527],
            ])
            .build()
            .unwrap();
        let fall = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![-0.0468281, -0.2512878, -0.5178079],
                vec![0.0636374, -0.1408223, -0.4146667],
                vec![0.1409486, -0.059849, -0.3349141],
            ])
            .build()
            .unwrap();
        ctx.add_constraint(
            SetupHoldConstraint::builder()
                .pvt(pvt)
                .port(d.into_single())
                .related_port(clk.into_single())
                .related_port_transition(EdgeDir::Rising)
                .kind(ConstraintKind::Hold)
                .rise(rise)
                .fall(fall)
                .build()
                .unwrap(),
        );

        let ss = corners.try_corner_named("ss")?;
        let pvt = Pvt::new(ss.clone(), 1.6, 100.0);

        let rise = FloatLut2::builder()
            .k1(vec![0.01, 2.5, 5.0])
            .k2(vec![0.01, 2.5, 5.0])
            .values(vec![
                vec![0.135667, 0.7907695, 1.1372458],
                vec![-0.2942158, 0.2821514, 0.5773581],
                vec![-0.4997008, 0.063849, 0.3499004],
            ])
            .build()
            .unwrap();
        let fall = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.2656719, 1.3016337, 1.9722067],
                vec![-0.4077412, 0.5897686, 1.2566794],
                vec![-0.7542175, 0.2176575, 0.8809062],
            ])
            .build()
            .unwrap();
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

// Register with fake timing data to test positive hold time constraints.
pub struct FakeRegister;

impl Component for FakeRegister {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("fake_register")
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
        let corners = ctx.inner().corner_db();
        let tt = corners.try_default_corner()?;
        let pvt = Pvt::new(tt.clone(), 1.8, 25.0);

        let rise = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.05, 0.1, 0.15],
                vec![0.1, 0.15, 0.2],
                vec![0.15, 0.2, 0.25],
            ])
            .build()
            .unwrap();
        let fall = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.05, 0.1, 0.15],
                vec![0.1, 0.15, 0.2],
                vec![0.15, 0.2, 0.25],
            ])
            .build()
            .unwrap();
        ctx.add_constraint(
            SetupHoldConstraint::builder()
                .pvt(pvt.clone())
                .port(d.into_single())
                .related_port(clk.into_single())
                .related_port_transition(EdgeDir::Rising)
                .kind(ConstraintKind::Setup)
                .rise(rise)
                .fall(fall)
                .build()
                .unwrap(),
        );

        let rise = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.02, 0.04, 0.06],
                vec![0.04, 0.06, 0.08],
                vec![0.06, 0.08, 0.1],
            ])
            .build()
            .unwrap();
        let fall = FloatLut2::builder()
            .k1(vec![0.01, 0.5, 1.5])
            .k2(vec![0.01, 0.5, 1.5])
            .values(vec![
                vec![0.02, 0.04, 0.06],
                vec![0.04, 0.06, 0.08],
                vec![0.06, 0.08, 0.1],
            ])
            .build()
            .unwrap();
        ctx.add_constraint(
            SetupHoldConstraint::builder()
                .pvt(pvt)
                .port(d.into_single())
                .related_port(clk.into_single())
                .related_port_transition(EdgeDir::Rising)
                .kind(ConstraintKind::Hold)
                .rise(rise)
                .fall(fall)
                .build()
                .unwrap(),
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegisterType {
    Real,
    Fake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegTb {
    td: f64,
    vdd: f64,
    tr: f64,
    tf: f64,
    rtype: RegisterType,
}

impl Component for RegTb {
    type Params = Self;

    fn new(
        params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(params.clone())
    }

    fn name(&self) -> arcstr::ArcStr {
        arcstr::literal!("reg_tb")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let vss = ctx.port("vss", Direction::InOut);
        let [vdd, clk, d, q] = ctx.signals(["vdd", "clk", "d", "q"]);

        match self.rtype {
            RegisterType::Real => {
                ctx.instantiate::<Register>(&NoParams)?
                    .with_connections([
                        ("clk", clk),
                        ("d", d),
                        ("vdd", vdd),
                        ("vss", vss),
                        ("q", q),
                    ])
                    .named("dut")
                    .add_to(ctx);
            }
            RegisterType::Fake => {
                ctx.instantiate::<FakeRegister>(&NoParams)?
                    .with_connections([
                        ("clk", clk),
                        ("d", d),
                        ("vdd", vdd),
                        ("vss", vss),
                        ("q", q),
                    ])
                    .named("dut")
                    .add_to(ctx);
            }
        }

        let vmax = SiValue::with_precision(1.8, SiPrefix::Nano);
        ctx.instantiate::<Vdc>(&vmax)?
            .with_connections([("p", vdd), ("n", vss)])
            .named("vdd")
            .add_to(ctx);

        let (clkw, dw) = self.waveforms();
        ctx.instantiate::<Vpwl>(&clkw)?
            .with_connections([("p", clk), ("n", vss)])
            .named("vclk")
            .add_to(ctx);
        ctx.instantiate::<Vpwl>(&dw)?
            .with_connections([("p", d), ("n", vss)])
            .named("vin")
            .add_to(ctx);

        Ok(())
    }
}

impl Testbench for RegTb {
    type Output = ();

    fn setup(
        &mut self,
        ctx: &mut substrate::verification::simulation::context::PreSimCtx,
    ) -> substrate::error::Result<()> {
        let an = TranAnalysis::builder()
            .start(0.0)
            .stop(self.td + 2.0 * self.tr + 100e-12)
            .step(self.tr / 10.0)
            .build()
            .unwrap();
        ctx.add_analysis(an);
        Ok(())
    }

    fn measure(
        &mut self,
        _ctx: &substrate::verification::simulation::context::PostSimCtx,
    ) -> substrate::error::Result<Self::Output> {
        Ok(())
    }
}

impl RegTb {
    fn waveforms(&self) -> (Arc<Waveform>, Arc<Waveform>) {
        let vdd = self.vdd;
        let ts = 100e-12;

        let mut clk = Waveform::with_initial_value(0.0);
        clk.push_low(ts + self.td, vdd, self.tf);
        clk.push_high(1.0, vdd, self.tf);

        let mut d = Waveform::with_initial_value(0.0);
        d.push_low(ts, vdd, self.tf);
        d.push_high(1.0, vdd, self.tf);

        (Arc::new(clk), Arc::new(d))
    }
}

#[test]
#[ignore = "long"]
fn test_setup_time_check() {
    let ctx = setup_ctx();
    let name = "test_setup_time_check";
    ctx.write_schematic_to_file::<Register>(&NoParams, out_path(name, "schematic.spice"))
        .expect("failed to write schematic");

    let corners = ctx.corner_db();
    let tt = corners.try_corner_named("tt").expect("no tt corner");
    let pvt = Pvt::new(tt.clone(), 1.8, 25.0);

    let valid_setup_tb = RegTb {
        td: 2e-9,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Fake,
    };
    let work_dir = out_path(name, "sim_valid_setup");
    ctx._write_simulation::<RegTb>(
        &valid_setup_tb,
        work_dir,
        None,
        VerifyTiming::Yes(pvt.clone()),
    )
    .expect("failed to run simulation");

    let invalid_setup_tb = RegTb {
        td: 50e-12,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Fake,
    };
    let work_dir = out_path(name, "sim_invalid_setup");
    let err = ctx
        ._write_simulation::<RegTb>(&invalid_setup_tb, work_dir, None, VerifyTiming::Yes(pvt))
        .expect_err("expected setup timing checks to fail and return error");
    assert!(matches!(err.source(), ErrorSource::TimingFailed(_)));
}

#[test]
#[ignore = "long"]
fn test_hold_time_check() {
    let ctx = setup_ctx();
    let name = "test_hold_time_check";
    ctx.write_schematic_to_file::<Register>(&NoParams, out_path(name, "schematic.spice"))
        .expect("failed to write schematic");

    let corners = ctx.corner_db();
    let tt = corners.try_corner_named("tt").expect("no tt corner");
    let pvt = Pvt::new(tt.clone(), 1.8, 25.0);

    let invalid_hold_tb = RegTb {
        td: -10e-12,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Fake,
    };
    let work_dir = out_path(name, "sim_invalid_hold");
    let err = ctx
        ._write_simulation::<RegTb>(
            &invalid_hold_tb,
            work_dir,
            None,
            VerifyTiming::Yes(pvt.clone()),
        )
        .expect_err("expected timing constraints to fail and return error");
    assert!(matches!(err.source(), ErrorSource::TimingFailed(_)));

    let valid_setup_tb = RegTb {
        td: 2e-9,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Fake,
    };
    let work_dir = out_path(name, "sim_valid_setup");
    ctx._write_simulation::<RegTb>(&valid_setup_tb, work_dir, None, VerifyTiming::Yes(pvt))
        .expect("failed to run simulation");
}

#[test]
#[ignore = "long"]
fn test_pvt_timing_lookup() {
    let ctx = setup_ctx();
    let name = "test_setup_time_check";
    ctx.write_schematic_to_file::<Register>(&NoParams, out_path(name, "schematic.spice"))
        .expect("failed to write schematic");

    let corners = ctx.corner_db();
    let tt = corners.try_corner_named("tt").expect("no tt corner");
    let pvt_tt = Pvt::new(tt.clone(), 1.8, 25.0);
    let ss = corners.try_corner_named("ss").expect("no ss corner");
    let pvt_ss = Pvt::new(ss.clone(), 1.6, 100.0);

    let valid_setup_tb = RegTb {
        td: 2e-9,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Real,
    };
    let work_dir = out_path(name, "sim_valid_setup_tt");
    ctx._write_simulation::<RegTb>(
        &valid_setup_tb,
        work_dir,
        None,
        VerifyTiming::Yes(pvt_tt.clone()),
    )
    .expect("failed to run simulation");

    let work_dir = out_path(name, "sim_valid_setup_ss");
    ctx._write_simulation::<RegTb>(
        &valid_setup_tb,
        work_dir,
        None,
        VerifyTiming::Yes(pvt_ss.clone()),
    )
    .expect("failed to run simulation");

    let invalid_ss_setup_tb = RegTb {
        td: 100e-12,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Real,
    };
    let work_dir = out_path(name, "sim_invalid_ss_setup_tt");
    ctx._write_simulation::<RegTb>(
        &invalid_ss_setup_tb,
        work_dir,
        None,
        VerifyTiming::Yes(pvt_tt.clone()),
    )
    .expect("failed to run simulation");

    let work_dir = out_path(name, "sim_invalid_ss_setup_ss");
    let err = ctx
        ._write_simulation::<RegTb>(
            &invalid_ss_setup_tb,
            work_dir,
            None,
            VerifyTiming::Yes(pvt_ss.clone()),
        )
        .expect_err("expected setup timing checks to fail and return error");
    assert!(matches!(err.source(), ErrorSource::TimingFailed(_)));

    let invalid_tt_setup_tb = RegTb {
        td: 40e-12,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Real,
    };
    let work_dir = out_path(name, "sim_invalid_tt_setup_tt");
    ctx._write_simulation::<RegTb>(
        &invalid_tt_setup_tb,
        work_dir,
        None,
        VerifyTiming::Yes(pvt_tt),
    )
    .expect_err("expected setup timing checks to fail and return error");
    assert!(matches!(err.source(), ErrorSource::TimingFailed(_)));

    let work_dir = out_path(name, "sim_invalid_tt_setup_ss");
    let err = ctx
        ._write_simulation::<RegTb>(
            &invalid_tt_setup_tb,
            work_dir,
            None,
            VerifyTiming::Yes(pvt_ss),
        )
        .expect_err("expected setup timing checks to fail and return error");
    assert!(matches!(err.source(), ErrorSource::TimingFailed(_)));
}

#[test]
#[ignore = "long"]
fn test_register_timing_constraints() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<Register>(
        &NoParams,
        out_path("test_register_timing_constraints", "schematic.spice"),
    )
    .expect("failed to write schematic");

    let corners = ctx.corner_db();
    let tt = corners.try_corner_named("tt").expect("no tt corner");
    let pvt = Pvt::new(tt.clone(), 1.8, 25.0);

    let valid_tb = RegTb {
        td: 2e-9,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Real,
    };
    let work_dir = out_path("test_register_timing_constraints", "sim_valid");
    ctx._write_simulation::<RegTb>(&valid_tb, work_dir, None, VerifyTiming::Yes(pvt.clone()))
        .expect("failed to run simulation");

    let invalid_tb = RegTb {
        td: 40e-12,
        vdd: 1.8,
        tr: 20e-12,
        tf: 20e-12,
        rtype: RegisterType::Real,
    };
    let work_dir = out_path("test_register_timing_constraints", "sim_invalid");
    let err = ctx
        ._write_simulation::<RegTb>(&invalid_tb, work_dir, None, VerifyTiming::Yes(pvt))
        .expect_err("expected timing constraints to fail and return error");
    assert!(matches!(err.source(), ErrorSource::TimingFailed(_)));
}
