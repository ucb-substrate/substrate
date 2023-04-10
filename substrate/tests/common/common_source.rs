use arcstr::ArcStr;
use substrate::component::{Component, NoParams};
use substrate::data::SubstrateCtx;
use substrate::pdk::mos::query::Query;
use substrate::pdk::mos::spec::MosKind;
use substrate::pdk::mos::MosParams;
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::mos::SchematicMos;
use substrate::schematic::elements::resistor::Resistor;
use substrate::units::{SiPrefix, SiValue};

pub struct CommonSourceAmp;

impl Component for CommonSourceAmp {
    type Params = NoParams;

    fn new(_params: &Self::Params, _ctx: &SubstrateCtx) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let vin = ctx.port("vin", Direction::Input);
        let vout = ctx.port("vout", Direction::Output);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);

        let mut r = ctx.instantiate::<Resistor>(&SiValue::new(5, SiPrefix::Kilo))?;
        r.connect_all([("p", &vdd), ("n", &vout)]);
        r.set_name("RL");
        ctx.add_instance(r);

        let mos_db = ctx.mos_db();
        let device = mos_db.query(Query::builder().kind(MosKind::Nmos).build().unwrap())?;
        let mut mos = ctx.instantiate::<SchematicMos>(&MosParams {
            w: 4 * device.spec().wmin,
            l: device.spec().lmin,
            nf: 4,
            m: 1,
            id: device.id(),
        })?;

        mos.connect_all([("d", &vout), ("g", &vin), ("s", &vss), ("b", &vss)]);
        mos.set_name("M1");
        ctx.add_instance(mos);
        Ok(())
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("common_source_amp")
    }
}
