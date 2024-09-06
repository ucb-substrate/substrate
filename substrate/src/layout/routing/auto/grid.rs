//! APIs for interacting with router grids.
use subgeom::bbox::{Bbox, BoundBox};
use subgeom::{Corner, Dir, Edge, Rect, Side, Sides, Sign, Span};

use super::abs::{Layer, PosSpan};
use super::GreedyRouter;
use crate::index::IndexOwned;
use crate::layout::context::LayoutCtx;
use crate::layout::elements::via::{Via, ViaParams};
use crate::layout::group::Group;
use crate::layout::layers::LayerKey;
use crate::layout::routing::tracks::{TrackLocator, UniformTracks};
use crate::layout::Draw;

/// Strategy for bringing an off-grid bus onto the grid.
#[derive(Clone, Copy, Debug)]
pub enum OffGridBusTranslationStrategy {
    /// Brings the bus onto grid on the same layer.
    Parallel,
    /// Brings the bus onto grid via a perpendicular layer.
    Perpendicular(LayerKey),
}

/// An off-grid bus translation to an on-grid bus.
pub struct OffGridBusTranslation {
    /// The translation strategy.
    strategy: OffGridBusTranslationStrategy,
    /// The bus geometry layer.
    layer: LayerKey,
    /// The output edge to be translated to the grid.
    ///
    /// Should be in the direction of wires in the bus if using the
    /// `OffGridBusTranslationStrategy::Parallel`.
    ///
    /// If using `OffGridBusTranslationStrategy::Perpendicular`, should
    /// be in the direction of the output bus wires and centered where
    /// the output bus should be centered.
    output: Edge,
    /// The line of the bus.
    line: i64,
    /// The space of the bus.
    space: i64,
    /// The start coordinate of the bus.
    start: i64,
    /// The number of wires in the bus.
    n: i64,
    /// The desired shift of the on-grid output wires.
    ///
    /// Selects between potential uniformly spaced grid tracks
    /// for placing output wires. A shift of 1 corresponds to offsetting
    /// the selected grid tracks by one track.
    shift: i64,
    /// The desired number of grid spaces between output tracks.
    ///
    /// A `output_pitch` of 1 corresponds to placing output tracks into
    /// adjacent grid tracks.
    output_pitch: i64,
}

/// A builder for an `OffGridBusTranslation`.
#[derive(Default)]
pub struct OffGridBusTranslationBuilder {
    strategy: Option<OffGridBusTranslationStrategy>,
    layer: Option<LayerKey>,
    output: Option<Edge>,
    line: Option<i64>,
    space: Option<i64>,
    start: Option<i64>,
    n: Option<i64>,
    shift: Option<i64>,
    output_pitch: Option<i64>,
}

impl OffGridBusTranslation {
    /// Creates a builder for an `OffGridBusTranslation`.
    pub fn builder() -> OffGridBusTranslationBuilder {
        OffGridBusTranslationBuilder::default()
    }

    pub fn output_span(&self) -> Span {
        Span::with_start_and_length(
            self.start,
            self.line + (self.line + self.space) * (self.n - 1),
        )
    }
}

impl OffGridBusTranslationBuilder {
    /// Specifies the translation strategy.
    pub fn strategy(&mut self, strategy: OffGridBusTranslationStrategy) -> &mut Self {
        self.strategy = Some(strategy);
        self
    }

    /// Specifies the bus geometry layer.
    pub fn layer(&mut self, layer: LayerKey) -> &mut Self {
        self.layer = Some(layer);
        self
    }

    /// Specifies the output edge of the bus.
    ///
    /// Should correspond to the edge of the bus wires that should be translated
    /// to on-grid output wires.
    pub fn output(&mut self, output: Edge) -> &mut Self {
        self.output = Some(output);
        self
    }

    /// Specifies the line and space of the bus.
    pub fn line_and_space(&mut self, line: i64, space: i64) -> &mut Self {
        self.line = Some(line);
        self.space = Some(space);
        self
    }

    /// Specifies the starting coordinate of the bus.
    pub fn start(&mut self, start: i64) -> &mut Self {
        self.start = Some(start);
        self
    }

    /// Specifies the number of wires in the bus.
    pub fn n(&mut self, n: i64) -> &mut Self {
        self.n = Some(n);
        self
    }

