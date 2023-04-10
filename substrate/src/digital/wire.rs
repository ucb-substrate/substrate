use std::marker::PhantomData;
use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use arcstr::ArcStr;
use bitvec::prelude::Lsb0;
use bitvec::vec::BitVec;
use bitvec::view::BitView;
use lazy_static::lazy_static;
use serde::Serialize;
use slotmap::{new_key_type, SlotMap};

use super::concat::Concat;
use super::types::{Bool, Clock, Hardware, HardwareType, Result as TypeResult, TypeError, UInt};
use super::validation::TypeValidatorOutput;
use crate::index::IndexOwned;

new_key_type! {
    #[doc(hidden)]
    pub struct WireKey;
}

pub(crate) struct WireDb {
    inner: Arc<RwLock<WireDbInner>>,
}

pub(crate) type WireDbInner = SlotMap<WireKey, WireInner>;

impl WireDb {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SlotMap::with_key())),
        }
    }

    pub(crate) fn inner(&self) -> RwLockReadGuard<WireDbInner> {
        self.inner.read().unwrap()
    }

    pub(crate) fn inner_mut(&self) -> RwLockWriteGuard<WireDbInner> {
        self.inner.write().unwrap()
    }
}

lazy_static! {
    pub(crate) static ref WIRE_DB: WireDb = WireDb::new();
}

#[derive(Debug)]
pub struct Wire<T> {
    key: WireKey,
    phantom: PhantomData<T>,
}

impl<T> Clone for Wire<T> {
    fn clone(&self) -> Wire<T> {
        *self
    }
}

impl<T> Copy for Wire<T> {}

#[derive(Debug, Serialize)]
pub struct Literal<T: Hardware> {
    pub(crate) inner: LiteralInner,
    pub(crate) t: T,
    phantom: PhantomData<T>,
}

impl<T: Hardware> Clone for Literal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            t: self.t.clone(),
            phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LiteralInner {
    pub(crate) hw_type: HardwareType,
    pub(crate) bits: BitVec,
}

#[derive(Debug, Clone)]
pub struct WireInner {
    pub(crate) hw_type: HardwareType,
    #[allow(unused)]
    pub(crate) value: WireValue,
}

#[derive(Debug, Clone)]
pub(crate) enum WireValue {
    Literal(BitVec),
    BinOp(Op, WireKey, WireKey),
    #[allow(unused)]
    Cast(HardwareType, WireKey),
    Port(ArcStr),
    /// Concatenation of wires in MSB order.
    Concat(WireKey, WireKey),
    /// Slice of wire.
    Slice(Range<usize>, WireKey),
    Reg(Reg),
    /// Output of an instance.
    InstanceOutput,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub(crate) struct Reg {
    pub(crate) clk: WireKey,
    pub(crate) reset: Option<RegReset>,
    pub(crate) d: WireKey,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub(crate) struct RegReset {
    /// The signal that triggers a register reset.
    pub(crate) reset: WireKey,
    /// The value the register takes upon being reset.
    pub(crate) value: LiteralInner,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Op {
    Add,
}

impl<T: Hardware> From<Literal<T>> for Wire<T> {
    fn from(value: Literal<T>) -> Self {
        Self::new(WireInner {
            hw_type: value.inner.hw_type,
            value: WireValue::Literal(value.inner.bits),
        })
    }
}

impl<T> Wire<T> {
    pub(crate) fn new(inner: WireInner) -> Self {
        let mut db = WIRE_DB.inner_mut();
        let key = db.insert(inner);
        drop(db);
        Self {
            key,
            phantom: PhantomData,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn _inner(&self) -> WireKey {
        self.key
    }

    #[allow(unused)]
    fn validate(&self) -> TypeValidatorOutput {
        let db = WIRE_DB.inner();
        let mut output = TypeValidatorOutput::new();
        db[self.key].validate(&db, &mut output);
        output
    }

    pub fn reg(&self, clk: Wire<Clock>) -> Wire<T> {
        let db = WIRE_DB.inner();
        let hw_type = self.hw_type_inner(&db).clone();
        drop(db);

        Wire::new(WireInner::new(
            hw_type,
            WireValue::Reg(Reg {
                clk: clk.key,
                reset: None,
                d: self.key,
            }),
        ))
    }

    fn hw_type_inner<'a>(&self, db: &'a WireDbInner) -> &'a HardwareType {
        &db[self.key].hw_type
    }

    #[allow(unused)]
    fn literal_value<'a>(&self, db: &'a WireDbInner) -> Option<&'a BitVec> {
        if let WireValue::Literal(vec) = &db[self.key].value {
            Some(vec)
        } else {
            None
        }
    }

    #[allow(unused)]
    fn value<'a>(&self, db: &'a RwLockReadGuard<SlotMap<WireKey, WireInner>>) -> &'a WireValue {
        &db[self.key].value
    }
}

