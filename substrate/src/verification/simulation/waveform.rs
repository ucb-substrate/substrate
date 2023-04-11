use std::cmp::Ordering;
use std::iter::FusedIterator;

use serde::{Deserialize, Serialize};

use super::bits::{is_logical_high, is_logical_low};
use super::RealSignal;

/// A time-dependent waveform.
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Waveform {
    /// List of [`TimePoint`]s.
    values: Vec<TimePoint>,
}

pub struct SharedWaveform<'a> {
    t: &'a [f64],
    x: &'a [f64],
}

#[derive(Debug, Default, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TimePoint {
    t: f64,
    x: f64,
}

impl TimePoint {
    #[inline]
    pub fn new(t: f64, x: f64) -> Self {
        Self { t, x }
    }

    #[inline]
    pub fn t(&self) -> f64 {
        self.t
    }

    #[inline]
    pub fn x(&self) -> f64 {
        self.x
    }
}

pub trait TimeWaveform {
    fn get(&self, idx: usize) -> Option<TimePoint>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn first_t(&self) -> Option<f64> {
        Some(self.first()?.t())
    }

    fn first_x(&self) -> Option<f64> {
        Some(self.first()?.x())
    }

    fn last_t(&self) -> Option<f64> {
        Some(self.last()?.t())
    }

    fn last_x(&self) -> Option<f64> {
        Some(self.last()?.x())
    }

    fn first(&self) -> Option<TimePoint> {
        self.get(0)
    }

    fn last(&self) -> Option<TimePoint> {
        self.get(self.len() - 1)
    }

    fn edges(&self, threshold: f64) -> Edges<'_, Self> {
        Edges {
            waveform: self,
            idx: 0,
            thresh: threshold,
        }
    }

    fn values(&self) -> Values<'_, Self> {
        Values {
            waveform: self,
            idx: 0,
        }
    }

    fn time_index_before(&self, t: f64) -> Option<usize> {
        search_for_time(self, t)
    }

    /// Retrieves the value of the waveform at the given time.
    ///
    /// By default, linearly interpolates between two adjacent points on the waveform.
    fn sample_at(&self, t: f64) -> f64 {
        let idx = self
            .time_index_before(t)
            .expect("cannot extrapolate to the requested time");
        debug_assert!(
            idx < self.len() - 1,
            "cannot extrapolate beyond end of signal"
        );
        let p0 = self.get(idx).unwrap();
        let p1 = self.get(idx + 1).unwrap();
        linear_interp(p0.t(), p0.x(), p1.t(), p1.x(), t)
    }
}

fn linear_interp(t0: f64, y0: f64, t1: f64, y1: f64, t: f64) -> f64 {
    let c = (t - t0) / (t1 - t0);
    y0 + c * (y1 - y0)
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize)]
pub struct Values<'a, T: ?Sized> {
    waveform: &'a T,
    idx: usize,
}

impl<'a, T> Iterator for Values<'a, T>
where
    T: TimeWaveform,
{
    type Item = TimePoint;
    fn next(&mut self) -> Option<Self::Item> {
        let val = self.waveform.get(self.idx);
        if val.is_some() {
            self.idx += 1;
        }
        val
    }
}
impl<'a, T> FusedIterator for Values<'a, T> where T: TimeWaveform {}

impl TimeWaveform for Waveform {
    fn get(&self, idx: usize) -> Option<TimePoint> {
        self.values.get(idx).copied()
    }

    fn len(&self) -> usize {
        self.values.len()
    }
}

impl<'a> TimeWaveform for SharedWaveform<'a> {
    fn get(&self, idx: usize) -> Option<TimePoint> {
        if idx > self.len() {
            return None;
        }
        Some(TimePoint::new(self.t[idx], self.x[idx]))
    }

    fn len(&self) -> usize {
        self.t.len()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum EdgeDir {
    Falling,
    Rising,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Edge {
    pub(crate) t: f64,
    pub(crate) start_idx: usize,
    pub(crate) dir: EdgeDir,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize)]
pub struct Edges<'a, T: ?Sized> {
    waveform: &'a T,
    idx: usize,
    thresh: f64,
}

impl Waveform {
    #[inline]
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    pub fn from_signal(time: &RealSignal, value: &RealSignal) -> Self {
        let values = time
            .values
            .iter()
            .zip(value.values.iter())
            .map(|(t, x)| TimePoint::new(*t, *x))
            .collect();
        Self { values }
    }

    pub fn with_initial_value(x: f64) -> Self {
        Self {
            values: vec![TimePoint::new(0.0, x)],
        }
    }

    pub fn push(&mut self, t: f64, x: f64) {
        if let Some(tp) = self.last_t() {
            assert!(t > tp);
        }
        self.values.push(TimePoint::new(t, x));
    }

    pub fn push_high(&mut self, until: f64, vdd: f64, tr: f64) {
        if let Some(t) = self.last_t() {
            assert!(until > t);
        }
        if is_logical_low(self.last_x().unwrap_or(vdd), vdd) {
            self.push(self.last_t().unwrap() + tr, vdd);
        }
        self.push(until, vdd);
    }

    pub fn push_low(&mut self, until: f64, vdd: f64, tf: f64) {
        if let Some(t) = self.last_t() {
            assert!(until > t);
        }
        if is_logical_high(self.last_x().unwrap_or(0f64), vdd) {
            self.push(self.last_t().unwrap() + tf, 0f64);
        }
        self.push(until, 0f64);
    }

