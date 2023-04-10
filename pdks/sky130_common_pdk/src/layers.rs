use substrate::layout::layers::{LayerInfo, LayerType, Layers};

use crate::Sky130Pdk;

impl Sky130Pdk {
    pub fn layers() -> Layers {
        // FIXME: add the other (infrequently used) layers.
        Layers::from_csv(
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/layers.csv")),
            |name| {
                if name.starts_with("met") || name == "li1" {
                    let num = if name == "li1" {
                        0
                    } else {
                        name[3..].parse::<usize>().unwrap()
                    };
                    LayerInfo::builder()
                        .route_idx(num)
                        .metal_idx(num)
                        .layer_type(LayerType::Metal)
                        .build()
                        .unwrap()
                } else if name.starts_with("via") || name == "mcon" {
                    let num = if name == "mcon" {
                        0
                    } else if name == "via" {
                        1
                    } else {
                        name[3..].parse::<usize>().unwrap()
                    };
                    LayerInfo::builder()
                        .via_idx(num)
                        .layer_type(LayerType::Via)
                        .build()
                        .unwrap()
                } else {
                    let layer_type = match name {
                        "licon1" => LayerType::Via,
                        "pwell" | "nwell" | "dnwell" | "diff" => LayerType::Well,
                        "tap" => LayerType::Tap,
                        "psdm" | "nsdm" | "lvtn" | "hvtp" => LayerType::Implant,
                        "poly" => LayerType::Gate,
                        _ => LayerType::Other,
                    };
                    LayerInfo::builder().layer_type(layer_type).build().unwrap()
                }
            },
        )
        .unwrap()
    }
}
