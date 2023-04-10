use arcstr::ArcStr;
use common::{out_path, setup_ctx};
use substrate::component::{Component, NoParams};
use substrate::error::ErrorSource;
use substrate::schematic::circuit::Direction;

mod common;

pub struct Buffer;

impl Component for Buffer {
    type Params = NoParams;
    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let _input = ctx.port("input", Direction::Input);
        let _output = ctx.port("output", Direction::Output);

        ctx.set_spice("* An opaque implementation of a buffer");
        Ok(())
    }
}

/// A component that "reads from" a net with no drivers.
pub struct NoDriver;

impl Component for NoDriver {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let out = ctx.port("out", Direction::Output);

        let no_driver = ctx.signal("input");

        let mut buf = ctx.instantiate::<Buffer>(&NoParams)?;
        buf.connect("input", no_driver);
        buf.connect("output", out);
        ctx.add_instance(buf);

        Ok(())
    }
}

/// A valid component.
pub struct Valid;

impl Component for Valid {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("valid_component")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let out = ctx.port("out", Direction::Output);
        let input = ctx.port("input", Direction::Input);

        let mid = ctx.signal("mid");

        let mut buf = ctx.instantiate::<Buffer>(&NoParams)?;
        buf.connect("input", input);
        buf.connect("output", mid);
        ctx.add_instance(buf);

        let mut buf = ctx.instantiate::<Buffer>(&NoParams)?;
        buf.connect("input", mid);
        buf.connect("output", out);
        ctx.add_instance(buf);
        Ok(())
    }
}

/// A component with signals that have a naming conflict.
pub struct NameConflict1;

impl Component for NameConflict1 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("name_conflict_1")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        // Ports with conflicting names
        let _out1 = ctx.port("out", Direction::Output);
        let _out2 = ctx.port("out", Direction::Output);

        ctx.set_spice("* Implementation omitted");
        Ok(())
    }
}

/// A component with signals that have a naming conflict.
pub struct NameConflict2;

impl Component for NameConflict2 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("name_conflict_2")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        // Signals with conflicting names.
        let _out = ctx.port("out", Direction::Output);
        let _in = ctx.port("in", Direction::Input);
        let _tmp1 = ctx.signal("tmp");
        let _tmp1 = ctx.signal("tmp");

        ctx.set_spice("* Implementation omitted");
        Ok(())
    }
}

/// A component with signals that have a naming conflict.
pub struct NameConflict3;

impl Component for NameConflict3 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("name_conflict_3")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        // A port whose name conflicts with the name of a signal.
        let _in = ctx.port("in", Direction::Input);
        let _out1 = ctx.port("out", Direction::Output);
        let _out2 = ctx.signal("out");

        ctx.set_spice("* Implementation omitted");
        Ok(())
    }
}

/// A component with invalid signal names.
pub struct BadNames1;

impl Component for BadNames1 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("bad_names_1")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let _out1 = ctx.port("in", Direction::Input);
        let _out2 = ctx.port("out", Direction::Output);

        let _tmp = ctx.signal("signal_with_a space");

        ctx.set_spice("* Implementation omitted");
        Ok(())
    }
}

/// A component with invalid signal names.
pub struct BadNames2;

impl Component for BadNames2 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("bad_names_2")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let _out1 = ctx.port("in", Direction::Input);
        let _out2 = ctx.port("out", Direction::Output);

        let _tmp = ctx.signal(""); // empty signal name

        ctx.set_spice("* Implementation omitted");
        Ok(())
    }
}

/// A component with invalid signal names.
pub struct BadNames3;

impl Component for BadNames3 {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        arcstr::literal!("bad_names_3")
    }

    fn schematic(
        &self,
        ctx: &mut substrate::schematic::context::SchematicCtx,
    ) -> substrate::error::Result<()> {
        let _out1 = ctx.port("in", Direction::Input);
        let _out2 = ctx.port("out", Direction::Output);

        // Signal with non-ASCII characters.
        // Note the diacritic above the last character.
        let _tmp = ctx.signal("non_ascii_namé");

        ctx.set_spice("* Implementation omitted");
        Ok(())
    }
}

#[test]
fn test_no_driver() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<NoDriver>(
            &NoParams,
            out_path("test_no_driver", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}

#[test]
fn test_valid() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<Valid>(&NoParams, out_path("test_valid", "schematic.spice"))
        .expect("failed to write schematic");
}

#[test]
fn test_name_conflict_1() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<NameConflict1>(
            &NoParams,
            out_path("test_name_conflict_1", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}

#[test]
fn test_name_conflict_2() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<NameConflict2>(
            &NoParams,
            out_path("test_name_conflict_2", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}

#[test]
fn test_name_conflict_3() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<NameConflict3>(
            &NoParams,
            out_path("test_name_conflict_3", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}

#[test]
fn test_bad_names_1() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<BadNames1>(
            &NoParams,
            out_path("test_bad_names_1", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}

#[test]
fn test_bad_names_2() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<BadNames2>(
            &NoParams,
            out_path("test_bad_names_2", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}

#[test]
fn test_bad_names_3() {
    let ctx = setup_ctx();
    let err = ctx
        .write_schematic_to_file::<BadNames3>(
            &NoParams,
            out_path("test_bad_names_3", "schematic.spice"),
        )
        .expect_err("expected error writing schematic");
    assert!(
        matches!(err.into_inner(), ErrorSource::InvalidNetlist(_)),
        "should report invalid netlist error"
    );
}
