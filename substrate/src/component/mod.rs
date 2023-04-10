//! APIs for creating SubComponents.

use std::any::Any;
use std::fmt::Display;

use arcstr::ArcStr;
use serde::{Deserialize, Serialize};

use crate::data::SubstrateCtx;
use crate::error::{ErrorSource, Result};
use crate::layout::context::LayoutCtx;
use crate::schematic::context::SchematicCtx;

pub mod error;

/// A view of a [`Component`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum View {
    /// A physical layout.
    ///
    /// Can be translated to a GDSII library.
    Layout,
    /// A digital, logical view.
    Digital,
    /// A schematic.
    Schematic,
    /// An abstract view of the physical layout.
    ///
    /// Can be translated to a LEF file.
    /// Typically includes information on pin layers/locations,
    /// blockages, and placement constraints.
    Abstract,
    /// A timing model of the component.
    ///
    /// Typically exported to a LIB file.
    Timing,
    /// A custom view, identified by a [`String`] name.
    Other(String),
}

impl Display for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use View::*;
        match *self {
            Layout => write!(f, "layout"),
            Digital => write!(f, "digital"),
            Schematic => write!(f, "schematic"),
            Abstract => write!(f, "abstract"),
            Timing => write!(f, "timing"),
            Other(ref name) => write!(f, "{name}"),
        }
    }
}

/// The trait that all SubComponents must implement.
///
/// # Examples
///
/// ```
#[doc = include_str!("../../tests/common/common_source.rs")]
/// ```
pub trait Component: Any {
    /// The parameter type.
    type Params: Serialize;

    /// Creates a new instance of this component with the given paramters.
    fn new(params: &Self::Params, ctx: &SubstrateCtx) -> Result<Self>
    where
        Self: Sized;

    /// Returns the desired name of this component, optionally
    /// taking into consideration relevant parameter values.
    ///
    /// The name should be compatible with SPICE/GDSII identifier requirements.
    ///
    /// If two components with the same name are used in the same schematic/layout/etc,
    /// Substrate will rename one of them. So this name should be thought of as a suggestion;
    /// it is not guaranteed that the returned component name will be used as-is in generated
    /// netlists.
    fn name(&self) -> ArcStr {
        arcstr::literal!("unnamed")
    }

    /// Creates a schematic view of this component.
    #[allow(unused_variables)]
    fn schematic(&self, ctx: &mut SchematicCtx) -> Result<()> {
        Err(ErrorSource::Component(error::Error::ViewUnsupported(View::Schematic)).into())
    }

    /// Creates a layout view of this component.
    #[allow(unused_variables)]
    fn layout(&self, ctx: &mut LayoutCtx) -> Result<()> {
        Err(ErrorSource::Component(error::Error::ViewUnsupported(View::Layout)).into())
    }
}

/// An empty type for components that are not parametrized.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub struct NoParams;

/// Uses [`flexbuffers`] to serialize component parameters.
///
/// For caching purposes.
pub(crate) fn serialize_params<T>(x: &T) -> Vec<u8>
where
    T: Serialize,
{
    let mut s = flexbuffers::FlexbufferSerializer::new();
    x.serialize(&mut s).unwrap();
    s.take_buffer()
}
