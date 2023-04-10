use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Default, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SiValue {
    value: i64,
    prefix: SiPrefix,
}

impl SiValue {
    #[inline]
    pub fn zero() -> Self {
        Self::default()
    }

    #[inline]
    pub fn new(value: i64, prefix: SiPrefix) -> Self {
        Self { value, prefix }
    }

    pub fn value(&self) -> i64 {
        self.value
    }

    pub fn prefix(&self) -> SiPrefix {
        self.prefix
    }

    /// Creates a new [`SiValue`] by rounding to the given precision.
    ///
    /// For example, if [`SiPrefix::Micro`] is given, and `value` is given in Volts,
    /// `value` will be rounded to the nearest microvolt before being stored in the
    /// resulting [`SiValue`].
    pub fn with_precision(value: f64, precision: SiPrefix) -> Self {
        let value = (value / precision.multiplier()).round() as i64;
        Self {
            value,
            prefix: precision,
        }
    }
}

impl From<SiValue> for f64 {
    #[inline]
    fn from(value: SiValue) -> Self {
        value.value as f64 * value.prefix.multiplier()
    }
}

#[derive(
    Copy, Clone, Default, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize,
)]
pub enum SiPrefix {
    Yocto,
    Zepto,
    Atto,
    Femto,
    Pico,
    Nano,
    Micro,
    Milli,
    #[default]
    None,
    Kilo,
    Mega,
    Giga,
    Tera,
    Peta,
    Exa,
    Zetta,
    Yotta,
}

impl SiPrefix {
    pub fn multiplier(&self) -> f64 {
        match self {
            SiPrefix::Yocto => 1e-24,
            SiPrefix::Zepto => 1e-21,
            SiPrefix::Atto => 1e-18,
            SiPrefix::Femto => 1e-15,
            SiPrefix::Pico => 1e-12,
            SiPrefix::Nano => 1e-9,
            SiPrefix::Micro => 1e-6,
            SiPrefix::Milli => 1e-3,
            SiPrefix::None => 1e0,
            SiPrefix::Kilo => 1e3,
            SiPrefix::Mega => 1e6,
            SiPrefix::Giga => 1e9,
            SiPrefix::Tera => 1e12,
            SiPrefix::Peta => 1e15,
            SiPrefix::Exa => 1e18,
            SiPrefix::Zetta => 1e21,
            SiPrefix::Yotta => 1e24,
        }
    }
}

impl Display for SiValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.value, self.prefix)
    }
}

impl Display for SiPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match *self {
            Self::Femto => "f",
            Self::Pico => "p",
            Self::Nano => "n",
            Self::Micro => "u",
            Self::Milli => "m",
            Self::None => "",
            Self::Kilo => "K",
            Self::Mega => "MEG",
            Self::Giga => "G",
            Self::Tera => "T",
            _ => panic!("unsupported prefix: {:?}", self),
        };

        write!(f, "{s}")
    }
}
