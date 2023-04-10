# mergegds

A utility for merging GDS files.

## Installation

Installation is fairly simple:
1. Install [Rust](https://www.rust-lang.org/tools/install).
2. Ensure you are in the `scripts/mergegds/` folder, then run `cargo install --path .`

This will add the `mergegds` binary to your `PATH`.
If you wish to remove debug info from the binary, you can run `strip` on it.

## Usage

### Command Line

`mergegds` can be run from the command line:

```
Merge GDS files with automatic renaming of duplicate cells

Usage: mergegds --output <OUTPUT> <INPUTS>...

Arguments:
  <INPUTS>...
          The input GDS files

Options:
  -o, --output <OUTPUT>
          The output GDS file

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

To view a help message, run `mergegds --help`, or `mergegds -h` for short.

### As a Library

`mergegds` can also be used as a library in any Rust project. Just add it as a dependency
in your `Cargo.toml`.
The primary entrypoint is `mergegds::merge`, which takes an output path and an iterator of
input paths. This function is defined in `src/lib.rs`.

## Behavior

Running `mergegds` will produce a GDS file that contains all cell definitions
that were present in the input files.

Note that a GDS file cannot contain two cells with the same name,
but multiple separate GDS files may contain duplicate names.

When there are multiple cells with the same name, `mergegds` automatically renames cells
according to the following rules:
1. Input files are read in the order they are specified.
2. Names are allocated on a first-come first-served basis.
3. When a duplicate name is first detected, the **second** cell found is renamed to
```
{original_cell_name}_2
```
4. For multiple conflicts for the same name, subsequent cells will be renamed to 
```
{original_cell_name}_{n}
```
where n = 2, 3, 4, ....

## Example

Suppose we have two files, `in1.gds` and `in2.gds` with the following cell definitions:
```
in1.gds: cell_a, cell_b
in2.gds: cell_a, cell_a_2, cell_b
```

Running `mergegds -o out.gds in1.gds in2.gds` will produce a GDS file
`out.gds` with the following contents:
```
cell_a: cell_a from in1.gds
cell_b: cell_b from in1.gds
cell_a_2: cell_a from in2.gds
cell_a_2_2: cell_a_2 from in2.gds
cell_b_2: cell_b from in2.gds
```
