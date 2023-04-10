use std::path::PathBuf;

use approx::abs_diff_eq;
use substrate::verification::simulation::{
    AcAnalysis, Analysis, AnalysisType, SimInput, Simulator, SimulatorOpts, SweepMode, TranAnalysis,
};

use crate::Spectre;

pub(crate) const TEST_BUILD_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");
pub(crate) const EXAMPLES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/examples");

#[test]
#[ignore = "requires Spectre"]
fn vdivider_test() {
    let path = PathBuf::from(EXAMPLES_PATH).join("vdivider_tb.spice");
    let work_dir = PathBuf::from(TEST_BUILD_PATH).join("vdivider_tb/sim/");
    let input = SimInput {
        work_dir,
        analyses: vec![
            Analysis::Tran(
                TranAnalysis::builder()
                    .stop(6e-3f64)
                    .step(1e-3f64)
                    .build()
                    .unwrap(),
            ),
            Analysis::Tran(
                TranAnalysis::builder()
                    .stop(8e-3f64)
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
            Analysis::Ac(
                AcAnalysis::builder()
                    .fstop(1f64)
                    .fstart(1e-2f64)
                    .points(4)
                    .sweep(SweepMode::Dec)
                    .build()
                    .unwrap(),
            ),
            Analysis::Tran(
                TranAnalysis::builder()
                    .stop(10e-3f64)
                    .step(1e-3f64)
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

    let simulator = Spectre::new(opts).unwrap();
    let out = simulator.simulate(input).unwrap();
    println!("{out:?}");

    assert_eq!(out.data.len(), 5);

    assert_eq!(out.data[0].analysis_type(), AnalysisType::Tran);
    let out_time = &out.data[0].tran().time;
    assert!(abs_diff_eq!(
        out_time.get(out_time.len() - 1).unwrap(),
        6e-3f64
    ));

    assert_eq!(out.data[1].analysis_type(), AnalysisType::Tran);
    let out_time = &out.data[1].tran().time;
    assert!(abs_diff_eq!(
        out_time.get(out_time.len() - 1).unwrap(),
        8e-3f64
    ));

    assert_eq!(out.data[2].analysis_type(), AnalysisType::Ac);
    let out_freq = &out.data[2].ac().freq;
    assert!(abs_diff_eq!(out_freq.get(0).unwrap(), 1e-3f64));

    assert_eq!(out.data[3].analysis_type(), AnalysisType::Ac);
    let out_freq = &out.data[3].ac().freq;
    assert!(abs_diff_eq!(out_freq.get(0).unwrap(), 1e-2f64));

    assert_eq!(out.data[4].analysis_type(), AnalysisType::Tran);
    let out_time = &out.data[4].tran().time;
    assert!(abs_diff_eq!(
        out_time.get(out_time.len() - 1).unwrap(),
        10e-3f64
    ));
}
