use std::cmp::Ordering;
use std::ops::Index;

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub enum SearchRange {
    /// Increase the search index.
    Up,
    /// Decrease the search index.
    Down,
    /// Stop searching and use the current index.
    #[default]
    Equal,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Serialize, Deserialize)]
pub enum SearchSide {
    /// Return the highest index for which the predicate returned [`SearchRange::Up`] or
    /// [`SearchRange::Equal`].
    Before,
    /// Return the lowest index for which the predicate returned [`SearchRange::Down`] or
    /// [`SearchRange::Equal`].
    After,
    /// Only accept elements for which the predicate returned [`SearchRange::Equal`].
    #[default]
    Equal,
}

impl From<std::cmp::Ordering> for SearchRange {
    fn from(value: Ordering) -> Self {
        match value {
            // If the current element is smaller than desired, search the upper range.
            Ordering::Less => SearchRange::Up,
            // If the current element is greater than desired, search the lower range.
            Ordering::Greater => SearchRange::Down,
            Ordering::Equal => SearchRange::Equal,
        }
    }
}

pub fn search<T, P>(lst: &[T], predicate: P, side: SearchSide) -> Option<(usize, &T)>
where
    P: FnMut(&T) -> SearchRange,
{
    search_in_range(lst, predicate, side, 0, lst.len() - 1)
}

pub fn search_in_range<'a, V, P>(
    lst: &'a V,
    mut predicate: P,
    side: SearchSide,
    mut lo: usize,
    mut hi: usize,
) -> Option<(usize, &'a V::Output)>
where
    V: Index<usize> + ?Sized,
    P: FnMut(&V::Output) -> SearchRange,
{
    let mut ans = None;

    while lo <= hi {
        let mid = (lo + hi) / 2;
        let val = &lst[mid];
        let pred = predicate(val);
        println!("lo = {lo}, hi = {hi}, mid = {mid}, pred = {pred:?}");
        match pred {
            SearchRange::Up => {
                lo = mid + 1;
                if let SearchSide::Before = side {
                    ans = Some((mid, val));
                }
            }
            SearchRange::Down => {
                hi = mid - 1;
                if let SearchSide::After = side {
                    ans = Some((mid, val));
                }
            }
            SearchRange::Equal => {
                return Some((mid, val));
            }
        }
    }

    ans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search() {
        let v = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 12, 14, 15];
        let (idx, val) = search(&v, |e| e.cmp(&11).into(), SearchSide::After).unwrap();
        assert_eq!(*val, 12);
        assert_eq!(idx, 9);

        let (idx, val) = search(&v, |e| e.cmp(&11).into(), SearchSide::Before).unwrap();
        assert_eq!(*val, 9);
        assert_eq!(idx, 8);

        let (idx, val) = search(&v, |e| e.cmp(&9).into(), SearchSide::After).unwrap();
        assert_eq!(*val, 9);
        assert_eq!(idx, 8);

        let (idx, val) = search(&v, |e| e.cmp(&9).into(), SearchSide::Before).unwrap();
        assert_eq!(*val, 9);
        assert_eq!(idx, 8);
    }
}
