use std::fmt::{Debug, Display};
use std::path::PathBuf;

use thiserror::Error;

use crate::component::{self, View};
use crate::deps::arcstr::ArcStr;
use crate::layout::cell::PortError;
use crate::layout::error::LayoutError;
use crate::layout::routing;
use crate::layout::straps::PowerStrapError;
use crate::pdk::corner::error::ProcessCornerError;
use crate::pdk::mos::error::MosError;
use crate::pdk::stdcell::error::StdCellError;
use crate::schematic::circuit::PortError as SchematicPortError;
use crate::schematic::netlist::interface::NetlistError;
use crate::verification::simulation::bits::BitConvError;
use crate::verification::timing::TimingReport;

pub type Result<T> = std::result::Result<T, SubstrateError>;

pub struct SubstrateError {
    pub(crate) source: ErrorSource,
    pub(crate) context: Vec<ErrorContext>,
}

impl SubstrateError {
    pub fn source(&self) -> &ErrorSource {
        &self.source
    }
}

impl std::error::Error for SubstrateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl Display for SubstrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Error:\n{}", self.source)?;
        if !self.context.is_empty() {
            writeln!(f, "\nError occurred:")?;
            for item in self.context.iter() {
                writeln!(f, "\twhile {}", item)?;
            }
        }
        Ok(())
    }
}

impl Debug for SubstrateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.source)?;
        if !self.context.is_empty() {
            writeln!(f, "\nError occurred:")?;
            for (i, item) in self.context.iter().enumerate() {
                writeln!(f, "\t{}: {:?}", i, item)?;
            }
        }
        Ok(())
    }
}

impl<T> From<T> for SubstrateError
where
    T: Into<ErrorSource>,
{
    fn from(value: T) -> Self {
        Self {
            source: value.into(),
            context: Vec::new(),
        }
    }
}

impl SubstrateError {
    pub fn new(source: impl Into<ErrorSource>) -> Self {
        Self {
            source: source.into(),
            context: Vec::new(),
        }
    }

    pub fn from_context(source: impl Into<ErrorSource>, ctx: impl Into<ErrorContext>) -> Self {
        Self {
            source: source.into(),
            context: vec![ctx.into()],
        }
    }

    pub fn with_context(mut self, ctx: impl Into<ErrorContext>) -> Self {
        self.context.push(ctx.into());
        self
    }

    #[inline]
    pub fn into_inner(self) -> ErrorSource {
        self.source
    }
}

#[inline]
pub fn with_err_context<T, E, C>(result: std::result::Result<T, E>, ctx: C) -> Result<T>
where
    C: FnOnce() -> ErrorContext,
    E: Into<SubstrateError>,
{
    result.map_err(|err| err.into().with_context(ctx()))
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum ErrorContext {
    GenComponent {
        name: ArcStr,
        type_name: ArcStr,
        view: View,
    },
    InitComponent {
        type_name: ArcStr,
    },
    CreateDir(PathBuf),
    CreateFile(PathBuf),
    ReadFile(PathBuf),
    Task(ArcStr),
}

impl Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorContext::*;
        match self {
            GenComponent {
                name,
                type_name,
                view,
            } => write!(
                f,
                "generating {view} view of component {type_name} ({name})"
            ),
            InitComponent { type_name } => write!(f, "initializing component {type_name}"),
            CreateDir(path) => write!(f, "creating directory {path:?}"),
            CreateFile(path) => write!(f, "creating file {path:?}"),
            ReadFile(path) => write!(f, "reading file {path:?}"),
            Task(task) => write!(f, "{task}"),
        }
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ErrorSource {
    #[error("error generating component: {0}")]
    Component(#[from] component::error::Error),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("cannot netlist external modules")]
    NetlistExternalModule,

    #[error("invalid netlist (enable logging for details): {0}")]
    InvalidNetlist(String),

    #[error("invalid layout (enable logging for details): {0}")]
    InvalidLayout(String),

    #[error("error while generating netlist: {0}")]
    Netlist(#[from] NetlistError),

    #[error("error while generating layout: {0}")]
    Layout(#[from] LayoutError),

    #[error("no such device")]
    DeviceNotFound,

    #[error("error while generating MOSFET device: {0}")]
    Mos(#[from] MosError),

    #[error("no such module: {0}")]
    ModuleNotFound(String),

    #[error("error accessing standard cells: {0}")]
    StdCell(#[from] StdCellError),

    #[error("error accessing process corners: {0}")]
    ProcessCorner(#[from] ProcessCornerError),

    #[error("power strap error: {0}")]
    PowerStrapError(#[from] PowerStrapError),

    #[error("no such layer: {0}")]
    LayerNotFound(String),

    #[error("no such port: {0}")]
    PortNotFound(ArcStr),

    #[error("no tool specified")]
    ToolNotSpecified,

    #[error("invalid pdk")]
    InvalidPdk,

    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("already exists: {0}")]
    AlreadyExists(ArcStr),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("error parsing SPICE: {0}")]
    SpiceParsing(#[from] subspice::error::Error),

    #[error("error parsing TOML: {0}")]
    TomlParsing(#[from] toml::de::Error),

    #[error("error writing TOML: {0}")]
    TomlWriting(#[from] toml::ser::Error),

    #[error("error parsing JSON: {0}")]
    JsonParsing(#[from] serde_json::Error),

    #[error("port index out of bounds: {index} is out of bounds for port with width {width}")]
    PortIndexOutOfBounds { width: usize, index: usize },

    #[error("error accessing layout port: {0}")]
    LayoutPort(#[from] PortError),

    #[error("error accessing schematic port: {0}")]
    SchematicPort(#[from] SchematicPortError),

    #[error("error performing automatic routing: {0}")]
    AutoRouting(#[from] routing::auto::error::Error),

    #[error("error converting signal to logic level: {0}")]
    BitConv(#[from] BitConvError),

    #[error("timing constraints not satisfied; see report for more details")]
    TimingFailed(TimingReport),

    #[error("unexpected error: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("unexpected error: {0}")]
    Anyhow(#[from] anyhow::Error),
}
