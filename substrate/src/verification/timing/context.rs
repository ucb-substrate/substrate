use super::{TimingConstraint, TimingView};
use crate::schematic::circuit::PortError;
use crate::schematic::module::Module;
use crate::schematic::signal::Slice;

#[derive(Clone)]
pub struct TimingCtx {
    module: Module,
}

impl TimingCtx {
    pub fn add_constraint(&mut self, constraint: impl Into<TimingConstraint>) {
        self.module.timing_mut().constraints.push(constraint.into())
    }

    #[inline]
    pub fn new(module: Module) -> Self {
        Self { module }
    }

    #[inline]
    pub(crate) fn into_inner(self) -> Module {
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
