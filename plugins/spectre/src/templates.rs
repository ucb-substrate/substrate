use std::path::{Path, PathBuf};

use lazy_static::lazy_static;
use serde::Serialize;
use substrate::error::{ErrorSource, Result};
use substrate::verification::simulation::Lib;
use tera::{Context, Tera};

use crate::TOP_NETLIST_NAME;

pub(crate) const TEMPLATES_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");

lazy_static! {
    pub(crate) static ref TEMPLATES: Tera = {
        match Tera::new(&format!("{TEMPLATES_PATH}/*")) {
            Ok(t) => t,
            Err(e) => {
                panic!("Encountered errors while parsing Tera templates: {e}");
            }
        }
    };
}

#[derive(Serialize)]
pub(crate) struct NetlistCtx<'a> {
    pub(crate) libs: &'a [Lib],
    pub(crate) includes: &'a [PathBuf],
    pub(crate) analyses: &'a [String],
    pub(crate) spectre_directives: &'a [String],
    pub(crate) spice_directives: &'a [String],
}

pub(crate) fn render_netlist(ctx: NetlistCtx<'_>, path: impl AsRef<Path>) -> Result<()> {
    let ctx = Context::from_serialize(ctx)
        .map_err(|e| ErrorSource::Internal(format!("template error: {e}")))?;

    let mut file = std::fs::File::create(&path)?;
    TEMPLATES
        .render_to(TOP_NETLIST_NAME, &ctx, &mut file)
        .map_err(|e| ErrorSource::Internal(format!("template error: {e}")))?;

    Ok(())
}
