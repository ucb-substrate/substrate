//! Utilities for indexing.

/// Index into an object.
///
///
/// Unlike [`std::ops::Index`], allows implementors
/// to return ownership of data, rather than just a reference.
pub trait IndexOwned<Idx>
where
    Idx: ?Sized,
{
    type Output;

    fn index(&self, index: Idx) -> Self::Output;
}
