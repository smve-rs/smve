//! A simple CLI to compile asset packs from asset folders

pub mod uncooker;

use clap::{arg, Parser, ValueHint};
use smve_asset_pack::pack_io::compiling::AssetPackCompiler;
use std::{error::Error, fs::File, io::Read, path::PathBuf};
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uncooker::UserDefinedUncooker;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the folder containing the assets.
    #[arg(short, long, value_hint = ValueHint::DirPath)]
    assets: PathBuf,
    /// Path to the output pack file.
    #[arg(short, long, value_hint = ValueHint::FilePath)]
    out: PathBuf,
    /// Paths (wildcards accepted) to custom uncooker lua files.
    #[arg(short, long, value_hint = ValueHint::FilePath, num_args = 0..)]
    uncookers: Vec<PathBuf>,
    /// Don't include built-in uncookers.
    #[arg(short, long)]
    no_default_uncookers: bool,
}

fn main_inner() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::WARN.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse_from(wild::args_os());

    let mut compiler = AssetPackCompiler::new();

    if !args.no_default_uncookers {
        compiler.register_default_uncookers();
    }

    for path in args.uncookers {
        let mut file_data = String::new();

        let mut file = File::open(path).unwrap();
        file.read_to_string(&mut file_data).unwrap();

        let uncooker = UserDefinedUncooker::new(&file_data)?;
        compiler.register_asset_uncooker(uncooker);
    }

    compiler.compile(args.assets, args.out)?;

    Ok(())
}

fn main() {
    let result = main_inner();

    if let Err(err) = result {
        error!("Failed to compile assets! Error: {err}");
        eprintln!("Failed to compile assets! Please read error logs above.");
    }
}
