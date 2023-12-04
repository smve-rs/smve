/*
 * RustyCraft: a voxel engine written in Rust
 * Copyright (C)  2023  SunnyMonster
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use clap::{Parser, Subcommand};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use lib_file_bundle::file_bundle::{compile, CompileStatus};
use std::error::Error;
use std::path::PathBuf;
use console::Emoji;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    match args.command {
        Commands::Compile { source, dest } => {
            let mut pb: Option<ProgressBar> = None;
            compile(&source, &dest, |compile_status, file_name, _done, total| {
                if pb.is_none() {
                    pb = Some(ProgressBar::new(total as u64 + 1));
                    pb.as_ref().unwrap().set_style(ProgressStyle::with_template("[{elapsed_precise}] [{bar}] {percent}% ({eta}) {wide_msg}").unwrap());
                }
                let pb = pb.as_ref().unwrap();
                match compile_status {
                    CompileStatus::Adding => pb.set_message(format!("Adding {}", file_name)),
                    CompileStatus::Added | CompileStatus::SkippedDirectory => {
                        pb.inc(1);
                    }
                    CompileStatus::WritingFile => {
                        pb.set_message("Writing file...");
                    }
                }
            })?;
            pb.as_ref().unwrap().inc(1);
            pb.as_ref().unwrap().finish_with_message("Finished!");
            println!("{}Outputted to {} in {}", Emoji("✅ ", ""), dest.display(), HumanDuration(pb.as_ref().unwrap().elapsed()));
        }
        Commands::Decompile { bundle, dest } => {
            println!(
                "Decompiling from {} to {}",
                bundle.display(),
                dest.display()
            );
        }
        Commands::Read { bundle, path, dest } => {
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
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a file bundle from the directory specified in SOURCE
    Compile {
        /// The directory of files to compile
        #[arg(value_parser = is_directory)]
        source: PathBuf,
        /// The destination file bundle to compile to
        dest: PathBuf,
    },
    /// Decompiles a file bundle into DEST
    Decompile {
        /// The bundle to decompile
        bundle: PathBuf,
        /// The directory to decompile to
        dest: PathBuf,
    },
    /// Reads a single file either into stdout or into a file
    Read {
        /// The bundle to read from
        bundle: PathBuf,
        /// The path where the file is found in the bundle
        path: String,
        /// Optional destination to save the file to
        dest: Option<PathBuf>,
    },
}

fn is_directory(s: &str) -> Result<PathBuf, String> {
    let path: PathBuf = s.into();

    if path.is_dir() {
        Ok(path)
    } else {
        Err(format!("{} is not a directory!", s))
    }
}

fn _is_file(s: &str) -> Result<PathBuf, String> {
    let path: PathBuf = s.into();

    if path.is_file() {
        Ok(path)
    } else {
        Err(format!("{} is not a file!", s))
    }
}
