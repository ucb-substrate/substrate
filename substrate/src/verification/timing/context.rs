use super::{TimingConstraint, TimingView};
use crate::data::SubstrateCtx;
use crate::schematic::circuit::PortError;
use crate::schematic::module::Module;
use crate::schematic::signal::Slice;

#[derive(Clone)]
pub struct TimingCtx {
    module: Module,
    inner: SubstrateCtx,
}

impl TimingCtx {
    pub fn add_constraint(&mut self, constraint: impl Into<TimingConstraint>) {
        self.module.timing_mut().constraints.push(constraint.into())
    }

    #[inline]
    pub(crate) fn new(module: Module, inner: SubstrateCtx) -> Self {
        Self { module, inner }
    }

    #[inline]
    pub fn inner(&self) -> &SubstrateCtx {
        &self.inner
    }

    #[inline]
    pub(crate) fn into_module(self) -> Module {
        self.module
    }

    pub fn try_port(&self, name: &str) -> Result<Slice, PortError> {
        let port = self.module.port(name)?;
        Ok(Slice::with_width(port.signal, port.width))
    }

    pub fn port(&self, name: &str) -> Slice {
        let port = self.module.port(name).expect("no such port");
        Slice::with_width(port.signal, port.width)
    }
}
