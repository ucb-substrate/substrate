pub mod concat;
pub mod context;
pub mod module;
pub mod modules;
pub mod wire;
// pub mod rtlgen;
pub mod types;
pub mod validation;

use self::context::DigitalCtx;
use self::module::{Instance, Port};
use self::wire::WireKey;
use crate::component::Component;
use crate::error::SubstrateError;

/// The `Result` type for digital component generators.
pub type Result<T> = std::result::Result<T, SubstrateError>;

pub trait DigitalComponent: Component {
    type Interface: Interface;
    fn interface(&self) -> Self::Interface;
    fn digital(
        &self,
        ctx: &mut DigitalCtx,
        input: <Self::Interface as Interface>::Input,
    ) -> Result<<Self::Interface as Interface>::Output>;
}

pub trait Interface {
    type Parent: ParentModulePort;
    type Input: ModulePort;
    type Output: ModulePort;

    fn parent(&self, instance: Instance, ctx: &mut DigitalCtx) -> Self::Parent;
    fn input(&self, ctx: &mut DigitalCtx) -> Self::Input;
    fn ports() -> Vec<Port>;
}

pub trait ParentModulePort: ModulePort {
    fn instance(&self) -> &Instance;
    fn into_instance(self) -> Instance;
}

pub trait ModulePort {
    fn port(&self, name: &str) -> WireKey;
}
