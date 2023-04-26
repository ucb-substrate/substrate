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

#[inline]
pub fn search<T, P>(lst: &[T], predicate: P, side: SearchSide) -> Option<(usize, &T)>
where
    P: FnMut(&T) -> SearchRange,
{
    search_in_range(lst, predicate, side, 0, lst.len())
}

pub fn search_in_range<V, P>(
    lst: &V,
    mut predicate: P,
    side: SearchSide,
    mut lo: usize,
    mut hi: usize,
) -> Option<(usize, &V::Output)>
where
    V: Index<usize> + ?Sized,
    P: FnMut(&V::Output) -> SearchRange,
{
    if lo == hi {
        return None;
    }
    if predicate(&lst[lo]) == SearchRange::Down {
        return match side {
            SearchSide::After => Some((lo, &lst[lo])),
            _ => None,
        };
    }

    let initial_hi = hi;

    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        let val = &lst[mid];
        let pred = predicate(val);
        match pred {
            SearchRange::Up => {
                lo = mid;
            }
            SearchRange::Down => {
                hi = mid;
            }
            SearchRange::Equal => {
                return Some((mid, val));
            }
        }
    }

    if predicate(&lst[lo]) == SearchRange::Equal {
        return Some((lo, &lst[lo]));
    }
    match side {
        SearchSide::Before => Some((lo, &lst[lo])),
        SearchSide::After => {
            if hi < initial_hi {
                Some((hi, &lst[hi]))
            } else {
                None
            }
        }
        _ => None,
    }
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

        let (idx, val) = search(&v, |e| e.cmp(&15).into(), SearchSide::Equal).unwrap();
        assert_eq!(*val, 15);
        assert_eq!(idx, 11);

        let (idx, val) = search(&v, |e| e.cmp(&4).into(), SearchSide::Equal).unwrap();
        assert_eq!(*val, 4);
        assert_eq!(idx, 3);

        let result = search(&v, |e| e.cmp(&10).into(), SearchSide::Equal);
        assert!(result.is_none());
    }
}
