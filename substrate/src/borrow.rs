use std::ops::Deref;

use Shared::*;

use crate::layout::{Draw, DrawRef};

/// A shared value that is either borrowed or owned.
///
/// Essentially equivalent to [`Cow`](std::borrow::Cow),
/// but re-defined in this crate so that we can implement traits on it.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Shared<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> Default for Shared<'a, T>
where
    T: Default,
{
    #[inline]
    fn default() -> Self {
        Self::Owned(T::default())
    }
}

impl<'a, T> Deref for Shared<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match *self {
            Borrowed(b) => b,
            Owned(ref v) => v,
        }
    }
}

impl<'a, T> From<&'a T> for Shared<'a, T> {
    #[inline]
    fn from(value: &'a T) -> Self {
        Self::Borrowed(value)
    }
}

impl<'a, T> From<T> for Shared<'a, T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::Owned(value)
    }
}

impl<'a, T> Shared<'a, T> {
    pub fn borrow(other: &'a Shared<'a, T>) -> Self {
        match other {
            Borrowed(v) => Borrowed(v),
            Owned(ref v) => Borrowed(v),
        }
    }

    pub fn borrowed(&'a self) -> Self {
        match self {
            Borrowed(v) => Borrowed(v),
            Owned(ref v) => Borrowed(v),
        }
    }
}

impl<T> Shared<'static, T> {
    pub fn from_owned(value: T) -> Self {
        Self::Owned(value)
    }
}

impl<'a, T> Shared<'a, T> {
    pub fn from_borrow(value: &'a T) -> Self {
        Self::Borrowed(value)
    }
}

impl<'a, T> Draw for Shared<'a, T>
where
    T: DrawRef,
{
    fn draw(self) -> crate::error::Result<crate::layout::group::Group> {
        match self {
            Self::Owned(v) => v.draw(),
            Self::Borrowed(v) => v.draw_ref(),
        }
    }
}

impl<'a, T> DrawRef for Shared<'a, T>
where
    T: DrawRef,
{
    #[inline]
    fn draw_ref(&self) -> crate::error::Result<crate::layout::group::Group> {
        self.deref().draw_ref()
    }
}
