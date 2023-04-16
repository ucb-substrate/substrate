use std::collections::HashMap;

use itertools::Itertools;
use subgeom::bbox::BoundBox;
use subgeom::{Dir, Rect, Sign};

use self::abs::{GreedyAbstractRouter, Net};
use super::tracks::UniformTracks;
use crate::index::IndexOwned;
use crate::layout::context::LayoutCtx;
use crate::layout::elements::via::{Via, ViaParams};
use crate::layout::group::Group;
use crate::layout::layers::LayerKey;
use crate::layout::routing::auto::abs::{AbstractLayerConfig, AbstractRoute};
use crate::layout::{Draw, DrawRef};

pub mod abs;
pub mod error;
pub mod grid;
pub mod straps;

#[allow(dead_code)]
pub struct TrackInfo {
    layer: LayerKey,
    tracks: UniformTracks,
    dir: Dir,
}

impl TrackInfo {
    pub fn tracks(&self) -> &UniformTracks {
        &self.tracks
    }
}

pub struct GreedyRouter {
    inner: GreedyAbstractRouter,
    area: Rect,
    layers: Vec<TrackInfo>,
    key_to_index: HashMap<LayerKey, usize>,
    grid_vtracks: UniformTracks,
    grid_htracks: UniformTracks,
    group: Group,
    net_map: HashMap<String, Net>,
}

pub struct GreedyRouterConfig {
    pub area: Rect,
    pub layers: Vec<LayerConfig>,
}

pub struct LayerConfig {
    pub line: i64,
    pub space: i64,
    pub dir: Dir,
    pub layer: LayerKey,
}

impl LayerConfig {
    fn pitch(&self) -> i64 {
        self.line + self.space
    }
}

impl GreedyRouter {
    pub fn with_config(config: GreedyRouterConfig) -> Self {
        assert!(!config.layers.is_empty());

        let layers = config
            .layers
            .iter()
            .map(|layer_cfg| TrackInfo {
                layer: layer_cfg.layer,
                tracks: UniformTracks {
                    line: layer_cfg.line,
                    space: layer_cfg.space,
                    sign: Sign::Pos,
                    start: config.area.span(!layer_cfg.dir).start() - layer_cfg.line / 2,
                },
                dir: layer_cfg.dir,
            })
            .collect();

        let key_to_index = HashMap::from_iter(
            config
                .layers
                .iter()
                .enumerate()
                .map(|(i, layer_cfg)| (layer_cfg.layer, i)),
        );

        let layer0 = &config.layers[0];

        let grid_vtracks = UniformTracks {
            line: layer0.line,
            space: layer0.space,
            sign: Sign::Pos,
            start: config.area.span(Dir::Horiz).start() - layer0.line / 2,
        };
        let grid_htracks = UniformTracks {
            line: layer0.line,
            space: layer0.space,
            sign: Sign::Pos,
            start: config.area.span(Dir::Vert).start() - layer0.line / 2,
        };
        let nx = grid_vtracks.track_at(config.area.right()) + 2;
        let ny = grid_htracks.track_at(config.area.top()) + 2;

        assert!(nx >= 0 && ny >= 0);

        let inner = GreedyAbstractRouter::new(
            config.layers.iter().enumerate().map(|(i, layer_cfg)| {
                assert_eq!(
                    layer_cfg.pitch() % layer0.pitch(),
                    0,
                    "layer pitch should be a multiple of the minimum pitch"
                );
                if i > 0 {
                    assert_ne!(
                        layer_cfg.dir,
                        config.layers[i - 1].dir,
                        "Layers should alternate directions"
                    );
                }
                AbstractLayerConfig {
                    grid_space: ((layer_cfg.line + layer_cfg.space) / (layer0.line + layer0.space))
                        as usize,
                    dir: layer_cfg.dir,
                }
            }),
            nx as usize,
            ny as usize,
        );

        Self {
            inner,
            area: config.area,
            layers,
            key_to_index,
            grid_vtracks,
            grid_htracks,
            group: Group::new(),
            net_map: HashMap::new(),
        }
    }

    pub fn get_net(&mut self, net: &str) -> Net {
        if let Some(net) = self.net_map.get(net) {
            *net
        } else {
            let new_net = self.inner.get_unused_net();
            self.net_map.insert(net.to_string(), new_net);
            new_net
        }
    }

    /// Generates a route between the provided geometries if one exists.
    pub fn route_with_net(
        &mut self,
        ctx: &mut LayoutCtx,
        src_layer: LayerKey,
        src: Rect,
        dst_layer: LayerKey,
        dst: Rect,
        net: &str,
    ) -> crate::error::Result<()> {
        let net = self.get_net(net);
        self.route_inner(ctx, src_layer, src, dst_layer, dst, net)
    }

    /// Generates a route between the provided geometries if one exists on the provided net.
    pub fn route(
        &mut self,
        ctx: &mut LayoutCtx,
        src_layer: LayerKey,
        src: Rect,
        dst_layer: LayerKey,
        dst: Rect,
    ) -> crate::error::Result<()> {
        let net = self.inner.get_unused_net();
        self.route_inner(ctx, src_layer, src, dst_layer, dst, net)
    }

