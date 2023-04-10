use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use substrate::component::NoParams;
use substrate::data::SubstrateCtx;
use substrate::script::Script;

mod common;
use common::setup_ctx;

pub struct MyDesignScript;

pub struct Output {
    wn: usize,
    wp: usize,
}

lazy_static! {
    static ref RUNS: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));
}

impl Script for MyDesignScript {
    type Params = NoParams;
    type Output = Output;

    fn run(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self::Output> {
        *RUNS.lock().unwrap() += 1;
        Ok(Output { wn: 6, wp: 12 })
    }
}

#[test]
fn test_design_script() {
    let ctx = setup_ctx();

    let output = ctx.run_script::<MyDesignScript>(&NoParams).unwrap();
    assert_eq!(output.wn, 6);
    assert_eq!(output.wp, 12);

    // When running the script a second time, the results should be cached.
    let output = ctx.run_script::<MyDesignScript>(&NoParams).unwrap();
    assert_eq!(output.wn, 6);
    assert_eq!(output.wp, 12);

    // The script should only run once despite `run_script` being called twice.
    assert_eq!(*RUNS.lock().unwrap(), 1);
}