impl<T: Hardware> Wire<T> {
    pub fn reg_reset(&self, clk: Wire<Clock>, rst: Wire<Bool>, reset_value: Literal<T>) -> Wire<T> {
        let db = WIRE_DB.inner();
        Wire::new(WireInner::new(
            self.hw_type_inner(&db).clone(),
            WireValue::Reg(Reg {
                clk: clk.key,
                reset: Some(RegReset {
                    reset: rst.key,
                    value: reset_value.inner,
                }),
                d: self.key,
            }),
        ))
    }

    pub fn from_literal(literal: Literal<T>) -> Self {
        Self::from(literal)
    }
}

impl Wire<UInt> {
    pub fn literal(value: usize, width: usize) -> TypeResult<Self> {
        let raw = value.view_bits::<Lsb0>();
        let bits = raw.iter_ones().last().unwrap_or(0) + 1;
        if bits > width {
            Err(TypeError::LiteralOverflow(value, width, bits))
        } else {
            Ok(Self::new(WireInner::new(
                HardwareType::UInt(UInt::new(width)),
                WireValue::Literal(raw[..bits].to_bitvec()),
            )))
        }
    }
    pub fn from_bitvec(vec: BitVec<usize, Lsb0>) -> Self {
        Self::new(WireInner::new(
            HardwareType::UInt(UInt::new(vec.len())),
            WireValue::Literal(vec),
        ))
    }

    pub fn hw_type(&self) -> UInt {
        let db = WIRE_DB.inner();
        if let HardwareType::UInt(uint) = &db[self.key].hw_type {
            *uint
        } else {
            // Should never have a different `HardwareType` within a `Wire<UInt>`.
            unreachable!()
        }
    }

    pub fn width(&self) -> usize {
        self.hw_type().width()
    }
}

impl<T: Hardware> Literal<T> {
    pub(crate) fn t(&self) -> &T {
        &self.t
    }
    pub(crate) fn from_typed_bits(t: T, bits: BitVec) -> Self {
        Self {
            inner: LiteralInner {
                hw_type: t.clone().into(),
                bits,
            },
            t,
            phantom: PhantomData,
        }
    }
}

impl Literal<UInt> {
    pub fn new(value: usize, width: usize) -> TypeResult<Self> {
        let raw = value.view_bits::<Lsb0>();
        let bits = raw.iter_ones().last().unwrap_or(0) + 1;
        if bits > width {
            Err(TypeError::LiteralOverflow(value, width, bits))
        } else {
            Ok(Self::from_typed_bits(
                UInt::new(width),
                raw[..bits].to_bitvec(),
            ))
        }
    }
}

impl Literal<Bool> {
    pub fn new(value: bool) -> TypeResult<Self> {
        let mut bv = BitVec::with_capacity(1);
        bv.push(value);
        Ok(Self::from_typed_bits(Bool, bv))
    }
}

impl std::ops::Add for Wire<UInt> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(WireInner {
            hw_type: HardwareType::UInt(UInt::new(std::cmp::max(self.width(), rhs.width()))),
            value: WireValue::BinOp(Op::Add, self.key, rhs.key),
        })
    }
}

impl Concat<Wire<UInt>> for Wire<UInt> {
    type Output = Self;

    fn concat(self, other: Self) -> Self::Output {
        Self::new(WireInner {
            hw_type: HardwareType::UInt(UInt::new(self.width() + other.width())),
            value: WireValue::Concat(self.key, other.key),
        })
    }
}

impl IndexOwned<usize> for Wire<UInt> {
    type Output = Self;
    fn index(&self, index: usize) -> Self::Output {
        assert!(index < self.width(), "index out of bounds");
        Self::new(WireInner {
            hw_type: UInt::new(1).into(),
            value: WireValue::Slice(index..index + 1, self.key),
        })
    }
}

