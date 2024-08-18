//! A simple CLI to compile asset packs from asset folders

use clap::{arg, Parser};
use smve_asset_pack::pack_io::compiling::AssetPackCompiler;
use std::path::PathBuf;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

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
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse();

    let result = AssetPackCompiler::new().compile(args.assets, args.out);

    if let Err(err) = result {
        error!("Failed to compile assets! Error: {err}");
    }
}
