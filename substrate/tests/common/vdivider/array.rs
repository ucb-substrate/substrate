use arcstr::ArcStr;
use subgeom::bbox::BoundBox;
use substrate::component::{Component, NoParams};
use substrate::index::IndexOwned;
use substrate::layout::placement::align::AlignRect;
use substrate::schematic::circuit::Direction;

use super::VDivider;

pub struct VDividerArray;

impl Component for VDividerArray {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("vdivider_array")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let n = 10;

        let vss = ctx.port("vss", Direction::InOut);
        let vdd = ctx.port("vdd", Direction::InOut);
        let out = ctx.bus_port("out", n, Direction::Output);

        for i in 0..n {
            let mut inst = ctx.instantiate::<VDivider>(&NoParams)?;
            inst.connect_all([("vdd", &vdd), ("vss", &vss), ("out", &out.index(i))]);
            inst.set_name(format!("vdivider_{i}"));
            ctx.add_instance(inst);
        }

        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let n = 10;

        let mut prev = None;
        for _ in 0..n {
            let mut inst = ctx.instantiate::<VDivider>(&NoParams)?;
            if let Some(prev) = prev {
                inst.align_to_the_right_of(prev, 0);
            }
            prev = Some(inst.bbox());
            ctx.draw(inst)?;
        }
        Ok(())
    }
}

/// A useless component that just instantiates a [`VDividerArray`] and bubbles the ports.
pub struct VDividerArrayWrapper;

impl Component for VDividerArrayWrapper {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("vdivider_array_wrapper")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let mut inst = ctx.instantiate::<VDividerArray>(&NoParams)?;
        ctx.bubble_all_ports(&mut inst);
        ctx.add_instance(inst);
        Ok(())
    }

    fn layout(
        &self,
        ctx: &mut substrate::layout::context::LayoutCtx,
    ) -> substrate::error::Result<()> {
        let inst = ctx.instantiate::<VDividerArray>(&NoParams)?;
        for port in inst.ports() {
            ctx.add_port(port).unwrap();
        }
        ctx.draw(inst)?;
        Ok(())
    }
}