    pub fn push_bit(&mut self, bit: bool, until: f64, vdd: f64, t_transition: f64) {
        if bit {
            self.push_high(until, vdd, t_transition);
        } else {
            self.push_low(until, vdd, t_transition);
        }
    }
}

pub(crate) fn edge_crossing_time(t0: f64, y0: f64, t1: f64, y1: f64, thresh: f64) -> f64 {
    let c = (thresh - y0) / (y1 - y0);
    debug_assert!(c >= 0.0);
    debug_assert!(c <= 1.0);
    t0 + c * (t1 - t0)
}

impl<'a, T> Edges<'a, T>
where
    T: TimeWaveform,
{
    fn check(&mut self) -> Option<Edge> {
        let p0 = self.waveform.get(self.idx)?;
        let p1 = self.waveform.get(self.idx + 1)?;
        let first = p0.x - self.thresh;
        let second = p1.x - self.thresh;
        if first.signum() != second.signum() {
            let dir = if second.signum() > 0.0 {
                EdgeDir::Rising
            } else {
                EdgeDir::Falling
            };
            Some(Edge {
                dir,
                t: edge_crossing_time(p0.t, p0.x, p1.t, p1.x, self.thresh),
                start_idx: self.idx,
            })
        } else {
            None
        }
    }
}

impl<'a, T> Iterator for Edges<'a, T>
where
    T: TimeWaveform,
{
    type Item = Edge;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.waveform.len() - 1 {
            return None;
        }
        loop {
            let val = self.check();
            self.idx += 1;
            if val.is_some() {
                break val;
            }
            if self.idx >= self.waveform.len() - 1 {
                break None;
            }
        }
    }
}
impl<'a, T> FusedIterator for Edges<'a, T> where T: TimeWaveform {}

impl Default for Waveform {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for Waveform {
    type Output = TimePoint;
    fn index(&self, index: usize) -> &Self::Output {
        self.values.index(index)
    }
}

impl std::ops::IndexMut<usize> for Waveform {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.values.index_mut(index)
    }
}

impl Edge {
    /// The direction (rising or falling) of the edge.
    pub fn dir(&self) -> EdgeDir {
        self.dir
    }

    /// The time at which the waveform crossed the threshold.
    ///
    /// The waveform is linearly interpolated to find the threshold crossing time.
    pub fn t(&self) -> f64 {
        self.t
    }

    /// The index in the waveform **before** the threshold was passed.
    pub fn idx_before(&self) -> usize {
        self.start_idx
    }

    /// The index in the waveform **after** the threshold was passed.
    pub fn idx_after(&self) -> usize {
        self.start_idx + 1
    }
}

impl<'a> SharedWaveform<'a> {
    #[inline]
    pub fn new(t: &'a [f64], x: &'a [f64]) -> Self {
        assert_eq!(t.len(), x.len());
        Self { t, x }
    }

    pub fn from_signal(time: &'a RealSignal, value: &'a RealSignal) -> Self {
        assert_eq!(time.len(), value.len());
        Self {
            t: &time.values,
            x: &value.values,
        }
    }
}

impl From<(f64, f64)> for TimePoint {
    fn from(value: (f64, f64)) -> Self {
        Self {
            t: value.0,
            x: value.1,
        }
    }
}

fn search_for_time<T>(data: &T, target: f64) -> Option<usize>
where
    T: TimeWaveform + ?Sized,
{
    if data.is_empty() {
        return None;
    }

    let mut ans = None;
    let mut lo = 0usize;
    let mut hi = data.len() - 1;
    let mut x;
    while lo < hi {
        let mid = (lo + hi) / 2;
        x = data.get(mid).unwrap().t();
        match target.total_cmp(&x) {
            Ordering::Less => hi = mid - 1,
            Ordering::Greater => {
                lo = mid + 1;
                ans = Some(mid)
            }
            Ordering::Equal => return Some(mid),
        }
    }

    ans
}

pub(crate) fn binary_search_before(data: &[f64], target: f64) -> Option<usize> {
    if data.is_empty() {
        return None;
    }

    let mut ans = None;
    let mut lo = 0usize;
    let mut hi = data.len() - 1;
    let mut x;
    while lo < hi {
        let mid = (lo + hi) / 2;
        x = data[mid];
        match target.total_cmp(&x) {
            Ordering::Less => hi = mid - 1,
            Ordering::Greater => {
                lo = mid + 1;
                ans = Some(mid)
            }
            Ordering::Equal => return Some(mid),
        }
    }

    ans
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;
    use crate::into_vec;

    #[test]
    fn waveform_edges() {
        let wav = Waveform {
            values: into_vec![(0., 0.), (1., 1.), (2., 0.9), (3., 0.1), (4., 0.), (5., 1.)],
        };
        let edges = wav.edges(0.5).collect_vec();
        assert_eq!(
            edges,
            vec![
                Edge {
                    t: 0.5,
                    start_idx: 0,
                    dir: EdgeDir::Rising,
                },
                Edge {
                    t: 2.5,
                    start_idx: 2,
                    dir: EdgeDir::Falling,
                },
                Edge {
                    t: 4.5,
                    start_idx: 4,
                    dir: EdgeDir::Rising,
                }
            ]
        );
    }
}