//! APIs for layout generation.

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use self::group::Group;

pub mod cell;
pub mod context;
pub mod convert;
pub mod elements;
pub mod error;
pub mod group;
pub mod layers;
pub mod placement;
pub mod routing;
pub mod straps;
pub mod validation;

/// An enumeration of layout formats.
#[derive(Default, Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum LayoutFormat {
    #[default]
    Gds,
}

/// A trait implemented by objects that can be drawn
/// inside a layout cell's context.
#[enum_dispatch]
pub trait Draw {
    /// Draws the object.
    fn draw(self) -> crate::error::Result<Group>;
}

/// A non-consuming trait implemented by objects that can be drawn
/// inside a layout cell's context.
#[enum_dispatch]
pub trait DrawRef: Draw {
    /// Draws the object.
    fn draw_ref(&self) -> crate::error::Result<Group>;
}
