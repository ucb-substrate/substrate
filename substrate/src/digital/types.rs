use std::any::Any;

use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::wire::Op;

/// An enumeration of type errors.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Error)]
pub enum TypeError {
    /// Literal is too large.
    #[error("Literal {0} is too large: expected width {1}, got width {2}")]
    LiteralOverflow(usize, usize, usize),
    /// Operands have incorrect widths.
    #[error("Expected operand of width {0} for output of width {1}, got {2}")]
    InvalidOperandWidth(usize, usize, usize),
    /// Operands have incorrect widths.
    #[error("Operands must have matching widths: {0} != {1}")]
    OperandWidthMismatch(usize, usize),
    /// Operands have invalid types.
    #[error("Cannot apply operation {0:?} to types {1:?} and {1:?}")]
    InvalidOperandType(Op, HardwareType, HardwareType),
    /// Operands have incorrect widths.
    #[error("Expected {0:?}, got {1:?}")]
    InvalidType(HardwareType, HardwareType),
    /// Literal is inconsistent with its type.
    #[error("Literal {0} is invalid for {1:?}")]
    InvalidLiteral(BitVec, HardwareType),
    /// Invalid cast.
    #[error("Cannot cast {0:?} to {1:?}")]
    InvalidCast(HardwareType, HardwareType),
    /// Unexpected errors.
    #[error("unexpected error: {0}")]
    Other(String),
}

/// The `Result` type for digital types.
pub type Result<T> = std::result::Result<T, TypeError>;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum HardwareType {
    UInt(UInt),
    Bool(Bool),
    Clock(Clock),
    Vector(Vector),
}

pub trait Hardware: Into<HardwareType> + Serialize + Deserialize<'static> + Clone + Any {}
impl Hardware for UInt {}
impl Hardware for Bool {}
impl Hardware for Clock {}
impl Hardware for Vector {}

impl HardwareType {
    pub fn get_uint(&self) -> Option<UInt> {
        if let HardwareType::UInt(uint) = self {
            Some(*uint)
        } else {
            None
        }
    }

    #[allow(unused)]
    pub(crate) fn castable_to(self, other: HardwareType) -> bool {
        self == other
    }
}

impl From<UInt> for HardwareType {
    fn from(value: UInt) -> Self {
        Self::UInt(value)
    }
}

impl From<Bool> for HardwareType {
    fn from(value: Bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Clock> for HardwareType {
    fn from(value: Clock) -> Self {
        Self::Clock(value)
    }
}

impl From<Vector> for HardwareType {
    fn from(value: Vector) -> Self {
        Self::Vector(value)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Vector {
    /// The length of the [`Vector`].
    len: usize,
    /// The type of the elements in the [`Vector`].
    elem: Box<HardwareType>,
}

#[derive(Copy, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Clock;

#[derive(Copy, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct Bool;

#[derive(Copy, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct UInt {
    width: usize,
}

impl UInt {
    pub fn new(width: usize) -> Self {
        Self { width }
    }
    pub fn width(&self) -> usize {
        self.width
    }
}