    /// Specifies the desired shift of the on-grid output wires.
    ///
    /// `shift` selects between potential uniformly spaced grid tracks
    /// for placing output wires. A shift of 1 corresponds to offsetting
    /// the selected grid tracks by one track.
    pub fn shift(&mut self, shift: i64) -> &mut Self {
        self.shift = Some(shift);
        self
    }

    /// Specifies the desired number of grid spaces between output tracks.
    ///
    /// A `output_pitch` of 1 corresponds to placing output tracks into
    /// adjacent grid tracks.
    pub fn output_pitch(&mut self, output_pitch: i64) -> &mut Self {
        self.output_pitch = Some(output_pitch);
        self
    }

    /// Builds an `OffGridBusTranslation`.
    pub fn build(&mut self) -> OffGridBusTranslation {
        OffGridBusTranslation {
            strategy: self.strategy.unwrap(),
            layer: self.layer.unwrap(),
            output: self.output.unwrap(),
            line: self.line.unwrap(),
            space: self.space.unwrap(),
            start: self.start.unwrap(),
            n: self.n.unwrap(),
            shift: self.shift.unwrap_or(0),
            output_pitch: self.output_pitch.unwrap_or(1),
        }
    }
}

pub struct OnGridBus {
    ports: Vec<Rect>,
}

impl OnGridBus {
    pub fn ports(&self) -> impl Iterator<Item = Rect> + '_ {
        self.ports.iter().copied()
    }
}

/// An enumeration of strategies for expanding off-grid geometry to the grid.
pub enum ExpandToGridStrategy {
    Minimum,
    All,
    Side(Side),
    Corner(Corner),
}

/// A jog for bringing off-grid geometry onto the grid.
pub struct JogToGrid {
    layer: LayerKey,
    rect: Rect,
    dst_layer: LayerKey,
    width: i64,
    first_dir: Option<Side>,
    second_dir: Option<Side>,
    extend_first: i64,
    extend_second: i64,
}

/// A builder for a [`JogToGrid`].
#[derive(Default)]
pub struct JogToGridBuilder {
    layer: Option<LayerKey>,
    rect: Option<Rect>,
    dst_layer: Option<LayerKey>,
    width: Option<i64>,
    first_dir: Option<Side>,
    second_dir: Option<Side>,
    extend_first: Option<i64>,
    extend_second: Option<i64>,
}

impl JogToGrid {
    pub fn builder() -> JogToGridBuilder {
        JogToGridBuilder::default()
    }
}

impl JogToGridBuilder {
    /// Specifies the geometry layer.
    pub fn layer(&mut self, layer: LayerKey) -> &mut Self {
        self.layer = Some(layer);
        self
    }

    /// Specifies the geometry.
    pub fn rect(&mut self, rect: Rect) -> &mut Self {
        self.rect = Some(rect);
        self
    }

    /// Specifies the destination layer.
    ///
    /// Defaults to the geometry layer.
    pub fn dst_layer(&mut self, layer: LayerKey) -> &mut Self {
        self.dst_layer = Some(layer);
        self
    }

    /// Specifies the width of the jog.
    pub fn width(&mut self, width: i64) -> &mut Self {
        self.width = Some(width);
        self
    }

    /// Specifies the first direction that the jog should extend.
    pub fn first_dir(&mut self, first_dir: Side) -> &mut Self {
        self.first_dir = Some(first_dir);
        self
    }

    /// Specifies the second direction that the jog should extend.
    ///
    /// Must be perpendicular to `first_dir`. Only used if `first_dir` is specified.
    pub fn second_dir(&mut self, second_dir: Side) -> &mut Self {
        self.second_dir = Some(second_dir);
        self
    }

    /// Extends the first leg of the jog by `amount` grid tracks.
    pub fn extend_first(&mut self, amount: i64) -> &mut Self {
        self.extend_first = Some(amount);
        self
    }

    /// Extends the second leg of the jog by `amount` grid tracks.
    pub fn extend_second(&mut self, amount: i64) -> &mut Self {
        self.extend_second = Some(amount);
        self
    }