impl IndexOwned<Range<usize>> for Wire<UInt> {
    type Output = Self;
    fn index(&self, index: Range<usize>) -> Self::Output {
        assert!(index.start < index.end, "invalid index range");
        assert!(index.end <= self.width(), "index out of bounds");
        Self::new(WireInner {
            hw_type: UInt::new(index.end - index.start).into(),
            value: WireValue::Slice(index, self.key),
        })
    }
}

impl IndexOwned<RangeFrom<usize>> for Wire<UInt> {
    type Output = Self;
    fn index(&self, index: RangeFrom<usize>) -> Self::Output {
        let width = self.width();
        assert!(index.start < width, "index out of bounds");
        Self::new(WireInner {
            hw_type: UInt::new(width - index.start).into(),
            value: WireValue::Slice(index.start..width, self.key),
        })
    }
}

impl IndexOwned<RangeFull> for Wire<UInt> {
    type Output = Self;
    fn index(&self, _index: RangeFull) -> Self::Output {
        *self
    }
}

impl IndexOwned<RangeInclusive<usize>> for Wire<UInt> {
    type Output = Self;
    fn index(&self, index: RangeInclusive<usize>) -> Self::Output {
        assert!(index.start() <= index.end(), "invalid index range");
        assert!(index.end() < &self.width(), "index out of bounds");
        Self::new(WireInner {
            hw_type: UInt::new(index.end() - index.start() + 1).into(),
            value: WireValue::Slice(*index.start()..index.end() + 1, self.key),
        })
    }
}

impl IndexOwned<RangeTo<usize>> for Wire<UInt> {
    type Output = Self;
    fn index(&self, index: RangeTo<usize>) -> Self::Output {
        assert!(index.end <= self.width(), "index out of bounds");
        Self::new(WireInner {
            hw_type: UInt::new(index.end).into(),
            value: WireValue::Slice(0..index.end, self.key),
        })
    }
}

impl IndexOwned<RangeToInclusive<usize>> for Wire<UInt> {
    type Output = Self;
    fn index(&self, index: RangeToInclusive<usize>) -> Self::Output {
        assert!(index.end < self.width(), "index out of bounds");
        Self::new(WireInner {
            hw_type: UInt::new(index.end + 1).into(),
            value: WireValue::Slice(0..index.end + 1, self.key),
        })
    }
}

impl WireInner {
    pub(crate) fn new(hw_type: HardwareType, value: WireValue) -> Self {
        Self { hw_type, value }
    }
}

#[cfg(test)]
mod tests {
    use bitvec::bitvec;

    use super::*;

    #[test]
    fn test_uint_literal() {
        Wire::literal(0, 1).expect("failed to create literal");
        Wire::literal(1, 1).expect("failed to create literal");

        Wire::literal(0, 2).expect("failed to create literal");
        Wire::literal(1, 2).expect("failed to create literal");
        Wire::literal(2, 2).expect("failed to create literal");
        Wire::literal(3, 2).expect("failed to create literal");

        let usize_literal = Wire::literal(2, 2).unwrap();
        let bitvec_literal = Wire::from_bitvec(bitvec![0, 1]);

        let db = WIRE_DB.inner();
        assert_eq!(usize_literal.literal_value(&db).unwrap(), &bitvec![0, 1]);
        assert_eq!(
            usize_literal.hw_type_inner(&db),
            &HardwareType::UInt(UInt::new(2))
        );

        assert_eq!(
            bitvec_literal.literal_value(&db).unwrap(),
            usize_literal.literal_value(&db).unwrap(),
            "incorrect literal created from bitvec"
        );

        assert_eq!(
            bitvec_literal.hw_type_inner(&db),
            usize_literal.hw_type_inner(&db),
            "incorrect type created from bitvec"
        );
    }

    #[test]
    fn test_uint_literal_overflow() {
        Wire::literal(2, 1).expect_err("should cause overflow error");
        Wire::literal(3, 1).expect_err("should cause overflow error");
        Wire::literal(4, 2).expect_err("should cause overflow error");
        Wire::literal(5, 2).expect_err("should cause overflow error");
    }

    #[test]
    fn test_uint_add() {
        let a = Wire::literal(2, 2).expect("failed to create literal");
        let b = Wire::literal(3, 2).expect("failed to create literal");
        let c = a + b;
        assert_eq!(c.hw_type(), UInt::new(2));
        let db = WIRE_DB.inner();
        assert!(matches!(c.value(&db), &WireValue::BinOp(_, _, _)));
        drop(db);
        assert!(!c.validate().has_errors());
    }

