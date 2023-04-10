use std::path::{Path, PathBuf};

use empty_pdk::EmptyPdk;
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::pdk::{Pdk, PdkParams};

pub fn merge<T: AsRef<Path>>(
    output: impl AsRef<Path>,
    inputs: impl IntoIterator<Item = T>,
) -> substrate::error::Result<()> {
    let ctx = ctx();
    for f in inputs.into_iter() {
        ctx.from_gds(f.as_ref())?;
    }
    ctx.to_gds(output)?;
    Ok(())
}

pub fn ctx() -> SubstrateCtx {
    let cfg = SubstrateConfig::builder()
        .pdk(
            EmptyPdk::new(&PdkParams {
                pdk_root: PathBuf::from("."),
            })
            .unwrap(),
        )
        .build();
    SubstrateCtx::from_config(cfg).unwrap()
}