    /// Builds an `JogToGrid`.
    pub fn build(&mut self) -> JogToGrid {
        let layer = self.layer.unwrap();
        let jog = JogToGrid {
            layer,
            rect: self.rect.unwrap(),
            dst_layer: self.dst_layer.unwrap_or(layer),
            width: self.width.unwrap(),
            first_dir: self.first_dir,
            second_dir: self.second_dir,
            extend_first: self.extend_first.unwrap_or(0),
            extend_second: self.extend_second.unwrap_or(0),
        };
        if let (Some(first_dir), Some(second_dir)) = (jog.first_dir, jog.second_dir) {
            assert_ne!(first_dir.coord_dir(), second_dir.coord_dir());
        }
        jog
    }
}

impl GreedyRouter {
    pub(crate) fn track_span(&self, layer: Layer, coord: usize) -> Span {
        let layer_info = self.inner.layer_info(layer);
        let track_info = self.track_info(self.layer(layer));
        track_info.tracks.index(coord / layer_info.grid_space)
    }

    pub(crate) fn grid_track(&self, dir: Dir) -> &UniformTracks {
        match dir {
            Dir::Horiz => &self.grid_htracks,
            Dir::Vert => &self.grid_vtracks,
        }
    }

    pub(crate) fn shrink_to_pos_span(&self, layer: LayerKey, rect: Rect) -> PosSpan {
        let (x_grid, y_grid) = (&self.grid_vtracks, &self.grid_htracks);

        let tx_min = x_grid
            .track_with_loc(TrackLocator::StartsAfter, rect.left())
            .try_into()
            .unwrap_or(0);
        let tx_max = x_grid
            .track_with_loc(TrackLocator::EndsBefore, rect.right())
            .try_into()
            .unwrap_or(0);
        let ty_min = y_grid
            .track_with_loc(TrackLocator::StartsAfter, rect.bottom())
            .try_into()
            .unwrap_or(0);
        let ty_max = y_grid
            .track_with_loc(TrackLocator::EndsBefore, rect.top())
            .try_into()
            .unwrap_or(0);

        PosSpan {
            layer: self.abs_layer(layer),
            tx_min,
            tx_max,
            ty_min,
            ty_max,
        }
    }

    pub(crate) fn expand_to_pos_span(&self, layer: LayerKey, rect: Rect) -> PosSpan {
        let (x_grid, y_grid) = (&self.grid_vtracks, &self.grid_htracks);

        let tx_min = x_grid
            .track_with_loc(TrackLocator::StartsBefore, rect.left())
            .try_into()
            .unwrap_or(0);
        let tx_max = x_grid
            .track_with_loc(TrackLocator::EndsAfter, rect.right())
            .try_into()
            .unwrap_or(0);
        let ty_min = y_grid
            .track_with_loc(TrackLocator::StartsBefore, rect.bottom())
            .try_into()
            .unwrap_or(0);
        let ty_max = y_grid
            .track_with_loc(TrackLocator::EndsAfter, rect.top())
            .try_into()
            .unwrap_or(0);

        PosSpan {
            layer: self.abs_layer(layer),
            tx_min,
            tx_max,
            ty_min,
            ty_max,
        }
    }

    pub(crate) fn move_to_track_index(&self, coord: i64, side: Side) -> i64 {
        let grid_tracks = match side.coord_dir() {
            Dir::Horiz => &self.grid_vtracks,
            Dir::Vert => &self.grid_htracks,
        };
        match side.sign() {
            Sign::Pos => grid_tracks.track_with_loc(TrackLocator::EndsAfter, coord),
            Sign::Neg => grid_tracks.track_with_loc(TrackLocator::StartsBefore, coord),
        }
    }

    pub(crate) fn off_grid_bus_out_span(&self, bus: &OffGridBusTranslation, i: i64) -> Span {
        let tracks = &self.track_info(bus.layer).tracks;
        let center_track = tracks.track_at(bus.output_span().center());
        tracks.index(center_track - (bus.n / 2 - 1 - i) * bus.output_pitch + bus.shift)
    }