    #[test]
    fn test_uint_add_error() {
        let a = Wire::literal(2, 2).expect("failed to create literal");
        let b = Wire::literal(3, 5).expect("failed to create literal");
        let c = a + b;
        let output = c.validate();
        assert!(output.has_errors());
    }

    #[test]
    fn test_uint_concat() {
        let a = Wire::literal(2, 2).expect("failed to create literal");
        let zeros = Wire::literal(0, 5).expect("failed to create literal");
        let b = zeros.concat(a);

        assert_eq!(b.hw_type(), UInt::new(7));
        let db = WIRE_DB.inner();
        assert!(matches!(b.value(&db), &WireValue::Concat(_, _)));
        drop(db);
        assert!(!b.validate().has_errors());
    }

    #[test]
    fn test_uint_slice() {
        let a = Wire::literal(2, 20).expect("failed to create literal");

        let b = a.index(19);
        assert_eq!(b.hw_type(), UInt::new(1));
        let db = WIRE_DB.inner();
        assert!(matches!(
            b.value(&db),
            &WireValue::Slice(Range { start: 19, end: 20 }, _)
        ));
        drop(db);
        assert!(!b.validate().has_errors());

        let b = a.index(13..15);
        assert_eq!(b.hw_type(), UInt::new(2));
        let db = WIRE_DB.inner();
        assert!(matches!(
            b.value(&db),
            &WireValue::Slice(Range { start: 13, end: 15 }, _)
        ));
        drop(db);
        assert!(!b.validate().has_errors());

        let b = a.index(13..20);
        assert_eq!(b.hw_type(), UInt::new(7));
        let db = WIRE_DB.inner();
        assert!(matches!(
            b.value(&db),
            &WireValue::Slice(Range { start: 13, end: 20 }, _)
        ));
        drop(db);
        assert!(!b.validate().has_errors());

        let b = a.index(13..=15);
        assert_eq!(b.hw_type(), UInt::new(3));
        let db = WIRE_DB.inner();
        assert!(matches!(
            b.value(&db),
            &WireValue::Slice(Range { start: 13, end: 16 }, _)
        ));
        drop(db);
        assert!(!b.validate().has_errors());

        let b = a.index(..=15);
        assert_eq!(b.hw_type(), UInt::new(16));
        let db = WIRE_DB.inner();
        assert!(matches!(
            b.value(&db),
            &WireValue::Slice(Range { start: 0, end: 16 }, _)
        ));
        drop(db);
        assert!(!b.validate().has_errors());
    }

    #[test]
    #[should_panic]
    fn test_uint_slice_invalid() {
        let a = Wire::literal(2, 20).expect("failed to create literal");
        a.index(13..21);
    }

    #[test]
    fn test_uint_all_ops() {
        let a = Wire::literal(2, 2).expect("failed to create literal");
        let zeros = Wire::literal(0, 3).expect("failed to create literal");
        let b = Wire::literal(3, 5).expect("failed to create literal");

        let c = zeros.concat(a) + b;
        assert_eq!(c.hw_type(), UInt::new(5));
        let db = WIRE_DB.inner();
        assert!(matches!(c.value(&db), &WireValue::BinOp(_, _, _)));
        drop(db);
        assert!(!c.validate().has_errors());

        let d = c.index(0..2) + a;
        assert_eq!(d.hw_type(), UInt::new(2));
        let db = WIRE_DB.inner();
        assert!(matches!(d.value(&db), &WireValue::BinOp(_, _, _)));
        drop(db);
        assert!(!d.validate().has_errors());
    }

    #[test]
    fn test_uint_nested_error() {
        let a = Wire::literal(2, 2).expect("failed to create literal");
        let b = Wire::literal(3, 5).expect("failed to create literal");
        let c = Wire::literal(3, 5).expect("failed to create literal");
        let d = Wire::literal(3, 5).expect("failed to create literal");

        let e = (a + b).concat(c).concat(d).index(2..12).concat(a);
        assert_eq!(e.hw_type(), UInt::new(12));
        let db = WIRE_DB.inner();
        assert!(matches!(e.value(&db), &WireValue::Concat(_, _)));
        drop(db);
        assert!(e.validate().has_errors());
    }
}
