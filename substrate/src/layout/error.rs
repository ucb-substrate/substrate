//! Layout result and error types.

use super::convert::error::ErrorContext;

/// A layout error.
pub enum LayoutError {
    /// An error exporting to a foreign format.
    Export {
        message: String,
        stack: Vec<ErrorContext>,
    },
    /// An error importing from a foreign format.
    Import {
        message: String,
        stack: Vec<ErrorContext>,
    },
    /// A conversion error with a boxed external error.
    Conversion {
        message: String,
        err: Box<dyn std::error::Error + Send + Sync>,
        stack: Vec<ErrorContext>,
    },
    /// A boxed external error.
    Boxed(Box<dyn std::error::Error + Send + Sync>),
    /// An uncategorized error with string message.
    Str(String),
}

/// The [`LayoutError`] result type.
pub type LayoutResult<T> = Result<T, LayoutError>;

impl LayoutError {
    /// Creates a [`LayoutError::Str`] from anything String-convertible.
    pub fn msg(s: impl Into<String>) -> Self {
        Self::Str(s.into())
    }
    /// Creates an error-variant [`Result`] of our [`LayoutError::Str`] variant from anything String-convertible.
    pub fn fail<T>(s: impl Into<String>) -> Result<T, Self> {
        Err(Self::msg(s))
    }
}
impl std::fmt::Debug for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LayoutError::Export { message, stack } => {
                write!(f, "Export Error: \n - {message} \n - {stack:?}")
            }
            LayoutError::Import { message, stack } => {
                write!(f, "Import Error: \n - {message} \n - {stack:?}")
            }
            LayoutError::Conversion {
                message,
                err,
                stack,
            } => write!(
                f,
                "Conversion Error: \n - {message} \n - {err} \n - {stack:?}"
            ),
            LayoutError::Boxed(err) => err.fmt(f),
            LayoutError::Str(err) => err.fmt(f),
        }
    }
}
impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for LayoutError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Boxed(e) => Some(&**e),
            _ => None,
        }
    }
}

impl From<String> for LayoutError {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}
impl From<&str> for LayoutError {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}
impl From<std::num::TryFromIntError> for LayoutError {
    fn from(e: std::num::TryFromIntError) -> Self {
        Self::Boxed(Box::new(e))
    }
}

impl From<gds21::GdsError> for LayoutError {
    fn from(e: gds21::GdsError) -> Self {
        Self::Boxed(Box::new(e))
    }
}

impl<T: std::error::Error + Send + Sync + 'static> From<Box<T>> for LayoutError {
    fn from(e: Box<T>) -> Self {
        Self::Boxed(e)
    }
}
