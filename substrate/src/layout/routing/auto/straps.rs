use std::collections::HashMap;

use subgeom::bbox::BoundBox;
use subgeom::Rect;

use super::GreedyRouter;
use crate::layout::cell::Instance;
use crate::layout::context::LayoutCtx;
use crate::layout::elements::via::{Via, ViaParams};
use crate::layout::layers::LayerKey;
use crate::layout::placement::place_bbox::PlaceBbox;
use crate::layout::straps::SingleSupplyNet;

#[derive(Default)]
pub struct RoutedStraps {
    strap_layers: Vec<LayerKey>,
    targets: HashMap<LayerKey, Vec<Target>>,
}

pub struct PlacedStraps {
    inner: HashMap<LayerKey, Vec<Strap>>,
}

impl PlacedStraps {
    pub fn on_layer(&self, layer: LayerKey) -> impl Iterator<Item = Strap> + '_ {
        self.inner[&layer].iter().copied()
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Strap {
    pub rect: Rect,
    pub net: SingleSupplyNet,
    /// Indicates if this segment is on the lower or left boundary of the routing region.
    pub lower_boundary: bool,
    /// Indicates if this segment is on the upper or right boundary of the routing region.
    pub upper_boundary: bool,
}

pub struct Target {
    rect: Rect,
    net: SingleSupplyNet,
    hit: bool,
}

impl Target {
    pub fn new(net: SingleSupplyNet, rect: impl Into<Rect>) -> Self {
        Self {
            rect: rect.into(),
            net,
            hit: false,
        }
    }
}

#[inline]
fn index(net: SingleSupplyNet) -> usize {
    match net {
        SingleSupplyNet::Vss => 0,
        SingleSupplyNet::Vdd => 1,
    }
}

#[inline]
fn net_from_idx(idx: usize) -> SingleSupplyNet {
    if idx % 2 == 0 {
        SingleSupplyNet::Vss
    } else {
        SingleSupplyNet::Vdd
    }
}

impl RoutedStraps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_strap_layers(&mut self, layers: impl IntoIterator<Item = LayerKey>) -> &mut Self {
        self.strap_layers = layers.into_iter().collect();
        self
    }

    pub fn add_target(&mut self, layer: LayerKey, target: Target) -> &mut Self {
        let list = self.targets.entry(layer).or_insert(Vec::with_capacity(1));
        list.push(target);
        self
    }

    pub fn fill(
        &mut self,
        router: &GreedyRouter,
        ctx: &mut LayoutCtx,
    ) -> crate::error::Result<PlacedStraps> {
        assert!(self.strap_layers.len() >= 2);
        let mut map = HashMap::new();

        for layer in self.strap_layers.iter() {
            let segments = router.segments(*layer);

            let layer_idx = router.key_to_index[layer];
            let mut valid_target_layers = Vec::new();
            if layer_idx > 0 {
                let below = router.layers[layer_idx - 1].layer;
                valid_target_layers.push((below, (below, *layer)));
            }
            if layer_idx + 1 < router.layers.len() {
                let above = router.layers[layer_idx + 1].layer;
                valid_target_layers.push((above, (*layer, above)));
            }
            for segment in segments {
                ctx.draw_rect(*layer, segment.rect);
                let entry = map.entry(*layer).or_insert(Vec::new());
                entry.push(Strap {
                    rect: segment.rect,
                    net: net_from_idx(segment.track_id),
                    lower_boundary: segment.lower_boundary,
                    upper_boundary: segment.upper_boundary,
                });
                for (target_layer, (bot, top)) in valid_target_layers.iter() {
                    if let Some(t) = self.targets.get_mut(target_layer) {
                        for t in t.iter_mut() {
                            if index(t.net) == segment.track_id % 2 {
                                let intersection = t.rect.intersection(segment.rect.bbox());
                                if !intersection.is_empty() {
                                    let viap = ViaParams::builder()
                                        .geometry(t.rect, segment.rect)
                                        .layers(*bot, *top)
                                        .build();
                                    let via = ctx.instantiate::<Via>(&viap)?;
                                    if intersection.bbox().union(via.bbox()) == intersection {
                                        ctx.draw(via)?;
                                        t.hit = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for i in 0..self.strap_layers.len() - 1 {
            let bot = self.strap_layers[i];
            let top = self.strap_layers[i + 1];

            let top_segments = router.segments(top);
            let bot_segments = router.segments(bot);

            let mut via: Option<Instance> = None;
            for t in top_segments.iter().copied() {
                for b in bot_segments.iter().copied() {
                    let intersection = t.rect.intersection(b.rect.bbox());
                    if t.track_id % 2 == b.track_id % 2 && !intersection.is_empty() {
                        if let Some(ref via) = via {
                            let mut via = via.clone();
                            via.place_center(
                                intersection.center().snap_to_grid(ctx.pdk().layout_grid()),
                            );
                            ctx.draw(via)?;
                        } else {
                            let viap = ViaParams::builder()
                                .geometry(b.rect, t.rect)
                                .layers(bot, top)
                                .build();
                            let inner = ctx.instantiate::<Via>(&viap)?;
                            via = Some(inner.clone());
                            ctx.draw(inner)?;
                        }
                    }
                }
            }
        }

        Ok(PlacedStraps { inner: map })
    }
}
