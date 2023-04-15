//! Abstract routing utilities.
//!
//! These APIs deal with abstract routing notions (tracks, layers, etc.)
//! rather than raw layout (rectangles, GDS layers, etc.).

use std::collections::{HashMap, HashSet};

use grid::Grid;
use itertools::Itertools;
use subgeom::Dir;

use super::error::*;

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub struct Net(usize);

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub struct Layer(pub usize);

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub enum HasVia {
    Yes,
    No,
}

impl Layer {
    pub fn id(&self) -> usize {
        self.0
    }

    pub fn above(&self) -> Self {
        Self(self.0 + 1)
    }
    pub fn below(&self) -> Option<Self> {
        match self.0 {
            0 => None,
            n => Some(Self(n - 1)),
        }
    }
}

impl From<bool> for HasVia {
    fn from(value: bool) -> Self {
        if value {
            Self::Yes
        } else {
            Self::No
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub enum State {
    Occupied { net: Net, via: HasVia },
    Blocked { net: Option<Net> },
    Empty,
}

impl State {
    pub fn blocked() -> State {
        State::Blocked { net: None }
    }
    pub fn is_occupied(&self) -> bool {
        matches!(self, State::Occupied { .. })
    }
    pub fn is_occupied_by(&self, net: Net) -> bool {
        if let State::Occupied { net: other, via: _ } = self {
            net == *other
        } else {
            false
        }
    }
    pub fn is_empty(&self) -> bool {
        matches!(self, State::Empty)
    }
    pub fn is_blocked(&self) -> bool {
        matches!(self, State::Blocked { .. })
    }
    pub fn is_blocked_by(&self, net: Net) -> bool {
        if let State::Blocked { net: other } = self {
            if let Some(other) = other {
                return net == *other;
            }
        }
        false
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub enum PosAction {
    // Move by 1 in the positive y direction.
    Up,
    // Move by 1 in the negative y direction.
    Down,
    // Move by 1 in the positive x direction.
    Right,
    // Move by 1 in the negative x direction.
    Left,
    // Move by 1 in the positive z direction.
    ZUp,
    // Move by 1 in the negative z direction.
    ZDown,
}

impl PosAction {
    fn all() -> impl Iterator<Item = PosAction> {
        use PosAction::*;
        [Up, Down, Right, Left, ZUp, ZDown].iter().copied()
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub struct Pos {
    /// The current layer.
    pub(crate) layer: Layer,
    /// X-coordinate. Indexes the vertical-going tracks.
    pub(crate) tx: usize,
    /// Y-coordinate. Indexes the horizontal-going tracks.
    pub(crate) ty: usize,
    /// Whether the current position is continuous with the previous position.
    ///
    /// For example, set to true if the start of a segment created after jumping through
    /// and existing route for a given net.
    jump: bool,
}

impl Pos {
    fn next(&self, action: PosAction) -> Pos {
        let pos = match action {
            PosAction::Up => Pos {
                ty: self.ty + 1,
                ..*self
            },
            PosAction::Down => Pos {
                ty: self.ty - 1,
                ..*self
            },
            PosAction::Right => Pos {
                tx: self.tx + 1,
                ..*self
            },
            PosAction::Left => Pos {
                tx: self.tx - 1,
                ..*self
            },
            PosAction::ZUp => Pos {
                layer: Layer(self.layer.0 + 1),
                ..*self
            },
            PosAction::ZDown => Pos {
                layer: Layer(self.layer.0 - 1),
                ..*self
            },
        };
        pos.no_jump()
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub struct Segment {
    pub(crate) track_id: usize,
    pub(crate) span: PosSpan,
    /// Indicates if this segment is on the lower or left boundary of the routing region.
    pub(crate) lower_boundary: bool,
    /// Indicates if this segment is on the upper or right boundary of the routing region.
    pub(crate) upper_boundary: bool,
}

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub struct PosSpan {
    /// The current layer.
    pub(crate) layer: Layer,
    pub(crate) tx_min: usize,
    pub(crate) tx_max: usize,
    pub(crate) ty_min: usize,
    pub(crate) ty_max: usize,
}

#[derive(Default, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
pub struct PosSpanBuilder {
    /// The current layer.
    pub(crate) layer: Option<Layer>,
    pub(crate) tx_min: Option<usize>,
    pub(crate) tx_max: Option<usize>,
    pub(crate) ty_min: Option<usize>,
    pub(crate) ty_max: Option<usize>,
}

impl PosSpanBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_layer(layer: Layer) -> Self {
        Self {
            layer: Some(layer),
            ..Default::default()
        }
    }

    pub fn with(&mut self, dir: Dir, min: usize, max: usize) -> &mut Self {
        match dir {
            Dir::Horiz => {
                self.tx_min = Some(min);
                self.tx_max = Some(max);
            }
            Dir::Vert => {
                self.ty_min = Some(min);
                self.ty_max = Some(max);
            }
        }
        self
    }

    pub fn build(&mut self) -> PosSpan {
        PosSpan {
            layer: self.layer.unwrap(),
            tx_min: self.tx_min.unwrap(),
            tx_max: self.tx_max.unwrap(),
            ty_min: self.ty_min.unwrap(),
            ty_max: self.ty_max.unwrap(),
        }
    }
}

impl From<Pos> for PosSpan {
    fn from(value: Pos) -> Self {
        Self {
            layer: value.layer,
            tx_min: value.tx,
            tx_max: value.tx,
            ty_min: value.ty,
            ty_max: value.ty,
        }
    }
}

impl Pos {
    pub fn new(layer: Layer, tx: usize, ty: usize) -> Self {
        Self {
            layer,
            tx,
            ty,
            jump: false,
        }
    }

    // Returns a new `Pos` that is the same as the provided [`Pos`] but is marked as the
    // beginning of a new segment of the path.
    pub fn mark_jump(&self) -> Self {
        let mut new_pos = *self;
        new_pos.jump = true;
        new_pos
    }

    // Returns a new `Pos` that is the same as the provided [`Pos`] but without a jump mark.
    pub fn no_jump(&self) -> Self {
        let mut new_pos = *self;
        new_pos.jump = false;
        new_pos
    }

    // Returns whether this [`Pos`] is the beginning of a new segment of the path.
    pub fn is_jump(&self) -> bool {
        self.jump
    }

    pub fn coord(&self, dir: Dir) -> usize {
        match dir {
            Dir::Horiz => self.tx,
            Dir::Vert => self.ty,
        }
    }
}

impl PosSpan {
    pub fn contains(&self, other: Pos) -> bool {
        self.layer == other.layer
            && (other.tx >= self.tx_min && other.tx <= self.tx_max)
            && (other.ty >= self.ty_min && other.ty <= self.ty_max)
    }

    pub fn span(&self, dir: Dir) -> (usize, usize) {
        match dir {
            Dir::Horiz => (self.tx_min, self.tx_max),
            Dir::Vert => (self.ty_min, self.ty_max),
        }
    }
}

pub struct AbstractLayerInfo {
    pub(crate) grid_space: usize,
    pub(crate) dir: Dir,
    pub(crate) grid: Grid<State>,
}

impl AbstractLayerInfo {
    pub fn num_tracks(&self) -> usize {
        // This seems backwards, but it is correct.
        // The number of horizontal tracks = the number of y coordinates,
        // which are the second indices into the grid and are therefore "cols".
        match self.dir {
            Dir::Horiz => self.grid.cols() / self.grid_space,
            Dir::Vert => self.grid.rows() / self.grid_space,
        }
    }

    pub fn num_parallel_grid_points(&self) -> usize {
        match self.dir {
            Dir::Horiz => self.grid.rows(),
            Dir::Vert => self.grid.cols(),
        }
    }

    pub fn iter_track(&self, id: usize) -> Box<dyn Iterator<Item = &State> + '_> {
        match self.dir {
            Dir::Horiz => Box::new(self.grid.iter_col(id)),
            Dir::Vert => Box::new(self.grid.iter_row(id)),
        }
    }
}

pub struct GreedyAbstractRouter {
    curr_net: Net,
    net_to_pos: HashMap<Net, HashSet<Pos>>,
    layers: Vec<AbstractLayerInfo>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct AbstractLayerConfig {
    pub grid_space: usize,
    pub dir: Dir,
}

pub type AbstractRoute = Vec<Pos>;

#[derive(Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Copy, Clone)]
enum Node {
    Span(PosSpan),
    Pos(Pos),
}

impl Node {
    pub fn unwrap_pos(self) -> Pos {
        match self {
            Self::Pos(pos) => pos,
            _ => panic!("expected `Node` to be a `Pos` variant"),
        }
    }
}

impl GreedyAbstractRouter {
    pub fn new(
        layers: impl IntoIterator<Item = AbstractLayerConfig>,
        tx: usize,
        ty: usize,
    ) -> Self {
        Self {
            curr_net: Net(0),
            net_to_pos: HashMap::new(),
            layers: layers
                .into_iter()
                .map(|cfg| AbstractLayerInfo {
                    grid_space: cfg.grid_space,
                    dir: cfg.dir,
                    grid: Grid::init(tx, ty, State::Empty),
                })
                .collect(),
        }
    }

    // Accepts `dst` so that we can route to destinations that are blocked.
    // Unfortunately, causes illegal routes to be accepted.
    pub fn route_with_net(
        &mut self,
        src: PosSpan,
        dst: PosSpan,
        net: Net,
    ) -> Result<AbstractRoute> {
        let grid = self.grid(src.layer);
        let (nx, ny) = (grid.rows(), grid.cols());
        assert!(src.tx_max >= src.tx_min);
        assert!(src.ty_max >= src.ty_min);
        assert!(src.tx_max < nx);
        assert!(src.ty_max < ny);
        assert!(dst.tx_max < nx);
        assert!(dst.ty_max < ny);
        assert!(dst.tx_min <= dst.tx_max);
        assert!(dst.ty_min <= dst.ty_max);
        let successors = |pos: &Node| self.successors(*pos, dst, net);
        let success = |pos: &Node| match pos {
            Node::Span(_) => false,
            Node::Pos(p) => dst.contains(*p),
        };
        let nodes = pathfinding::directed::bfs::bfs(&Node::Span(src), successors, success)
            .ok_or(Error::NoRouteFound)?;

        for i in 1..nodes.len() {
            let node = &nodes[i].unwrap_pos();
            let has_via = {
                i < nodes.len() - 1
                    && (nodes[i + 1].unwrap_pos().layer != node.layer)
                    && !nodes[i + 1].unwrap_pos().is_jump()
            };
            let state = self.grid_mut(node.layer).get_mut(node.tx, node.ty).unwrap();
            *state = State::Occupied {
                net,
                via: has_via.into(),
            };
            self.net_to_pos
                .entry(net)
                .or_insert(HashSet::new())
                .insert(*node);
        }

        Ok(nodes[1..].iter().map(|n| n.unwrap_pos()).collect_vec())
    }

    pub fn route(&mut self, src: PosSpan, dst: PosSpan) -> Result<AbstractRoute> {
        let net = self.get_unused_net();
        self.route_with_net(src, dst, net)
    }

    pub fn get_unused_net(&mut self) -> Net {
        for net in self.net_to_pos.keys() {
            if &self.curr_net == net {
                self.curr_net.0 += 1;
            }
        }
        self.net_to_pos.insert(self.curr_net, HashSet::new());
        self.curr_net
    }

    fn block_inner(&mut self, pos: Pos, net: Option<Net>) {
        *self.grid_mut(pos.layer).get_mut(pos.tx, pos.ty).unwrap() = State::Blocked { net };
    }
    fn block_span_inner(&mut self, layer: Layer, span: PosSpan, net: Option<Net>) {
        for tx in span.tx_min..=span.tx_max {
            for ty in span.ty_min..=span.ty_max {
                self.grid_mut(layer)[tx][ty] = State::Blocked { net };
            }
        }
    }

    pub fn block(&mut self, pos: Pos) {
        self.block_inner(pos, None);
    }
    pub fn block_span(&mut self, layer: Layer, span: PosSpan) {
        self.block_span_inner(layer, span, None);
    }

    pub fn block_for_net(&mut self, pos: Pos, net: Net) {
        self.block_inner(pos, Some(net));
    }
    pub fn block_span_for_net(&mut self, layer: Layer, span: PosSpan, net: Net) {
        self.block_span_inner(layer, span, Some(net));
    }

    pub fn occupy(&mut self, pos: Pos, net: Net) {
        *self.grid_mut(pos.layer).get_mut(pos.tx, pos.ty).unwrap() = State::Occupied {
            net,
            via: HasVia::No,
        };
    }
    pub fn occupy_span(&mut self, layer: Layer, span: PosSpan, net: Net) {
        for tx in span.tx_min..=span.tx_max {
            for ty in span.ty_min..=span.ty_max {
                self.grid_mut(layer)[tx][ty] = State::Occupied {
                    net,
                    via: HasVia::No,
                };
            }
        }
    }

    pub fn segments(&self, layer: Layer) -> Vec<Segment> {
        let mut out = Vec::new();
        let info = self.layer_info(layer);
        let p_round = self.parallel_grid_spacing(layer);

        let max_grid = round_down(info.num_parallel_grid_points() - 1, p_round);
        // 0 always rounds up to 0, so this doesn't do anything.
        // It's here just in case we change behavior later.
        let min_grid = round_up(0, p_round);

        let dir = info.dir;

        for i in 0..info.num_tracks() {
            let tid = i * info.grid_space;
            for (empty, run) in &info
                .iter_track(tid)
                .enumerate()
                .group_by(|(_, s)| s.is_empty())
            {
                if !empty {
                    continue;
                }
                let (p_min, p_max) = run.map(|(x, _)| x).minmax().into_option().unwrap();
                let p_min_r = round_up(p_min, p_round);
                let p_max_r = round_down(p_max, p_round);
                if p_min_r >= p_max_r {
                    continue;
                }
                let span = PosSpanBuilder::with_layer(layer)
                    .with(dir, p_min_r, p_max_r)
                    .with(!dir, tid, tid)
                    .build();

                let lower_boundary = p_min_r <= min_grid;
                let upper_boundary = p_max_r >= max_grid;

                out.push(Segment {
                    track_id: i,
                    span,
                    lower_boundary,
                    upper_boundary,
                });
            }
        }
        out
    }

    fn pos_next(&self, pos: Pos, dst_span: PosSpan, net: Net) -> Vec<Node> {
        let mut candidates = Vec::new();
        for action in PosAction::all() {
            if self.is_valid_action(pos, action) {
                candidates.push(pos.next(action));
            }
        }

        let state = self.grid(pos.layer).get(pos.tx, pos.ty).unwrap();

        let mut filtered_candidates = candidates
            .into_iter()
            .filter(|n| {
                let val = self.grid(n.layer).get(n.tx, n.ty);
                val.map(|s| s.is_empty() || s.is_occupied_by(net) || s.is_blocked_by(net))
                    .unwrap_or_default()
                    || dst_span.contains(*n)
            })
            .map(Node::Pos)
            .collect_vec();

        if state.is_occupied_by(net) {
            if let Some(positions) = self.net_to_pos.get(&net) {
                for n in positions {
                    filtered_candidates.push(Node::Pos(n.mark_jump()));
                }
            }
        }

        filtered_candidates
    }

    fn is_valid_action(&self, pos: Pos, action: PosAction) -> bool {
        let layer_info = self.layer_info(pos.layer);
        let grid = &layer_info.grid;

        // Check boundary conditions.
        if !match action {
            PosAction::Up => pos.ty < grid.cols() - 1,
            PosAction::Down => pos.ty > 0,
            PosAction::Right => pos.tx < grid.rows() - 1,
            PosAction::Left => pos.tx > 0,
            PosAction::ZUp => pos.layer.0 < self.layers.len() - 1,
            PosAction::ZDown => pos.layer.0 > 0,
        } {
            return false;
        }

        // Check layer direction matches up with action.
        match layer_info.dir {
            Dir::Horiz => {
                if action == PosAction::Up || action == PosAction::Down {
                    return false;
                }
            }
            Dir::Vert => {
                if action == PosAction::Right || action == PosAction::Left {
                    return false;
                }
            }
        }

        // Ensure that next position is on the grid for its corresponding layer.
        let next_pos = pos.next(action);
        let next_layer_info = self.layer_info(next_pos.layer);
        next_pos.coord(!next_layer_info.dir) % next_layer_info.grid_space == 0
    }

    fn span_next(&self, span: PosSpan) -> Vec<Node> {
        let mut next =
            Vec::with_capacity((span.tx_max - span.tx_min + 1) * (span.ty_max - span.ty_min + 1));
        for tx in span.tx_min..=span.tx_max {
            for ty in span.ty_min..=span.ty_max {
                let pos = Pos::new(span.layer, tx, ty);
                next.push(Node::Pos(pos));
            }
        }
        next
    }

    fn successors(&self, node: Node, dst_span: PosSpan, net: Net) -> Vec<Node> {
        match node {
            Node::Pos(pos) => self.pos_next(pos, dst_span, net),
            Node::Span(span) => self.span_next(span),
        }
    }

    fn grid(&self, layer: Layer) -> &Grid<State> {
        &self.layers[layer.0].grid
    }

    fn grid_mut(&mut self, layer: Layer) -> &mut Grid<State> {
        &mut self.layers[layer.0].grid
    }

    pub(crate) fn dir(&self, layer: Layer) -> Dir {
        self.layers[layer.0].dir
    }

    pub(crate) fn layer_info(&self, layer: Layer) -> &AbstractLayerInfo {
        &self.layers[layer.0]
    }

    pub(crate) fn parallel_grid_spacing(&self, layer: Layer) -> usize {
        let above = if layer.0 < self.layers.len() - 1 {
            self.layer_info(layer.above()).grid_space
        } else {
            1
        };
        let below = if let Some(below) = layer.below() {
            self.layer_info(below).grid_space
        } else {
            1
        };
        std::cmp::max(above, below)
    }
}

fn round_down(x: usize, grid: usize) -> usize {
    (x / grid) * grid
}

fn round_up(x: usize, grid: usize) -> usize {
    ((x + grid - 1) / grid) * grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_down() {
        assert_eq!(round_down(331, 200), 200);
        assert_eq!(round_down(400, 331), 331);
        assert_eq!(round_down(900, 300), 900);
        assert_eq!(round_down(901, 300), 900);
        assert_eq!(round_down(900, 299), 897);
    }

    #[test]
    fn test_basic_greedy_two_layer_abstract_routing() {
        let mut router = GreedyAbstractRouter::new(
            vec![
                AbstractLayerConfig {
                    grid_space: 1,
                    dir: Dir::Horiz,
                },
                AbstractLayerConfig {
                    grid_space: 1,
                    dir: Dir::Vert,
                },
            ],
            1_000,
            1_000,
        );

        router
            .route(
                Pos::new(Layer(0), 0, 0).into(),
                Pos::new(Layer(1), 4, 4).into(),
            )
            .expect("failed to route");
    }
}
