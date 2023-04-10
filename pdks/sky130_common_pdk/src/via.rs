use subgeom::{Dims, Dir};
use substrate::error::Result;
use substrate::layout::context::LayoutCtx;
use substrate::layout::elements::via::generators::{MaxViaArray, ViaArrayDims};
use substrate::layout::elements::via::{ViaParams, ViaSelector};
use substrate::layout::layers::selector::Selector;
use substrate::layout::layers::LayerBoundBox;

use crate::constants::NPC_LICON_POLY_ENCLOSURE;
use crate::Sky130Pdk;

impl Sky130Pdk {
    pub fn via_layout(ctx: &mut LayoutCtx, params: &ViaParams) -> Result<()> {
        let layers = ctx.layers();
        let (top, bot) = match params.selector {
            ViaSelector::Name(_) => panic!("named via selectors are not supported by this PDK"),
            ViaSelector::Layers { top, bot } => (top, bot),
        };

        let layer_top = layers.info(top)?;
        let layer_bot = layers.info(bot)?;
        let (bot_name, top_name) = (layer_bot.name.as_str(), layer_top.name.as_str());

        let via = if let (Some(idx_top), Some(idx_bot)) = (layer_top.metal_idx, layer_bot.metal_idx)
        {
            assert_eq!(
                idx_bot + 1,
                idx_top,
                "must create via between adjacent metal layers"
            );
            layers.get(Selector::Via(idx_bot))?
        } else {
            match top_name {
                "li1" => layers.get(Selector::Name("licon1"))?,
                _ => panic!("unsupported via: {bot_name}-{top_name}"),
            }
        };

        let (size, space, bot_ext, bot_ext_one, top_ext, top_ext_one) = match (bot_name, top_name) {
            ("li1", "met1") => (170, 190, 0, 0, 30, 60),
            ("met1", "met2") => (150, 170, 55, 85, 55, 85),
            ("met2", "met3") => (200, 200, 40, 85, 65, 65),
            ("diff", "li1") => (170, 170, 40, 60, 0, 80),
            ("tap", "li1") => (170, 170, 0, 120, 0, 80),
            ("poly", "li1") => (170, 170, 50, 80, 0, 80),
            (bot, top) => {
                panic!("unsupported via: {bot}-{top}");
            }
        };

        let (top_fixed, top_dims) = if let Some(top_extension) = params.top_extension {
            match top_extension {
                Dir::Horiz => (true, Dims::new(top_ext_one, top_ext)),
                Dir::Vert => (true, Dims::new(top_ext, top_ext_one)),
            }
        } else {
            (false, Dims::new(top_ext, top_ext_one))
        };

        let (bot_fixed, bot_dims) = if let Some(bot_extension) = params.bot_extension {
            match bot_extension {
                Dir::Horiz => (true, Dims::new(bot_ext_one, bot_ext)),
                Dir::Vert => (true, Dims::new(bot_ext, bot_ext_one)),
            }
        } else {
            (false, Dims::new(bot_ext, bot_ext_one))
        };

        let dims = ViaArrayDims::new(
            bot_dims,
            bot_fixed,
            top_dims,
            top_fixed,
            Dims::square(size),
            Dims::square(space),
        );
        let generator = MaxViaArray::new(dims, bot, top, via, params.expand, 5);
        let elems = generator.draw(params.bot, params.top);

        if (bot_name, top_name) == ("poly", "li1") {
            ctx.draw_rect(
                layers.get(Selector::Name("npc"))?,
                elems
                    .layer_bbox(via)
                    .into_rect()
                    .expand(NPC_LICON_POLY_ENCLOSURE),
            )
        }
        ctx.draw(elems)?;

        Ok(())
    }
}
