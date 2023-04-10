use std::path::PathBuf;

use clap::Parser;
use mergegds::merge;

#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = "Merge GDS files with automatic renaming of duplicate cells"
)]
pub struct Args {
    /// The output GDS file.
    #[arg(short, long)]
    output: PathBuf,
    /// The input GDS files.
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

pub fn main() {
    let args = Args::parse();
    merge(args.output, args.inputs).expect("failed to merge GDS files");
}
