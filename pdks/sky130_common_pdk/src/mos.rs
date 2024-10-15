use std::collections::HashMap;

use subgeom::bbox::BoundBox;
use subgeom::transform::Translate;
use subgeom::{Dir, Point, Rect, Shape, Span};
use substrate::error::Result;
use substrate::layout::cell::{CellPort, Element};
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::{Via, ViaExpansion, ViaParams};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::{LayerBoundBox, LayerPurpose, LayerSpec};
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::{GateContactStrategy, LayoutMosParams};

use crate::constants::{
    DIFF_EDGE_TO_GATE, DIFF_NSDM_ENCLOSURE, DIFF_NWELL_ENCLOSURE, DIFF_PSDM_ENCLOSURE, DIFF_SPACE,
    DIFF_TO_OPPOSITE_DIFF, FINGER_SPACE, POLY_DIFF_EXTENSION, POLY_SPACE,
};
use crate::Sky130Pdk;

impl Sky130Pdk {
    /// Fold MOS devices to avoid MOS devices wider than 100 microns.
    pub fn fold_mos(w: i64) -> Vec<i64> {
        let grid_w = w / 5;
        const MAX_WIDTH: i64 = 100_000;
        let n = grid_w / MAX_WIDTH + 1;
        let div = grid_w / n;
        let rem = grid_w % n;
        let mut out = vec![5 * (div + 1); rem as usize];
        out.extend(vec![5 * div; (n - rem) as usize]);
        out
    }