    pub(crate) fn expand_to_grid_inner(
        &self,
        rect: Rect,
        strategy: ExpandToGridStrategy,
        x_grid: &UniformTracks,
        y_grid: &UniformTracks,
    ) -> Rect {
        match strategy {
            ExpandToGridStrategy::All => {
                let tx_min = x_grid.track_with_loc(TrackLocator::StartsBefore, rect.left());
                let tx_max = x_grid.track_with_loc(TrackLocator::EndsAfter, rect.right());
                let ty_min = y_grid.track_with_loc(TrackLocator::StartsBefore, rect.bottom());
                let ty_max = y_grid.track_with_loc(TrackLocator::EndsAfter, rect.top());
                Rect::from_spans(
                    x_grid.index(tx_min).union(x_grid.index(tx_max)),
                    y_grid.index(ty_min).union(y_grid.index(ty_max)),
                )
            }
            _ => {
                // Tracks that may straddle both sides of the source rectangle.
                let tx_min = x_grid.track_with_loc(TrackLocator::EndsAfter, rect.left());
                let tx_max = x_grid.track_with_loc(TrackLocator::StartsBefore, rect.right());
                let ty_min = y_grid.track_with_loc(TrackLocator::EndsAfter, rect.bottom());
                let ty_max = y_grid.track_with_loc(TrackLocator::StartsBefore, rect.top());

                // Tracks that do not straddle both sides of the source rectangle.
                let track_right = x_grid.track_with_loc(TrackLocator::StartsAfter, rect.left());
                let track_left = x_grid.track_with_loc(TrackLocator::EndsBefore, rect.right());
                let track_top = y_grid.track_with_loc(TrackLocator::StartsAfter, rect.bottom());
                let track_bot = y_grid.track_with_loc(TrackLocator::EndsBefore, rect.top());

                // Only case where non-straddling rectangles are better is when a track is fully
                // enclosed by the source rectangle.
                let (tx_min, tx_max) = if track_right <= track_left {
                    (track_right, track_left)
                } else {
                    (tx_min, tx_max)
                };
                let (ty_min, ty_max) = if track_top <= track_bot {
                    (track_top, track_bot)
                } else {
                    (ty_min, ty_max)
                };

                // In the case where a track is fully enclosed, potentially need to swap the two so
                // that `track_left` and `track_bot` actually corresponds to the
                // leftmost/bottom-most track, respectively.
                let (track_right, track_left) = if track_right < track_left {
                    (track_left, track_right)
                } else {
                    (track_right, track_left)
                };

                let (track_top, track_bot) = if track_top < track_bot {
                    (track_bot, track_top)
                } else {
                    (track_top, track_bot)
                };

                let mut tracks = Sides::new(
                    y_grid.index(ty_max),
                    x_grid.index(tx_max),
                    y_grid.index(ty_min),
                    x_grid.index(tx_min),
                );

                let tracks_constrained = Sides::new(
                    y_grid.index(track_top),
                    x_grid.index(track_right),
                    y_grid.index(track_bot),
                    x_grid.index(track_left),
                );

                let (first_dirs, second_dirs) = match strategy {
                    ExpandToGridStrategy::All => unreachable!(),
                    ExpandToGridStrategy::Side(first_dir) => {
                        tracks[first_dir] = tracks_constrained[first_dir];
                        (
                            vec![first_dir],
                            vec![Side::with_dir(first_dir.edge_dir()).collect()],
                        )
                    }
                    ExpandToGridStrategy::Corner(corner) => {
                        tracks[corner.side(Dir::Horiz)] =
                            tracks_constrained[corner.side(Dir::Horiz)];
                        tracks[corner.side(Dir::Vert)] = tracks_constrained[corner.side(Dir::Vert)];
                        (
                            vec![corner.side(Dir::Horiz)],
                            vec![vec![corner.side(Dir::Vert)]],
                        )
                    }
                    ExpandToGridStrategy::Minimum => {
                        let horiz_dirs: Vec<Side> = Side::with_dir(Dir::Horiz).collect();
                        let vert_dirs: Vec<Side> = Side::with_dir(Dir::Vert).collect();
                        (
                            vec![Side::Top, Side::Bot, Side::Left, Side::Right],
                            vec![horiz_dirs.clone(), horiz_dirs, vert_dirs.clone(), vert_dirs],
                        )
                    }
                };
                let mut best_rect = None;
                let mut best_area = i64::MAX;
                for (first_dir, second_dirs) in first_dirs.iter().zip(second_dirs.iter()) {
                    for second_dir in second_dirs {
                        let rect = Rect::span_builder()
                            .with(
                                first_dir.coord_dir(),
                                tracks[*first_dir].union(rect.span(first_dir.coord_dir())),
                            )
                            .with(
                                second_dir.coord_dir(),
                                tracks[*second_dir].union(rect.span(second_dir.coord_dir())),
                            )
                            .build();
                        if rect.area() < best_area {
                            best_rect = Some(rect);
                            best_area = rect.area();
                        }
                    }
                }

                best_rect.unwrap()
            }
        }
    }