    fn route_inner(
        &mut self,
        ctx: &mut LayoutCtx,
        src_layer: LayerKey,
        src: Rect,
        dst_layer: LayerKey,
        dst: Rect,
        net: Net,
    ) -> crate::error::Result<()> {
        // src and dst geometry must be contained within the routing area.
        assert!(self.area.bbox().intersection(src.bbox()).into_rect() == src);
        assert!(self.area.bbox().intersection(dst.bbox()).into_rect() == dst);
        assert!(self.key_to_index.contains_key(&src_layer));
        assert!(self.key_to_index.contains_key(&dst_layer));

        let src_span = self.shrink_to_pos_span(src_layer, src);
        let dst_span = self.shrink_to_pos_span(dst_layer, dst);

        let route = self.inner.route_with_net(src_span, dst_span, net)?;

        let mut counter = 0;
        while counter < route.len() {
            let mut subroute = vec![route[counter]];
            counter += 1;
            while counter < route.len() && !route[counter].is_jump() {
                subroute.push(route[counter]);
                counter += 1;
            }
            let runs: Vec<(abs::Layer, AbstractRoute)> = subroute
                .into_iter()
                .group_by(|n| n.layer)
                .into_iter()
                .map(|(layer, group)| (layer, group.into_iter().collect::<AbstractRoute>()))
                .collect();
            let mut rects = Vec::new();
            for (i, (layer, run)) in runs.iter().enumerate() {
                let layer = *layer;
                let dir = self.inner.dir(layer);
                let (first, last) = (run[0], run[run.len() - 1]);
                let tid = first.coord(!dir);
                let track = self.track_span(layer, tid);

                let first = if i > 0 && self.inner.dir(runs[i - 1].0) != dir {
                    self.track_span(runs[i - 1].0, first.coord(dir))
                } else {
                    self.grid_track(!dir).index(first.coord(dir))
                };

                let last = if i < runs.len() - 1 && self.inner.dir(runs[i + 1].0) != dir {
                    self.track_span(runs[i + 1].0, last.coord(dir))
                } else {
                    self.grid_track(!dir).index(last.coord(dir))
                };

                let rect = Rect::span_builder()
                    .with(dir, first.union(last))
                    .with(!dir, track)
                    .build();

                rects.push((layer, rect));
            }

            let mut prev = None;
            for (layer, rect) in rects {
                let layer_key = self.layer(layer);
                self.group.add_rect(layer_key, rect);

                if let Some((prev_layer, prev_rect)) = prev {
                    if prev_layer != layer {
                        let (bot, top) = if prev_layer < layer {
                            (prev_layer, layer)
                        } else {
                            (layer, prev_layer)
                        };
                        let viap = ViaParams::builder()
                            .layers(self.layer(bot), self.layer(top))
                            .geometry(prev_rect, rect)
                            .build();
                        let via = ctx.instantiate::<Via>(&viap)?;
                        self.group.add_instance(via);
                    }
                }
                prev = Some((layer, rect));
            }
        }

        Ok(())
    }

    pub fn segments(&self, layer: LayerKey) -> Vec<Segment> {
        let layer = self.abs_layer(layer);
        let spans = self.inner.segments(layer);
        let mut out = Vec::with_capacity(spans.len());
        let dir = self.inner.dir(layer);
        for abs::Segment {
            span,
            track_id,
            lower_boundary,
            upper_boundary,
        } in spans
        {
            let tid = span.span(!dir).0;
            let track = self.track_span(layer, tid);
            let start = self.grid_track(!dir).index(span.span(dir).0);
            let end = self.grid_track(!dir).index(span.span(dir).1);
            let xspan = start.union(end);
            out.push(Segment {
                track_id,
                rect: Rect::span_builder()
                    .with(dir, xspan)
                    .with(!dir, track)
                    .build(),
                lower_boundary,
                upper_boundary,
            });
        }
        out
    }

    fn layer_idx(&self, layer: LayerKey) -> usize {
        *self.key_to_index.get(&layer).unwrap()
    }

    pub fn track_info(&self, layer: LayerKey) -> &TrackInfo {
        &self.layers[self.layer_idx(layer)]
    }

    pub fn block(&mut self, layer: LayerKey, rect: Rect) {
        let span = self.expand_to_pos_span(layer, rect);
        self.inner.block_span(span);
    }

    pub fn block_with_shrink(&mut self, layer: LayerKey, rect: Rect) {
        let span = self.shrink_to_pos_span(layer, rect);
        self.inner.block_span(span);
    }

    pub fn occupy(&mut self, layer: LayerKey, rect: Rect, net: &str) -> crate::error::Result<()> {
        let net = self.get_net(net);
        let span = self.expand_to_pos_span(layer, rect);
        self.inner.block_span_for_net(span, net);
        let span = self.shrink_to_pos_span(layer, rect);
        self.inner.occupy_span(span, net)?;
        Ok(())
    }

    fn abs_layer(&self, layer: LayerKey) -> abs::Layer {
        abs::Layer(self.layer_idx(layer))
    }

    fn layer(&self, layer: abs::Layer) -> LayerKey {
        self.layers[layer.0].layer
    }
}

impl Draw for GreedyRouter {
    fn draw(self) -> crate::error::Result<Group> {
        Ok(self.group)
    }
}

impl DrawRef for GreedyRouter {
    fn draw_ref(&self) -> crate::error::Result<Group> {
        Ok(self.group.clone())
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct Segment {
    track_id: usize,
    rect: Rect,
    lower_boundary: bool,
    upper_boundary: bool,
}
