use std::path::PathBuf;

use common::{setup_ctx, DATA_DIR};
use subgeom::Span;
use substrate::layout::straps::{SingleSupplyNet, Strap, StrapConfig};

mod common;

#[test]
fn test_hammer_straps() {
    let ctx = setup_ctx();
    let path = PathBuf::from(DATA_DIR).join("hammer/power_straps_sky130.json");
    let straps =
        StrapConfig::<SingleSupplyNet>::from_hammer_json_file(path, "macro", &ctx).unwrap();
    assert!(straps.above_top_exists());

    let top = straps.top();
    assert_eq!(
        top.strap(0),
        Strap::new(SingleSupplyNet::Vss, Span::new(500, 1_100))
    );
    assert_eq!(
        top.strap(1),
        Strap::new(SingleSupplyNet::Vdd, Span::new(1_500, 2_100))
    );
    assert_eq!(
        top.strap(2),
        Strap::new(SingleSupplyNet::Vss, Span::new(10_500, 11_100))
    );
    assert_eq!(
        top.strap(3),
        Strap::new(SingleSupplyNet::Vdd, Span::new(11_500, 12_100))
    );
    assert_eq!(
        top.strap(4),
        Strap::new(SingleSupplyNet::Vss, Span::new(20_500, 21_100))
    );
    assert_eq!(
        top.strap(5),
        Strap::new(SingleSupplyNet::Vdd, Span::new(21_500, 22_100))
    );

    assert_eq!(top.strap_pitch(), 1_000);
    assert_eq!(top.straps_until(23_000).count(), 6);
}
