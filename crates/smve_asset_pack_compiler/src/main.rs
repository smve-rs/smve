//! A simple CLI to compile asset packs from asset folders

use std::path::PathBuf;
use clap::{arg, Parser};
use env_logger::Env;
use smve_asset_pack::pack_io::compiling::AssetPackCompiler;
use smve_asset_pack::pack_io::compiling::raw_assets::uncookers::text::TextAssetUncooker;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the folder containing the assets.
    #[arg(short, long)]
    assets: PathBuf,
    /// Path to the output pack file.
    #[arg(short, long)]
    out: PathBuf,
    // TODO: Add custom uncookers with lua scripting
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    let args = Args::parse();

    let result = AssetPackCompiler::new()
        .compile(args.assets, args.out);

    if let Err(err) = result {
        eprintln!("Failed to compile assets! Error: {err}");
    }
}
