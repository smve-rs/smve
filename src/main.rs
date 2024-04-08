#![cfg_attr(feature = "windowed", windows_subsystem = "windows")]
#![deny(missing_docs)]
#![doc(html_favicon_url = "https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/icon.png")]
#![doc(html_logo_url = "https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/icon.png")]

//! <picture>
//!     <source media="(prefers-color-scheme: dark)" srcset="https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/title_logo_dark.svg">
//!     <source media="(prefers-color-scheme: light)" srcset="https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/title_logo_light.svg">
//!     <img alt="Ruxel" width="200" src="https://cdn.jsdelivr.net/gh/ItsSunnyMonster/ruxel/images/title_logo_dark.svg">
//! </picture>
//!
//! A voxel engine written in Rust.

pub mod core;

use crate::core::window::WindowPlugin;
use bevy_app::prelude::*;

use cfg_if::cfg_if;

/// The main entry point for the application.
///
/// Initializes the logger and runs the [bevy application](https://docs.rs/bevy_app/latest/bevy_app/).
pub fn main() {
    init_logger();

    log_panics::init();

    App::new().add_plugins(WindowPlugin::default()).run();
}

/// Initializes loggers based on the features enabled.
///
/// If the `log-to-file` feature is enabled, logs (level DEBUG) will be written to a file.
/// Otherwise, logs (level INFO) will be written to the console with `env_logger`.
///
/// # Panics
/// - If the log directory cannot be initialized
/// - If the logger cannot be initialized
/// - If the log file cannot be created
fn init_logger() {
    cfg_if! {
        if #[cfg(feature="log-to-file")] {
            let date = chrono::Utc::now();
            let log_file = date.format("logs/ruxel_log_%Y-%m-%d_%H-%M-%S-%f.log").to_string();

            initialize_log_directory().unwrap_or_else(|e| {
                panic!("Failed to initialize log directory: {}", e);
            });

            fern::Dispatch::new()
                .format(|out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                        record.level(),
                        record.target(),
                        message
                    ))
                })
                .level(log::LevelFilter::Debug)
                .chain(fern::log_file(log_file).unwrap_or_else(|e| {
                    panic!("Failed to create log file: {}", e);
                }))
                .apply().unwrap_or_else(|e| {
                    panic!("Failed to initialize logger: {}", e);
                });
        } else {

    use std::io::Write;
            use env_logger::fmt::style::AnsiColor;

    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or("info"))
        .format(|buf, record| {
            let level_style = buf.default_level_style(record.level()).bold();
            let time_target_style = env_logger::fmt::style::Style::new().fg_color(Some(AnsiColor::BrightBlack.into()));
            writeln!(buf, "{time_target_style}{}{time_target_style:#}  {level_style}[{}]{level_style:#} {time_target_style}{}:{time_target_style:#} {}",
                     humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                     record.level(),
                     record.target(),
                     record.args()
            )
        })
        .init();
        }
    }
}

/// Initializes the log directory and compresses old logs.
#[cfg(feature = "log-to-file")]
fn initialize_log_directory() -> Result<(), std::io::Error> {
    // Create the logs directory if it doesn't exist
    if !std::path::Path::new("logs").exists() {
        std::fs::create_dir("logs")?;
    }

    // Compress old logs
    for log in std::fs::read_dir("logs")? {
        let log = log?;
        let path = log.path();
        let metadata = log.metadata()?;

        if metadata.is_file() {
            let file_name = path
                .file_name()
                .expect("Path should be a file")
                .to_str()
                .expect("Path should contain valid unicode");
            if file_name.ends_with(".log") {
                let compressed_file_name = format!("{}.gz", file_name);
                let compressed_file_path = path.with_file_name(compressed_file_name);

                let file = std::fs::File::open(&path)?;
                let mut reader = std::io::BufReader::new(file);
                let mut compressed_file = flate2::write::GzEncoder::new(
                    std::fs::File::create(&compressed_file_path)?,
                    flate2::Compression::default(),
                );

                std::io::copy(&mut reader, &mut compressed_file)?;
                std::fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}
