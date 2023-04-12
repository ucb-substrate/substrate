#![allow(dead_code)]

use std::path::PathBuf;

use ngspice::Ngspice;
use sky130_open_pdk::Sky130OpenPdk;
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::pdk::PdkParams;
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::verification::simulation::{Simulator, SimulatorOpts};

pub mod common_source;
pub mod sp_cell;
pub mod vdivider;

pub const DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/tests");
pub const LIB_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/tests/lib");
pub const BUILD_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/build");

pub fn gds_path(name: &str) -> PathBuf {
    PathBuf::from(DATA_DIR).join(format!("gds/{name}.gds"))
}

/// Returns the path to the hard macro config TOML file with the given name.
pub fn hm_toml_path(name: &str) -> PathBuf {
    PathBuf::from(DATA_DIR).join(format!("hard_macros/{name}.toml"))
}

pub fn out_path(test_name: &str, file_name: &str) -> PathBuf {
    PathBuf::from(BUILD_DIR).join(format!("tests/{test_name}/{file_name}"))
}

pub fn setup_ctx() -> SubstrateCtx {
    let simulator = Ngspice::new(SimulatorOpts::default()).unwrap();
    let pdk_root = std::env::var("SKY130_OPEN_PDK_ROOT").expect("the SKY130_OPEN_PDK_ROOT environment variable should be set to the root of the skywater-pdk repository").into();

    let cfg = SubstrateConfig::builder()
        .netlister(SpiceNetlister::new())
        .simulator(simulator)
        .pdk(Sky130OpenPdk::new(&PdkParams { pdk_root }).unwrap())
        .build();
    SubstrateCtx::from_config(cfg).unwrap()
}
