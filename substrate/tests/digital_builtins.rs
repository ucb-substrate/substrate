use common::setup_ctx;
use substrate::digital::modules::reg::RegReset;
use substrate::digital::types::UInt;
use substrate::digital::wire::Literal;

mod common;

#[test]
fn test_generate_digital_reg() {
    let ctx = setup_ctx();
    ctx.instantiate_digital::<RegReset<UInt>>(&Literal::<UInt>::new(4, 5).unwrap())
        .expect("failed to instantiate digital view");
}