    /// Expands the provided rectangle to align with the routing grid.
    pub fn expand_to_layer_grid(
        &self,
        rect: Rect,
        layer: LayerKey,
        strategy: ExpandToGridStrategy,
    ) -> Rect {
        let track_info = self.track_info(layer);
        let (x_grid, y_grid) = match track_info.dir {
            Dir::Horiz => (&self.grid_vtracks, &track_info.tracks),
            Dir::Vert => (&track_info.tracks, &self.grid_htracks),
        };
        self.expand_to_grid_inner(rect, strategy, x_grid, y_grid)
    }

    /// Expands the provided rectangle to align with the routing grid.
    pub fn expand_to_grid(&self, rect: Rect, strategy: ExpandToGridStrategy) -> Rect {
        self.expand_to_grid_inner(rect, strategy, &self.grid_vtracks, &self.grid_htracks)
    }

    /// Registers an off-grid bus translation with the router.
    ///
    /// Blocks off grid spaces corresponding to the translation geometry
    /// and returns an `OnGridBus` that can be used for routing.
    pub fn register_off_grid_bus_translation(
        &mut self,
        ctx: &mut LayoutCtx,
        bus: OffGridBusTranslation,
    ) -> crate::error::Result<OnGridBus> {
        let dir = bus.output.norm_dir();

        let start = bus.output.coord();
        let sign = bus.output.side().sign();

        let in_tracks = UniformTracks::builder()
            .line(bus.line)
            .space(bus.space)
            .start(bus.start)
            .sign(Sign::Pos)
            .build()
            .unwrap();

        let mut ports = Vec::new();

        match bus.strategy {
            OffGridBusTranslationStrategy::Parallel => {
                let perp_tracks = UniformTracks::builder()
                    .line(bus.line)
                    .space(bus.space)
                    .start(start)
                    .sign(sign)
                    .build()
                    .unwrap();

                let mut up = 0;
                for i in 0..bus.n {
                    let in_span = in_tracks.index(i);
                    let out_span = self.off_grid_bus_out_span(&bus, i);

                    if in_span.start() < out_span.start() {
                        up += 1;
                    }
                }

                let max_perp_index = std::cmp::max(up, bus.n - up) - 1;
                let max_perp_track = perp_tracks.index(max_perp_index);
                let grid_tracks = self.grid_track(!dir);
                let last_perp_grid_track = grid_tracks
                    .index(self.move_to_track_index(max_perp_track.point(sign), bus.output.side()));

                let mut down = 0;
                let mut rects = Vec::new();
                for i in 0..bus.n {
                    let in_span = in_tracks.index(i);
                    let out_span = self.off_grid_bus_out_span(&bus, i);
                    let perp_span = out_span.union(in_span);

                    let perp_index = if in_span.start() < out_span.start() {
                        up -= 1;
                        if in_span.stop() > out_span.stop() {
                            down += 1;
                        }
                        up
                    } else {
                        incr(&mut down)
                    };

                    let perp_track = perp_tracks.index(perp_index);

                    let rect = Rect::span_builder()
                        .with(!dir, perp_span)
                        .with(dir, perp_track)
                        .build();
                    rects.push(rect);

                    let rect = Rect::span_builder()
                        .with(dir, Span::new(start, perp_track.point(sign)))
                        .with(!dir, in_span)
                        .build();
                    rects.push(rect);

                    let rect = Rect::span_builder()
                        .with(dir, perp_track.union(last_perp_grid_track))
                        .with(!dir, out_span)
                        .build();
                    rects.push(rect);
                    ports.push(rect);
                }

                self.block(
                    bus.layer,
                    rects
                        .iter()
                        .map(|rect| rect.bbox())
                        .reduce(|acc, bbox| acc.union(bbox))
                        .unwrap_or(Bbox::empty())
                        .into_rect(),
                );

                for rect in rects.iter() {
                    self.group.add_rect(bus.layer, *rect);
                }
            }
            OffGridBusTranslationStrategy::Perpendicular(layer) => {
                let output_sign = bus.output.side().sign();
                let parallel_grid = self.grid_track(bus.output.edge_dir());
                let output_edge = parallel_grid.index(parallel_grid.track_with_loc(
                    match output_sign {
                        Sign::Pos => TrackLocator::EndsAfter,
                        Sign::Neg => TrackLocator::StartsBefore,
                    },
                    bus.output.coord(),
                ));
                let out_tracks = &self.track_info(layer).tracks;
                let center_track = out_tracks.track_at(bus.output.span().center());
                let mut vias = Vec::new();
                for i in 0..bus.n {
                    let in_span = in_tracks.index(i);
                    let out_span = out_tracks
                        .index(center_track - (bus.n / 2 - 1 - i) * bus.output_pitch + bus.shift);
                    let rect = Rect::span_builder()
                        .with(bus.output.edge_dir(), out_span)
                        .with(bus.output.norm_dir(), in_span.union(output_edge))
                        .build();
                    ports.push(rect);
                    let bus_layer_idx = self.layer_idx(bus.layer);
                    let out_layer_idx = self.layer_idx(layer);

                    let src = Rect::span_builder()
                        .with(bus.output.edge_dir(), out_span)
                        .with(bus.output.norm_dir(), in_span)
                        .build();
                    let src_expanded = Rect::span_builder()
                        .with(
                            bus.output.edge_dir(),
                            out_span.expand_all(out_span.length() * 10),
                        )
                        .with(bus.output.norm_dir(), in_span)
                        .build();
                    // FIXME: `top_src` and `bot_src` are a hack to make vias prefer to follow the bus
                    // direction. Can make this an explicit parameter.
                    let (top, bot, top_src, bot_src) = if bus_layer_idx > out_layer_idx {
                        (bus_layer_idx, out_layer_idx, src_expanded, src)
                    } else {
                        (out_layer_idx, bus_layer_idx, src, src_expanded)
                    };

                    for j in bot..top {
                        vias.push(
                            ctx.instantiate::<Via>(
                                &ViaParams::builder()
                                    .layers(self.layers[j].layer, self.layers[j + 1].layer)
                                    .geometry(
                                        if j == bot { bot_src } else { src },
                                        if j == top - 1 { top_src } else { src },
                                    )
                                    .build(),
                            )?,
                        );
                    }
                }

                for via in vias {
                    self.group.add_group(via.draw()?);
                }

                for port in ports.iter() {
                    self.block(layer, *port);
                    self.group.add_rect(layer, *port);
                }
            }
        }

        Ok(OnGridBus { ports })
    }

