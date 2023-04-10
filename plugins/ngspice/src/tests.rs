use std::path::PathBuf;

use substrate::verification::simulation::{
    AcAnalysis, Analysis, AnalysisType, DcAnalysis, OpAnalysis, SimInput, Simulator, SimulatorOpts,
    SweepMode, TranAnalysis,
};

use crate::Ngspice;

pub(crate) const TEST_BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");
pub(crate) const EXAMPLES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

#[test]
fn vdivider_test() {
    let path = PathBuf::from(EXAMPLES_PATH).join("vdivider_tb.spice");
    let work_dir = PathBuf::from(TEST_BUILD_PATH).join("vdivider_tb/sim/");
    let input = SimInput {
        work_dir,
        analyses: vec![
            Analysis::Op(OpAnalysis {}),
            Analysis::Tran(
                TranAnalysis::builder()
                    .stop(5e-3f64)
                    .step(1e-3f64)
                    .build()
                    .unwrap(),
            ),
            Analysis::Ac(
                AcAnalysis::builder()
                    .fstop(1f64)
                    .fstart(1e-3f64)
                    .points(4)
                    .sweep(SweepMode::Dec)
                    .build()
                    .unwrap(),
            ),
            Analysis::Dc(
                DcAnalysis::builder()
                    .sweep("TEMP")
                    .start(200.0)
                    .stop(300.0)
                    .step(20.0)
                    .build()
                    .unwrap(),
            ),
        ],
        includes: vec![path],
        ..Default::default()
    };
    let opts = SimulatorOpts {
        opts: Default::default(),
    };

    let simulator = Ngspice::new(opts).unwrap();
    let out = simulator.simulate(input).unwrap();
    println!("{out:?}");

    assert_eq!(out.data.len(), 4);
    assert_eq!(out.data[0].analysis_type(), AnalysisType::Op);
    assert_eq!(out.data[1].analysis_type(), AnalysisType::Tran);
    assert_eq!(out.data[2].analysis_type(), AnalysisType::Ac);
    assert_eq!(out.data[3].analysis_type(), AnalysisType::Dc);
}
