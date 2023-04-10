use std::collections::HashMap;
use std::path::PathBuf;

use substrate::pdk::corner::{CornerData, CornerDb, CornerSkew};
use substrate::pdk::{Supplies, Supply, SupplyId, Units};
use substrate::units::SiPrefix;

pub mod constants;
pub mod layers;
pub mod mos;
pub mod stdcells;
pub mod via;

pub struct Sky130Pdk {
    pub pdk_root: PathBuf,
}

impl Sky130Pdk {
    pub fn new(params: &substrate::pdk::PdkParams) -> substrate::error::Result<Self> {
        Ok(Self {
            pdk_root: params.pdk_root.clone(),
        })
    }

    pub fn process(&self) -> &'static str {
        "sky130"
    }

    pub fn lengths(&self) -> Units {
        Units::new(SiPrefix::Nano, SiPrefix::Nano)
    }

    pub fn voltages(&self) -> SiPrefix {
        SiPrefix::None
    }

    pub fn supplies(&self) -> substrate::pdk::Supplies {
        let values = HashMap::from_iter([(
            SupplyId::Core,
            Supply {
                typ: 1.8f64,
                ..Default::default()
            },
        )]);
        Supplies { values }
    }

    /// The grid resolution in SKY 130 is 5 nanometers.
    pub fn layout_grid(&self) -> i64 {
        5
    }

    pub fn corners(&self) -> CornerDb {
        let mut db = CornerDb::new();
        let tt = CornerData::builder()
            .name("tt")
            .nmos(CornerSkew::Typical)
            .pmos(CornerSkew::Typical)
            .build()
            .unwrap();
        let ss = CornerData::builder()
            .name("ss")
            .nmos(CornerSkew::Slow)
            .pmos(CornerSkew::Slow)
            .build()
            .unwrap();
        let sf = CornerData::builder()
            .name("sf")
            .nmos(CornerSkew::Slow)
            .pmos(CornerSkew::Fast)
            .build()
            .unwrap();
        let fs = CornerData::builder()
            .name("fs")
            .nmos(CornerSkew::Fast)
            .pmos(CornerSkew::Slow)
            .build()
            .unwrap();
        let ff = CornerData::builder()
            .name("ff")
            .nmos(CornerSkew::Fast)
            .pmos(CornerSkew::Fast)
            .build()
            .unwrap();
        let tt = db.add_corner(tt);
        db.add_corner(ss);
        db.add_corner(sf);
        db.add_corner(fs);
        db.add_corner(ff);
        db.set_default_corner(tt);
        db
    }
}
