//! Contains the [`LogPlugin`]

use bevy_app::{App, Plugin};
use std::io;
use log::{info, Level};
use owo_colors::{OwoColorize, Style};

/// The plugin that manages logging.
/// 
/// Adding this plugin will initialize `log` implementation that will log to the console and/or a file.
/// If the feature `log-to-console` is set then console logging will be initialized.
/// If the feature `log-to-file` is set then file logging will be initialized.
/// Both can be set.
pub struct LogPlugin;

impl Plugin for LogPlugin {
    fn build(&self, _app: &mut App) {
        let mut dispatch = fern::Dispatch::new();

        #[cfg(feature = "log-to-console")]
        {
            let level_styles = LevelStyles::default();

            dispatch = dispatch.chain(
                fern::Dispatch::new()
                    .format(move |out, message, record| {
                        out.finish(format_args!(
                            "{}  {} {} {}",
                            humantime::format_rfc3339_millis(std::time::SystemTime::now()).bright_black(),
                            format!("[{}]", record.level()).style(level_styles.style(record.level())),
                            record.target().bright_black(),
                            message
                        ))
                    })
                    .level(log::LevelFilter::Info)
                    .chain(io::stdout()),
            )
        };

        #[cfg(feature = "log-to-file")]
        {
            let result = initialize_log_directory();
            if result.is_err() {
                eprintln!(
                    "Failed to initialize log directory: {}",
                    result.unwrap_err()
                );
                return;
            }

            let date = chrono::Utc::now();
            let log_path = date
                .format("logs/ruxel_log_%Y-%m-%d_%H-%M-%S-%f.log")
                .to_string();

            let log_file = fern::log_file(log_path);
            if log_file.is_err() {
                eprintln!("Failed to open log file: {}", log_file.unwrap_err());
                return;
            }
            let log_file = log_file.unwrap();

            dispatch = dispatch.chain(
                fern::Dispatch::new()
                    .format(|out, message, record| {
                        out.finish(format_args!(
                            "[{} | {} | {}] {}",
                            humantime::format_rfc3339_millis(std::time::SystemTime::now()),
                            record.level(),
                            record.target(),
                            message
                        ))
                    })
                    .level(log::LevelFilter::Debug)
                    .level_for("wgpu_core", log::LevelFilter::Info)
                    .level_for("wgpu_hal", log::LevelFilter::Info)
                    .chain(log_file),
            )
        }

        let result = dispatch.apply();
        if result.is_err() {
            eprintln!("Failed to set logger: {}", result.unwrap_err());
            return;
        }

        log_panics::init();

        /// Value containing the cargo `version` metadata
        const RUXEL_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");

        info!("Hello! This is the LogPlugin speaking.");
        info!("You are reading the logs of Ruxel version {}. Enjoy!", RUXEL_VERSION.unwrap_or("unknown"));
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

/// This contains styles (colors) for logging the different levels.
/// 
/// [`LevelStyles::default`] returns the following styles:
/// 
/// | Level            | Style                |
/// | :--------------: | :------------------: |
/// | [`Level::Trace`] | Bright Purple + Bold |
/// | [`Level::Debug`] | Blue + Bold          |
/// | [`Level::Info`]  | Green + Bold         |
/// | [`Level::Warn`]  | Yellow + Bold        |
/// | [`Level::Error`] | Red + Bold           |
struct LevelStyles {
    /// Style for [`Level::Trace`]
    trace: Style,
    /// Style for [`Level::Debug`]
    debug: Style,
    /// Style for [`Level::Info`]
    info: Style,
    /// Style for [`Level::Warn`]
    warn: Style,
    /// Style for [`Level::Error`]
    error: Style
}

impl Default for LevelStyles {
    fn default() -> Self {
        LevelStyles {
            trace: Style::new().bright_purple().bold(),
            debug: Style::new().blue().bold(),
            info: Style::new().green().bold(),
            warn: Style::new().yellow().bold(),
            error: Style::new().red().bold()
        }
    }
}

impl LevelStyles {
    /// Returns the styles for the specified level
    fn style(&self, level: Level) -> Style {
        match level {
            Level::Error => self.error,
            Level::Warn => self.warn,
            Level::Info => self.info,
            Level::Debug => self.debug,
            Level::Trace => self.trace
        }
    }
}
