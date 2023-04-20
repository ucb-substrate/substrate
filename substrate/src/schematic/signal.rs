use std::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use serde::{Deserialize, Serialize};
use slotmap::new_key_type;

use super::circuit::InstanceKey;
use crate::deps::arcstr::ArcStr;
use crate::index::IndexOwned;

new_key_type! {
    /// A key identifying a [`Signal`] within a [`Module`].
    pub struct SignalKey;
}

/// A path to a node in a circuit, starting from the top-level module.
///
/// Signal paths are preserved even after automatic module and instance renaming.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SignalPathBuf {
    pub(crate) insts: Vec<InstanceKey>,
    pub(crate) slice: SliceOne,
}

/// A path to a node in a circuit, starting from the top-level module.
///
/// Uses string identifiers instead of opaque IDs.
///
/// `NamedSignalPathBuf`s are NOT preserved after automatic module and instance renaming.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct NamedSignalPathBuf {
    pub insts: Vec<ArcStr>,
    pub signal: ArcStr,
    pub idx: Option<usize>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SignalInfo {
    name: ArcStr,
    width: usize,
    /// Whether or not this signal is a port.
    is_port: bool,
}

#[derive(Clone, Debug)]
pub struct Signal {
    parts: Vec<Slice>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Slice {
    signal: SignalKey,
    range: SliceRange,
}

/// A slice with a width of 1.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SliceOne {
    pub(crate) signal: SignalKey,
    pub(crate) idx: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SliceRange {
    start: usize,
    end: usize,
}

impl SliceOne {
    #[inline]
    pub fn new(signal: SignalKey, idx: usize) -> Self {
        Self { signal, idx }
    }

    pub fn from_slice(slice: Slice) -> Self {
        assert_eq!(slice.width(), 1);
        Self {
            signal: slice.signal,
            idx: slice.range.start,
        }
    }
}

impl From<SliceOne> for Slice {
    fn from(value: SliceOne) -> Self {
        Self {
            signal: value.signal,
            range: SliceRange::new(value.idx, value.idx + 1),
        }
    }
}

impl SliceRange {
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    #[inline]
    pub fn with_width(end: usize) -> Self {
        debug_assert!(end > 0);
        Self { start: 0, end }
    }

    #[inline]
    pub fn one() -> Self {
        Self::with_width(1)
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.end - self.start
    }
}

impl IntoIterator for SliceRange {
    type Item = usize;
    type IntoIter = std::ops::Range<usize>;
    fn into_iter(self) -> Self::IntoIter {
        self.start..self.end
    }
}

impl SignalInfo {
    #[inline]
    pub fn new(name: impl Into<ArcStr>, width: usize, is_port: bool) -> Self {
        Self {
            name: name.into(),
            width,
            is_port,
        }
    }

    #[inline]
    pub fn name(&self) -> &ArcStr {
        &self.name
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn set_name(&mut self, name: impl Into<ArcStr>) {
        self.name = name.into();
    }

    #[inline]
    pub fn is_port(&self) -> bool {
        self.is_port
    }
}

impl Slice {
    #[inline]
    pub fn new(signal: SignalKey, range: SliceRange) -> Self {
        Self { signal, range }
    }

    pub fn with_width(signal: SignalKey, width: usize) -> Self {
        assert!(width > 0);
        Self {
            signal,
            range: SliceRange::with_width(width),
        }
    }

    #[inline]
    pub fn range(&self) -> SliceRange {
        self.range
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.range.width()
    }

    #[inline]
    pub fn signal(&self) -> SignalKey {
        self.signal
    }

