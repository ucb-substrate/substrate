//! Cacheable design and measurement scripts.

pub(crate) mod map;

use std::any::Any;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::data::SubstrateCtx;
use crate::error::Result;

pub trait Script: Any {
    type Params: Serialize + for<'a> Deserialize<'a>;
    type Output: Send + Sync;

    fn run(params: &Self::Params, ctx: &SubstrateCtx) -> Result<Self::Output>;
}

pub trait Reload {
    /// Saves this object in the given directory.
    ///
    /// Should not write to any files outside the given directory.
    fn save(&self, dir: &Path) -> Result<()>;

    /// Loads this object from the given directory.
    ///
    /// Should not read from any files outside the given directory.
    fn load(dir: &Path) -> Result<Self>
    where
        Self: Sized;
}
