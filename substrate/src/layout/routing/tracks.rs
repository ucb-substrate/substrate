//! Track management, including sizing and spacing.

use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use subgeom::{Sign, Span};

use crate::index::IndexOwned;

/// Specifier for a track relative to a position.
///
/// If the position is already on the appropriate edge of a track, that track is specified.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TrackLocator {
    /// The track nearest a position.
    Nearest,
    /// The track nearest a position that starts before that position.
    StartsBefore,
    /// The track nearest a position that starts beyond that position.
    StartsAfter,
    /// The track nearest a position that ends before that position.
    EndsBefore,
    /// The track nearest a position that ends beyond that position.
    EndsAfter,
}

/// An infinite number of tracks, starting at a fixed location.
#[derive(Builder, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct UniformTracks {
    pub line: i64,
    pub space: i64,
    pub start: i64,
    pub sign: Sign,
}

/// A fixed number of tracks.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FixedTracks {
    pub line: i64,
    pub space: i64,
    pub boundary_space: i64,
    pub interior_tracks: usize,
    pub start: i64,
    pub lower_boundary: Boundary,
    pub upper_boundary: Boundary,
    pub sign: Sign,
}

pub struct FixedTracksIter<'a> {
    idx: usize,
    ptr: &'a FixedTracks,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct CenteredTrackParams {
    pub line: i64,
    pub space: i64,
    pub num: usize,
    pub span: Span,
    pub lower_boundary: Boundary,
    pub upper_boundary: Boundary,
    pub grid: i64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum Boundary {
    Space,
    HalfSpace,
    Track,
    HalfTrack,
}

impl FixedTracks {
    pub fn from_centered_tracks(params: CenteredTrackParams) -> Self {
        let num = params.num;
        let lower = params.lower_boundary.num_tracks();
        let upper = params.upper_boundary.num_tracks();
        assert!(num > lower + upper);

        let interior_tracks = num - lower - upper;

        let margin = params.span.length()
            - params.lower_boundary.width(params.line, params.space)
            - params.upper_boundary.width(params.line, params.space)
            - params.line * (interior_tracks as i64)
            - params.space * (interior_tracks as i64 - 1);
        let boundary_space = margin / 2;
        assert_eq!(
            boundary_space % params.grid,
            0,
            "calculated boundary spacing {} is off grid for grid {}",
            boundary_space,
            params.grid
        );

        Self {
            line: params.line,
            space: params.space,
            boundary_space,
            interior_tracks: num - lower - upper,
            start: params.span.start(),
            lower_boundary: params.lower_boundary,
            upper_boundary: params.upper_boundary,
            sign: Sign::Pos,
        }
    }

    fn lower_tracks(&self) -> usize {
        self.lower_boundary.num_tracks()
    }
    fn upper_tracks(&self) -> usize {
        self.upper_boundary.num_tracks()
    }
    fn num_tracks(&self) -> usize {
        self.lower_tracks() + self.interior_tracks + self.upper_tracks()
    }
    fn boundary_width(&self, boundary: Boundary) -> i64 {
        boundary.width(self.line, self.space)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.num_tracks()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> FixedTracksIter {
        FixedTracksIter { idx: 0, ptr: self }
    }
}

impl<'a> Iterator for FixedTracksIter<'a> {
    type Item = Span;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.ptr.len() {
            return None;
        }
        let res = self.ptr.index(self.idx);
        self.idx += 1;
        Some(res)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.ptr.len() - self.idx;
        (len, Some(len))
    }
}

impl<'a> IntoIterator for &'a FixedTracks {
    type Item = Span;
    type IntoIter = FixedTracksIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Boundary {
    fn num_tracks(&self) -> usize {
        match self {
            Self::Space | Self::HalfSpace => 0,
            Self::Track | Self::HalfTrack => 1,
        }
    }

    fn width(&self, line: i64, space: i64) -> i64 {
        use Boundary::*;
        match self {
            Space => space,
            HalfSpace => space / 2,
            Track => line,
            HalfTrack => line / 2,
        }
    }
}

impl IndexOwned<usize> for FixedTracks {
    type Output = Span;
    fn index(&self, mut index: usize) -> Self::Output {
        assert!(
            index < self.num_tracks(),
            "track index {} out of bounds; the number of tracks is {}",
            index,
            self.num_tracks()
        );
        let sgn = self.sign.as_int();
        if index < self.lower_tracks() {
            return Span::new(
                self.start,
                self.start + sgn * self.boundary_width(self.lower_boundary),
            );
        }
        index -= self.lower_tracks();

        if self.interior_tracks == 0 {
            assert!(self.upper_tracks() > 0);
            let start = self.start
                + self.boundary_width(self.lower_boundary)
                + 2 * sgn * self.boundary_space;
            return Span::new(
                start,
                start + sgn * self.boundary_width(self.upper_boundary),
            );
        }

        if index >= self.interior_tracks {
            let start = self.start
                + sgn
                    * (self.boundary_width(self.lower_boundary)
                        + self.boundary_space
                        + (self.line + self.space) * (self.interior_tracks as i64 - 1)
                        + self.line
                        + self.boundary_space);
            Span::new(
                start,
                start + sgn * self.boundary_width(self.upper_boundary),
            )
        } else {
            let start = self.start
                + sgn
                    * (self.boundary_width(self.lower_boundary)
                        + self.boundary_space
                        + (self.line + self.space) * (index as i64));
            Span::new(start, start + sgn * self.line)
        }
    }
}

impl UniformTracks {
    #[inline]
    pub fn builder() -> UniformTracksBuilder {
        UniformTracksBuilder::default()
    }

    pub fn track_at(&self, pos: i64) -> i64 {
        (pos - self.start) / (self.line + self.space)
    }

    pub fn track_with_loc(&self, loc: TrackLocator, pos: i64) -> i64 {
        match loc {
            TrackLocator::Nearest => {
                let before = self.track_with_loc(TrackLocator::StartsBefore, pos);
                let after = self.track_with_loc(TrackLocator::EndsAfter, pos);

                if self.index(after).distance_to(pos) < self.index(before).distance_to(pos) {
                    after
                } else {
                    before
                }
            }
            TrackLocator::StartsBefore => self.track_at(pos),
            TrackLocator::StartsAfter => (pos - self.start - 1) / (self.line + self.space) + 1,
            TrackLocator::EndsBefore => {
                self.track_with_loc(TrackLocator::StartsBefore, pos - self.line)
            }
            TrackLocator::EndsAfter => {
                self.track_with_loc(TrackLocator::StartsAfter, pos - self.line)
            }
        }
    }
}

impl IndexOwned<i64> for UniformTracks {
    type Output = Span;
    fn index(&self, index: i64) -> Self::Output {
        let sgn = self.sign.as_int();
        let start = self.start + sgn * index * (self.line + self.space);
        let stop = start + sgn * self.line;
        Span::new(start, stop)
    }
}

impl IndexOwned<usize> for UniformTracks {
    type Output = Span;
    fn index(&self, index: usize) -> Self::Output {
        let index = i64::try_from(index).expect("index must be at most `i64::MAX`");
        self.index(index)
    }
}
