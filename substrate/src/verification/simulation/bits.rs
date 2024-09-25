use std::fmt::Display;

use bitvec::bitvec;
use bitvec::vec::BitVec;
use serde::{Deserialize, Serialize};

use super::waveform::Waveform;

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BitSignal {
    bits: BitVec,
}

impl BitSignal {
    #[inline]
    pub fn width(&self) -> usize {
        self.bits.len()
    }

    pub fn bit(&self, i: usize) -> bool {
        self.bits[i]
    }

    pub fn bits(&self) -> impl Iterator<Item = bool> + '_ {
        self.bits.iter().by_refs().copied()
    }

    pub fn bits_rev(&self) -> impl Iterator<Item = bool> + '_ {
        self.bits.iter().by_refs().rev().copied()
    }

    pub fn from_u32(mut value: u32, width: usize) -> Self {
        assert!(width <= 32);
        let mut bits = BitVec::with_capacity(width);
        for _ in 0..width {
            bits.push(value & 1 != 0);
            value >>= 1;
        }
        Self { bits }
    }

    pub fn from_u64(mut value: u64, width: usize) -> Self {
        assert!(width <= 64);
        let mut bits = BitVec::with_capacity(width);
        for _ in 0..width {
            bits.push(value & 1 != 0);
            value >>= 1;
        }
        Self { bits }
    }

    pub fn from_u128(mut value: u128, width: usize) -> Self {
        assert!(width <= 128);
        let mut bits = BitVec::with_capacity(width);
        for _ in 0..width {
            bits.push(value & 1 != 0);
            value >>= 1;
        }
        Self { bits }
    }

    #[inline]
    pub fn ones(width: usize) -> Self {
        Self {
            bits: bitvec![1; width],
        }
    }

    #[inline]
    pub fn zeros(width: usize) -> Self {
        Self {
            bits: bitvec![0; width],
        }
    }

    #[inline]
    pub fn from_vec(bits: Vec<bool>) -> Self {
        Self {
            bits: BitVec::from_iter(bits.iter()),
        }
    }

    #[inline]
    pub fn into_vec(self) -> Vec<bool> {
        self.bits().collect()
    }

    #[inline]
    pub fn from_slice(bits: &[bool]) -> Self {
        Self {
            bits: BitVec::from_iter(bits.iter()),
        }
    }

    /// Assigns the i-th bit to the given value.
    #[inline]
    pub fn assign(&mut self, i: usize, value: bool) {
        self.bits.set(i, value)
    }

    /// Clears the i-th bit (ie. sets it to 0).
    #[inline]
    pub fn clear(&mut self, i: usize) {
        self.bits.set(i, false)
    }

    /// Sets the i-th bit (ie. sets it to 1).
    #[inline]
    pub fn set(&mut self, i: usize) {
        self.bits.set(i, true)
    }

    pub fn inner(&self) -> &BitVec {
        &self.bits
    }

    pub fn inner_mut(&mut self) -> &BitVec {
        &mut self.bits
    }
}

impl From<Vec<bool>> for BitSignal {
    #[inline]
    fn from(value: Vec<bool>) -> Self {
        Self::from_vec(value)
    }
}

impl From<BitSignal> for Vec<bool> {
    fn from(value: BitSignal) -> Self {
        value.into_vec()
    }
}

impl From<BitVec> for BitSignal {
    fn from(value: BitVec) -> Self {
        Self { bits: value }
    }
}

impl From<BitSignal> for BitVec {
    fn from(value: BitSignal) -> Self {
        value.bits
    }
}

impl Display for BitSignal {
    /// Displays a binary representation of this bit signal, with the last bit displayed first.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}'b", self.width())?;
        if self.width() == 0 {
            write!(f, "0")?;
        } else {
            for b in self.bits_rev() {
                let s = if b { "1" } else { "0" };
                write!(f, "{}", s)?;
            }
        }
        Ok(())
    }
}

pub const DIGITAL_REL_TOL: f64 = 0.2;

pub fn is_logical_low(x: f64, vdd: f64) -> bool {
    (x / vdd).abs() < DIGITAL_REL_TOL
}

pub fn is_logical_high(x: f64, vdd: f64) -> bool {
    ((vdd - x) / vdd).abs() < DIGITAL_REL_TOL
}

pub fn logical_eq(x: f64, y: f64, vdd: f64) -> bool {
    ((x - y) / vdd).abs() < DIGITAL_REL_TOL
}

use thiserror::Error;

#[derive(Debug, Error)]
#[error("value {value} was not a valid logic level for vdd {vdd}")]
pub struct BitConvError {
    value: f64,
    vdd: f64,
}

pub fn to_bit(x: f64, vdd: f64) -> Result<bool, BitConvError> {
    if is_logical_low(x, vdd) {
        Ok(false)
    } else if is_logical_high(x, vdd) {
        Ok(true)
    } else {
        Err(BitConvError { value: x, vdd })
    }
}

pub fn push_bus(
    waveforms: &mut [Waveform],
    signal: &BitSignal,
    until: f64,
    vdd: f64,
    tr: f64,
    tf: f64,
) {
    assert_eq!(waveforms.len(), signal.width());
    for (i, bit) in signal.bits().enumerate() {
        if bit {
            waveforms[i].push_high(until, vdd, tr);
        } else {
            waveforms[i].push_low(until, vdd, tf);
        }
    }
}