    pub fn mos_layout(ctx: &mut LayoutCtx, params: &LayoutMosParams) -> Result<()> {
        params.validate()?;

        let layers = ctx.layers();

        let gate_metal = layers.get(Selector::Metal(0))?;
        let sd_metal = layers.get(Selector::Metal(0))?;
        let poly = layers.get(Selector::Name("poly"))?;
        let diff = layers.get(Selector::Name("diff"))?;
        let npc = layers.get(Selector::Name("npc"))?;

        let nf = params.fingers();
        assert!(nf > 0);

        // Diff length perpendicular to gates
        let diff_perp =
            2 * DIFF_EDGE_TO_GATE + nf as i64 * params.length() + (nf as i64 - 1) * FINGER_SPACE;

        let mut prev = None;
        let x0 = 0;
        let mut cx = x0;
        let y0 = 0;

        let mut diff_xs = Vec::new();

        let mut prev_psdm: Option<Rect> = None;
        let mut prev_nsdm: Option<Rect> = None;

        for d in params.devices.iter() {
            let mut d_diff_xs = Vec::new();
            for w in Self::fold_mos(d.w) {
                if let Some(mt) = prev {
                    if mt != d.kind(&ctx.mos_db()) {
                        cx += DIFF_TO_OPPOSITE_DIFF;
                    } else {
                        cx += DIFF_SPACE;
                    }
                }

                d_diff_xs.push(cx);

                let rect = Rect::new(Point::new(cx, y0), Point::new(cx + w, y0 + diff_perp));

                if d.kind(&ctx.mos_db()) == MosKind::Pmos {
                    let mut psdm_box = rect;
                    psdm_box = psdm_box.expand(DIFF_PSDM_ENCLOSURE);

                    let psdm = layers.get(Selector::Name("psdm"))?;

                    if let Some(prev_psdm) = prev_psdm {
                        psdm_box = psdm_box.union(prev_psdm.into()).into_rect();
                    }

                    ctx.draw_rect(psdm, psdm_box);

                    prev_psdm = Some(psdm_box);
                    prev_nsdm = None;

                    let mut well_box = rect;
                    well_box = well_box.expand(DIFF_NWELL_ENCLOSURE);

                    let nwell = layers.get(Selector::Name("nwell"))?;

                    ctx.draw_rect(nwell, well_box);
                } else {
                    let mut nsdm_box = rect;
                    nsdm_box = nsdm_box.expand(DIFF_NSDM_ENCLOSURE);

                    let nsdm = layers.get(Selector::Name("nsdm"))?;

                    if let Some(prev_nsdm) = prev_nsdm {
                        nsdm_box = nsdm_box.union(prev_nsdm.into()).into_rect();
                    }

                    prev_nsdm = Some(nsdm_box);
                    prev_psdm = None;

                    ctx.draw_rect(nsdm, nsdm_box);
                }

                ctx.draw_rect(diff, rect);

                cx += w;

                prev = Some(d.kind(&ctx.mos_db()));
            }
            diff_xs.push(d_diff_xs);
        }

        let empty_rect = Rect::new(Point::zero(), Point::zero());
        let gate_ctp = ViaParams::builder()
            .layers(poly, gate_metal)
            .geometry(empty_rect, empty_rect)
            .expand(ViaExpansion::Minimum)
            .build();
        let gate_ct = ctx.instantiate::<Via>(&gate_ctp)?;
        let gate_bbox = gate_ct.layer_bbox(poly);

        let mut gate_pins = Vec::with_capacity(nf as usize);

        let xpoly = x0 - POLY_DIFF_EXTENSION;
        let mut ypoly = y0 + DIFF_EDGE_TO_GATE;
        let wpoly = cx - xpoly + POLY_DIFF_EXTENSION;

        // TODO: Need to move gate contacts further away from transistor.
        // There are several relevant design rules, but for now I'll just
        // add a constant offset.
        let poly_fudge_x = 60;
        let mut poly_rects = Vec::with_capacity(nf as usize);
        for _ in 0..nf {
            let rect = Rect {
                p0: Point::new(xpoly - poly_fudge_x, ypoly),
                p1: Point::new(xpoly + wpoly, ypoly + params.length()),
            };
            poly_rects.push(rect);
            ctx.draw(Element {
                net: None,
                layer: LayerSpec::new(poly, LayerPurpose::Drawing),
                inner: Shape::Rect(rect),
            })?;

            ypoly += params.length();
            ypoly += FINGER_SPACE;
        }

        let gate_span = Span::new(poly_rects[0].p0.y, poly_rects.last().unwrap().p1.y);
        // Place gate contacts and create gate ports
        match params.contact_strategy {
            GateContactStrategy::SingleSide => {
                assert!(
                    nf <= 2,
                    "can only contact nf=2 transistors on a single side"
                );
                let line = gate_bbox.height();
                let space = POLY_SPACE;
                let total_contact_len = nf as i64 * line + (nf as i64 - 1) * space;
                let contact_span = Span::from_center_span_gridded(
                    gate_span.center(),
                    total_contact_len,
                    ctx.pdk().layout_grid(),
                );

                let mut npc_boxes = Vec::new();

                for i in 0..nf as i64 {
                    let empty_rect = Rect::new(Point::zero(), Point::zero());
                    let gate_ctp = ViaParams::builder()
                        .layers(poly, gate_metal)
                        .geometry(empty_rect, empty_rect)
                        .expand(ViaExpansion::Minimum)
                        .build();
                    let mut gate_ct = ctx.instantiate::<Via>(&gate_ctp)?;

                    let bot = contact_span.start() + i * (line + space);
                    let rect = poly_rects[i as usize];
                    let ofsx = rect.p0.x - gate_bbox.p1.x;
                    let ofsy = bot - gate_bbox.p0.y;

                    let ct_ofs = Point::new(ofsx, ofsy);
                    gate_ct.translate(ct_ofs);
                    let ct_box = gate_ct.layer_bbox(gate_metal).into_rect();
                    let mut port = CellPort::new(format!("gate_{i}"));
                    port.add(gate_metal, Shape::Rect(ct_box));
                    ctx.add_port(port).unwrap();
                    gate_pins.push(ct_box);

                    let npc_bbox = gate_ct.layer_bbox(npc).into_rect();
                    npc_boxes.push(npc_bbox);

                    ctx.draw(gate_ct)?;

                    let top_npc = npc_boxes.last().unwrap();
                    let npc_merge_rect = Rect::new(
                        Point::new(npc_boxes[0].p0.x, npc_boxes[0].p0.y),
                        Point::new(top_npc.p1.x, top_npc.p1.y),
                    );
                    ctx.draw(Element {
                        net: None,
                        layer: LayerSpec::new(npc, LayerPurpose::Drawing),
                        inner: Shape::Rect(npc_merge_rect),
                    })?;
                }
            }
            GateContactStrategy::Merge => {
                let ct_rect = Rect::from_spans(
                    Span::with_stop_and_length(poly_rects[0].p0.x, 330),
                    gate_span,
                );
                let gate_ctp = ViaParams::builder()
                    .layers(poly, gate_metal)
                    .geometry(ct_rect, ct_rect)
                    .expand(ViaExpansion::LongerDirection)
                    .build();
                ctx.draw_rect(poly, ct_rect);
                let gate_ct = ctx.instantiate::<Via>(&gate_ctp)?;
                ctx.draw_ref(&gate_ct)?;
                let ct_box = gate_ct.layer_bbox(gate_metal).into_rect();
                let mut port = CellPort::new("gate");
                port.add(gate_metal, Shape::Rect(ct_box));
                ctx.add_port(port).unwrap()
            }
            _ => unimplemented!(),
        }

        // Add source/drain contacts
        let mut cy = y0;

        let mut sd_pins = (0..params.devices.len())
            .map(|_| HashMap::new())
            .collect::<Vec<_>>();

        for i in 0..=nf {
            for ((device, skip_sd_metal), (j, xs)) in params
                .devices
                .iter()
                .zip(params.skip_sd_metal.iter())
                .zip(diff_xs.iter().enumerate())
            {
                if Self::fold_mos(device.w).len() == 1 && skip_sd_metal.contains(&(i as usize)) {
                    continue;
                }

                let mut sd_rects = Vec::new();
                for (w, x) in Sky130Pdk::fold_mos(device.w).into_iter().zip(xs) {
                    let via_rect = Rect::new(Point::zero(), Point::new(w, 0));
                    let via_params = ViaParams::builder()
                        .layers(diff, sd_metal)
                        .geometry(via_rect, via_rect)
                        .expand(ViaExpansion::LongerDirection)
                        .bot_extension(Dir::Horiz)
                        .top_extension(Dir::Horiz)
                        .build();
                    let mut inst = ctx.instantiate::<Via>(&via_params)?;
                    let bbox = inst.layer_bbox(diff);
                    let ofsx = (w - bbox.width()) / 2;
                    let loc = Point::new(x - bbox.p0.x + ofsx, cy - bbox.p0.y);
                    inst.translate(loc);
                    sd_rects.push(inst.layer_bbox(sd_metal));
                    ctx.draw(inst)?;
                }

                let sd_rect = sd_rects
                    .into_iter()
                    .reduce(|a, b| a.union(b))
                    .unwrap()
                    .into_rect();
                let mut port = CellPort::new(format!("sd_{j}_{i}"));
                port.add(sd_metal, Shape::Rect(sd_rect));
                ctx.add_port(port).unwrap();
                sd_pins[j].insert(i, Some(sd_rect));
            }
            cy += params.length();
            cy += FINGER_SPACE;
        }
        Ok(())
    }
}
