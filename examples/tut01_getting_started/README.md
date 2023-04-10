# Getting started

In this tutorial, we'll use Substrate to generate an ideal voltage divider.

Start by creating a new Rust project:

```bash
cargo new vdivider
cd vdivider
```

Ensure that your `~/.cargo/config.toml` file contains the following lines:
```toml
[net]
git-fetch-with-cli = true
```

Next, add the following dependencies to your new project's `Cargo.toml`:

```toml
substrate = { git = "https://github.com/rahulk29/substrate" }
empty_pdk = { git = "https://github.com/rahulk29/substrate" }
arcstr = "1"
```

## Setup

Before we can begin writing generators, we need to tell Substrate
what PDK we'd like to use.

All Substrate-managed configuration and state is store in an object called
a Substrate **context**. The context includes things such as:
* A set of all components you've generated.
* The current PDK.
* The currently selected simulator.
* The selected netlist writer.
* Plugins selected for DRC, LVS, and PEX.
* And much more.

For now, we'll use a fairly minimal configuration that uses
an empty PDK and Substrate's built-in SPICE netlist writer.

Open up `main.rs` and add these `use` statements:

```rust
use std::path::PathBuf;

use arcstr::ArcStr;
use empty_pdk::EmptyPdk;
use substrate::component::{Component, NoParams};
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::pdk::{Pdk, PdkParams};
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::resistor::Resistor;
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::units::{SiPrefix, SiValue};
```

Then add this function, which sets up a new Substrate context:

```rust
pub fn setup_ctx() -> SubstrateCtx {
    let pdk = EmptyPdk::new(&PdkParams {
        pdk_root: PathBuf::from("."),
    })
    .unwrap();
    let cfg = SubstrateConfig::builder()
        .netlister(SpiceNetlister::new())
        .pdk(pdk)
        .build();
    SubstrateCtx::from_config(cfg).unwrap()
}
```

## Your first component

Substrate components are types that implement the `Component` trait.

### Creating a component type

Let's start by declaring a type called `VDivider`:

```rust
struct VDivider;
```

To make this a component, we need to implement the `Component` trait.
Implementing this trait requires doing two things:
* Declaring a parameter type, which specifies the parameters your generator accepts.
* Implementing the `new` method, which returns an instance of your `Component` type,
  given the generator parameters and access to a Substrate context.

Let's fill in those two methods now:

```rust
impl Component for VDivider {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }
}
```

We use the `NoParams` parameter type to indicate that our generator doesn't take in any significant parameters.
We also fill in a very simple implementation of `new`, which just returns an instance of `Self` (ie. `VDivider`).

In general, you can use the `new` method to implement more sophisticated behavior, such as validating
that the parameters you receive are valid.

This turns `VDivider` into a mostly useless component.

### Naming components

Let's give our `VDivider` component a name by implementing the `Component::name` method:
```rust
impl Component for VDivider {
    // ...
    fn name(&self) -> ArcStr {
        arcstr::literal!("vdivider")
    }
}
```

This will tell Substrate to call instances of our component `vdivider` in generated files,
such as exported SPICE netlists and GDS files.

If you don't specify a name, Substrate will automatically pick a unique one for you.
However, it will likely be something uninformative, such as `unnamed_123`.

### Creating a schematic view

To do something useful with `VDivider`, we need to define **views**. In this tutorial, we'll
add a schematic view by implementing the `Component::schematic` method.

Let's add an empty implementation of the `schematic` method to the `impl Component` block:

```rust
fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
    Ok(())
}
```

This adds an empty schematic view to our `VDivider`. Let's add some content to our schematic.

### Defining ports

The first thing we'll do is add ports. Ports define the signals exchanged between
a component and the outside world. Our voltage divider will have three ports:
* The output port, called `out`.
* The input supply voltage port, called `vdd`.
* A ground port called `vss`.

We create ports by calling `SchematicCtx::port`. Add this to your implementation of `schematic`
to declare the ports:
```rust
let out = ctx.port("out", Direction::Output);
let vdd = ctx.port("vdd", Direction::InOut);
let vss = ctx.port("vss", Direction::InOut);
```

### Instantiating subcomponents

Next, we'll add 2 resistors. The "top" resistor will be 2 kΩ; the "bottom" resistor will be 1 kΩ.