    #[inline]
    pub fn into_single(self) -> SliceOne {
        SliceOne::from_slice(self)
    }
}

impl IndexOwned<usize> for Slice {
    type Output = Self;
    fn index(&self, index: usize) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<Range<usize>> for Slice {
    type Output = Self;
    fn index(&self, index: Range<usize>) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<RangeFrom<usize>> for Slice {
    type Output = Self;
    fn index(&self, index: RangeFrom<usize>) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<RangeFull> for Slice {
    type Output = Self;
    fn index(&self, index: RangeFull) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<RangeInclusive<usize>> for Slice {
    type Output = Self;
    fn index(&self, index: RangeInclusive<usize>) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<RangeTo<usize>> for Slice {
    type Output = Self;
    fn index(&self, index: RangeTo<usize>) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<RangeToInclusive<usize>> for Slice {
    type Output = Self;
    fn index(&self, index: RangeToInclusive<usize>) -> Self::Output {
        Self::new(self.signal, self.range.index(index))
    }
}

impl IndexOwned<usize> for SliceRange {
    type Output = Self;
    fn index(&self, index: usize) -> Self::Output {
        let idx = self.start + index;
        assert!(idx < self.end, "index out of bounds");
        Self::new(idx, idx + 1)
    }
}

impl IndexOwned<Range<usize>> for SliceRange {
    type Output = Self;
    fn index(&self, index: Range<usize>) -> Self::Output {
        assert!(self.start + index.end <= self.end, "index out of bounds");
        Self::new(self.start + index.start, self.start + index.end)
    }
}

impl IndexOwned<RangeFrom<usize>> for SliceRange {
    type Output = Self;
    fn index(&self, index: RangeFrom<usize>) -> Self::Output {
        assert!(self.start + index.start <= self.end, "index out of bounds");
        Self::new(self.start + index.start, self.end)
    }
}

impl IndexOwned<RangeFull> for SliceRange {
    type Output = Self;
    fn index(&self, _index: RangeFull) -> Self::Output {
        *self
    }
}

impl IndexOwned<RangeInclusive<usize>> for SliceRange {
    type Output = Self;
    fn index(&self, index: RangeInclusive<usize>) -> Self::Output {
        assert!(self.start + index.end() < self.end, "index out of bounds");
        Self::new(self.start + index.start(), self.start + index.end() + 1)
    }
}

impl IndexOwned<RangeTo<usize>> for SliceRange {
    type Output = Self;
    fn index(&self, index: RangeTo<usize>) -> Self::Output {
        assert!(self.start + index.end <= self.end, "index out of bounds");
        Self::new(self.start, self.start + index.end)
    }
}

impl IndexOwned<RangeToInclusive<usize>> for SliceRange {
    type Output = Self;
    fn index(&self, index: RangeToInclusive<usize>) -> Self::Output {
        assert!(self.start + index.end < self.end, "index out of bounds");
        Self::new(self.start, self.start + index.end + 1)
    }
}

impl IndexOwned<usize> for Signal {
    type Output = Slice;

    fn index(&self, mut index: usize) -> Self::Output {
        for part in self.parts.iter() {
            let width = part.width();
            if index < width {
                return part.index(index);
            }
            index -= width;
        }
        panic!("index {index} out of bounds for signal");
    }
}

impl From<Slice> for Signal {
    fn from(value: Slice) -> Self {
        Self { parts: vec![value] }
    }
}

impl<T> From<T> for Signal
where
    T: AsRef<Slice>,
{
    fn from(value: T) -> Self {
        let slice = value.as_ref();
        Self {
            parts: vec![slice.to_owned()],
        }
    }
}

impl From<&Slice> for Signal {
    fn from(value: &Slice) -> Self {
        Self {
            parts: vec![value.to_owned()],
        }
    }
}

impl Signal {
    pub fn repeat(part: Slice, num: usize) -> Self {
        Self {
            parts: vec![part; num],
        }
    }
    pub fn new(parts: Vec<Slice>) -> Self {
        Self { parts }
    }
    #[inline]
    pub fn parts(&self) -> &[Slice] {
        &self.parts
    }

    pub fn width(&self) -> usize {
        self.parts.iter().map(Slice::width).sum()
    }
}

impl SignalPathBuf {
    #[inline]
    pub fn new(insts: Vec<InstanceKey>, slice: SliceOne) -> Self {
        Self { insts, slice }
    }
}