    /// Registers a jog to the grid and returns the new grid-aligned port.
    ///
    /// Creates a jog to connect the provided geometry to a point on the grid.
    pub fn register_jog_to_grid(&mut self, jog_to_grid: JogToGrid) -> Rect {
        let track_info = self.track_info(jog_to_grid.dst_layer);
        let (x_grid, y_grid) = match track_info.dir {
            Dir::Horiz => (&self.grid_vtracks, &track_info.tracks),
            Dir::Vert => (&track_info.tracks, &self.grid_htracks),
        };
        let track_right = x_grid.track_with_loc(TrackLocator::StartsAfter, jog_to_grid.rect.left());
        let track_left = x_grid.track_with_loc(TrackLocator::EndsBefore, jog_to_grid.rect.right());
        let track_top = y_grid.track_with_loc(TrackLocator::StartsAfter, jog_to_grid.rect.bottom());
        let track_bot = y_grid.track_with_loc(TrackLocator::EndsBefore, jog_to_grid.rect.top());

        let (track_right, track_left) = if track_right < track_left {
            (track_left, track_right)
        } else {
            (track_right, track_left)
        };
        let (track_top, track_bot) = if track_top < track_bot {
            (track_bot, track_top)
        } else {
            (track_top, track_bot)
        };

        let mut tracks = Sides::new(track_top, track_right, track_bot, track_left);

        let (first_dirs, second_dirs) = if let Some(first_dir) = jog_to_grid.first_dir {
            tracks[first_dir] += jog_to_grid.extend_first;
            (
                vec![first_dir],
                if let Some(second_dir) = jog_to_grid.second_dir {
                    tracks[second_dir] += jog_to_grid.extend_second;
                    vec![vec![second_dir]]
                } else {
                    vec![Side::with_dir(first_dir.edge_dir()).collect()]
                },
            )
        } else {
            let horiz_dirs: Vec<Side> = Side::with_dir(Dir::Horiz).collect();
            let vert_dirs: Vec<Side> = Side::with_dir(Dir::Vert).collect();
            (
                vec![Side::Top, Side::Bot, Side::Left, Side::Right],
                vec![horiz_dirs.clone(), horiz_dirs, vert_dirs.clone(), vert_dirs],
            )
        };

        let tracks = tracks.map(|side, track_index| match side.coord_dir() {
            Dir::Horiz => x_grid.index(track_index),
            Dir::Vert => y_grid.index(track_index),
        });
        let mut best_group = None;
        let mut best_rect = None;
        let mut best_len = i64::MAX;
        for (first_dir, second_dirs) in first_dirs.iter().zip(second_dirs.iter()) {
            for second_dir in second_dirs {
                let target = Rect::span_builder()
                    .with(first_dir.coord_dir(), tracks[*first_dir])
                    .with(second_dir.coord_dir(), tracks[*second_dir])
                    .build();

                let needs_first_dir = !jog_to_grid
                    .rect
                    .span(first_dir.coord_dir())
                    .contains(target.span(first_dir.coord_dir()));
                let needs_second_dir = !jog_to_grid
                    .rect
                    .span(second_dir.coord_dir())
                    .contains(target.span(second_dir.coord_dir()));

                let mut group = Group::new();

                let len = match (needs_first_dir, needs_second_dir) {
                    (true, true) => {
                        let src_span = Span::with_point_and_length(
                            second_dir.sign(),
                            jog_to_grid.rect.side(*second_dir),
                            jog_to_grid.width,
                        );
                        let r1 = Rect::span_builder()
                            .with(first_dir.edge_dir(), src_span)
                            .with(
                                first_dir.coord_dir(),
                                target
                                    .span(first_dir.coord_dir())
                                    .add_point(jog_to_grid.rect.side(*first_dir)),
                            )
                            .build();

                        let r2 = Rect::span_builder()
                            .with(second_dir.edge_dir(), target.span(second_dir.edge_dir()))
                            .with(
                                second_dir.coord_dir(),
                                target
                                    .span(second_dir.coord_dir())
                                    .add_point(src_span.point(!second_dir.sign())),
                            )
                            .build();
                        group.add_rect(jog_to_grid.layer, r1);
                        group.add_rect(jog_to_grid.layer, r2);
                        r1.length(first_dir.coord_dir()) + r2.length(second_dir.coord_dir())
                    }
                    (true, false) => {
                        let r1 = Rect::span_builder()
                            .with(first_dir.edge_dir(), target.span(first_dir.edge_dir()))
                            .with(
                                first_dir.coord_dir(),
                                target
                                    .span(first_dir.coord_dir())
                                    .add_point(jog_to_grid.rect.side(*first_dir)),
                            )
                            .build();
                        group.add_rect(jog_to_grid.layer, r1);

                        r1.length(first_dir.coord_dir())
                    }
                    (false, true) => {
                        let r2 = Rect::span_builder()
                            .with(second_dir.edge_dir(), target.span(second_dir.edge_dir()))
                            .with(
                                second_dir.coord_dir(),
                                target
                                    .span(second_dir.coord_dir())
                                    .add_point(jog_to_grid.rect.side(*second_dir)),
                            )
                            .build();
                        group.add_rect(jog_to_grid.layer, r2);

                        r2.length(first_dir.coord_dir())
                    }
                    (false, false) => {
                        return target;
                    }
                };

                if len < best_len {
                    best_group = Some(group);
                    best_rect = Some(target);
                    best_len = len;
                }
            }
        }

        self.group.add_group(best_group.unwrap());

        let rect = best_rect.unwrap();
        self.group.add_rect(jog_to_grid.layer, rect);

        rect
    }
}

fn incr(i: &mut i64) -> i64 {
    let tmp = *i;
    *i += 1;
    tmp
}