Like most things in Substrate, resistors are components themselves. To instantiate
components in a schematic, we use the `SchematicCtx::instantiate` method. This
method takes in a generic type parameter, which indicates which component we'd like to instantiate.
It also takes the parameters we'd like to pass to the component generator.

Update your implementation of `schematic` to look like this:
```rust
fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
    let out = ctx.port("out", Direction::Output);
    let vdd = ctx.port("vdd", Direction::InOut);
    let vss = ctx.port("vss", Direction::InOut);

    ctx.instantiate::<Resistor>(&SiValue::new(2, SiPrefix::Kilo))?
        .with_connections([("p", vdd), ("n", out)])
        .named("R1")
        .add_to(ctx);

    ctx.instantiate::<Resistor>(&SiValue::new(1, SiPrefix::Kilo))?
        .with_connections([("p", out), ("n", vss)])
        .named("R2")
        .add_to(ctx);

    Ok(())
}
```

This instantiates 2 resistors with the appropriate values. It connects their ports
(named `p` and `n`) to the appropriate signals, gives them a name, and adds them to our schematic context.

We now have a voltage divider schematic!

## Exporting schematics

You will usually want to export your schematics to formats that can be
consumed by other tools, such as circuit simulators.

Let's export our voltage divider as a SPICE netlist.

In your `main` method, set up a Substrate context, then tell
the Substrate context to write a schematic to a file:
```rust
fn main() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<VDivider>(&NoParams, "build/vdivider.spice")
        .expect("failed to write schematic");
}
```

Now run `cargo run`. If all goes well, you should get a file called `build/vdivider.spice`
that looks something like this:
```spice
* vdivider
* Schematic generated by Substrate



.subckt vdivider out vdd vss
XR1 vdd out resistor_2K
XR2 out vss resistor_1K
.ends vdivider
.subckt resistor_2K p n
R1 p n 2K
.ends resistor_2K
.subckt resistor_1K p n
R1 p n 1K
.ends resistor_1K
```

This is indeed a voltage divider. If you wanted to, you could use this netlist
in most SPICE-compatible simulators.

## Final code

Your final code should look like this:

```rust
use std::path::PathBuf;

use arcstr::ArcStr;
use empty_pdk::EmptyPdk;
use substrate::component::{Component, NoParams};
use substrate::data::{SubstrateConfig, SubstrateCtx};
use substrate::pdk::{Pdk, PdkParams};
use substrate::schematic::circuit::Direction;
use substrate::schematic::context::SchematicCtx;
use substrate::schematic::elements::resistor::Resistor;
use substrate::schematic::netlist::impls::spice::SpiceNetlister;
use substrate::units::{SiPrefix, SiValue};

struct VDivider;

impl Component for VDivider {
    type Params = NoParams;

    fn new(
        _params: &Self::Params,
        _ctx: &substrate::data::SubstrateCtx,
    ) -> substrate::error::Result<Self> {
        Ok(Self)
    }

    fn name(&self) -> ArcStr {
        "vdivider".into()
    }

    fn schematic(&self, ctx: &mut SchematicCtx) -> substrate::error::Result<()> {
        let out = ctx.port("out", Direction::Output);
        let vdd = ctx.port("vdd", Direction::InOut);
        let vss = ctx.port("vss", Direction::InOut);

        ctx.instantiate::<Resistor>(&SiValue::new(2, SiPrefix::Kilo))?
            .with_connections([("p", vdd), ("n", out)])
            .named("R1")
            .add_to(ctx);

        ctx.instantiate::<Resistor>(&SiValue::new(1, SiPrefix::Kilo))?
            .with_connections([("p", out), ("n", vss)])
            .named("R2")
            .add_to(ctx);

        Ok(())
    }
}

pub fn setup_ctx() -> SubstrateCtx {
    let pdk = EmptyPdk::new(&PdkParams {
        pdk_root: PathBuf::from("."),
    })
    .unwrap();
    let cfg = SubstrateConfig::builder()
        .netlister(SpiceNetlister::new())
        .pdk(pdk)
        .build();
    SubstrateCtx::from_config(cfg).unwrap()
}

fn main() {
    let ctx = setup_ctx();
    ctx.write_schematic_to_file::<VDivider>(&NoParams, "build/vdivider.spice")
        .expect("failed to write schematic");
}
```

You can find this code in runnable form in the Substrate repository.
See the `examples/tut01_getting_started` folder.
