use std::path::PathBuf;
use clap::{Parser, Subcommand};

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Compile { source } => {
            println!("Compiling from {}", source.display());
        }
        Commands::Decompile { bundle, dest } => {
            println!("Decompiling from {} to {}", bundle.display(), dest.display());
        }
        Commands::Read { bundle, path, dest} => {
            println!("Reading {} from {}", path, bundle.display());
            match dest {
                None => {
                    println!("Outputting to stdout")
                }
                Some(dest) => {
                    println!("Outputting to {}", dest.display());
                }
            }
        }
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a file bundle from the directory specified in SOURCE
    Compile {
        /// The directory of files to compile
        source: PathBuf
    },
    /// Decompiles a file bundle into DEST
    Decompile {
        /// The bundle to decompile
        bundle: PathBuf,
        /// The directory to decompile to
        dest: PathBuf
    },
    /// Reads a single file either into stdout or into a file
    Read {
        /// The bundle to read from
        bundle: PathBuf,
        /// The path where the file is found in the bundle
        path: String,
        /// Optional destination to save the file to
        dest: Option<PathBuf>
    }
}
