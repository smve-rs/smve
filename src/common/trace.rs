//! Contains the [`TracePlugin`]

use bevy_app::{App, Plugin};
use cfg_if::cfg_if;
use tracing_panic::panic_hook;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry;
use tracing_subscriber::util::SubscriberInitExt;

cfg_if! {
    if #[cfg(feature = "log-to-file")] {
        use std::fs::{File, OpenOptions};
        use std::time::SystemTime;
        use tracing::{Event, Subscriber};
        use tracing_log::NormalizeEvent;
        use tracing_subscriber::fmt::format::Writer;
        use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
        use tracing_subscriber::registry::LookupSpan;
    }
}

cfg_if! {
    if #[cfg(any(feature = "log-to-console", feature = "log-to-file"))] {
        use tracing::metadata::LevelFilter;
        use tracing_subscriber::{Layer, EnvFilter};
    }
}

/// The plugin that manages logging.
///
/// Adding this plugin will initialize `log` implementation that will log to the console and/or a file.
/// If the feature `log-to-console` is set then console logging will be initialized.
/// If the feature `log-to-file` is set then file logging will be initialized.
/// Both can be set.
pub struct TracePlugin;

impl Plugin for TracePlugin {
    #[cfg_attr(not(feature = "trace"), allow(unused_variables))]
    fn build(&self, app: &mut App) {
        cfg_if! {
            if #[cfg(feature = "log-to-console")]
            {

                let filter = EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .with_env_var("SMVE_LOG")
                    .from_env_lossy();

                let stdout_log =
                    tracing_subscriber::fmt::layer()
                        .with_filter(filter);
            } else {
                // This creates a layer that does nothing
                let stdout_log = tracing_subscriber::layer::Identity::new();
            }
        }

        cfg_if! {
            if #[cfg(feature = "log-to-file")]
            {
                let filter = EnvFilter::builder()
                    .with_default_directive(LevelFilter::INFO.into())
                    .with_env_var("SMVE_LOG")
                    .from_env_lossy();

                let file = get_log_file();
                if file.is_err() {
                    eprintln!("Failed to open log file: {}", file.unwrap_err());
                    return;
                }
                let file = file.unwrap();

                let file_log =
                    tracing_subscriber::fmt::layer()
                        .event_format(FileFormatter)
                        .with_writer(file)
                        .with_ansi(false)
                        .with_filter(filter);
            } else {
                let file_log = tracing_subscriber::layer::Identity::new();
            }
        }

        cfg_if! {
            if #[cfg(feature = "trace")] {
                let result = initialize_tracing_directory();
                if result.is_err() {
                    eprintln!("Failed to initialize tracing directory: {}", result.unwrap_err());
                    return;
                }
                let date = chrono::Utc::now();
                let log_path = date
                    .format("tracing/smve_trace_%Y-%m-%d_%H-%M-%S-%f.json")
                    .to_string();
                let (chrome, guard) = tracing_chrome::ChromeLayerBuilder::new()
                    .file(log_path)
                    .name_fn(Box::new(|event_or_span| match event_or_span {
                        tracing_chrome::EventOrSpan::Event(event) => event.metadata().name().into(),
                        tracing_chrome::EventOrSpan::Span(span) => {
                            if let Some(fields) =
                                span.extensions().get::<tracing_subscriber::fmt::FormattedFields<tracing_subscriber::fmt::format::DefaultFields >>()
                            {
                                format!("{}: {}", span.metadata().name(), fields.fields.as_str())
                            } else {
                                span.metadata().name().into()
                            }
                        }
                    }))
                    .build();
                app.insert_non_send_resource(guard);
            } else {
                let chrome = tracing_subscriber::layer::Identity::new();
            }
        }

        registry()
            .with(stdout_log)
            .with(file_log)
            .with(chrome)
            .init();

        // Feed panic through tracing
        let old_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |infos| {
            panic_hook(infos);
            old_hook(infos);
        }));
    }
}

#[cfg(feature = "log-to-file")]
/// Initializes the log directory, compresses old logs and then creating a new log file and returns it.
fn get_log_file() -> Result<File, std::io::Error> {
    initialize_log_directory()?;

    let date = chrono::Utc::now();
    let log_path = date
        .format("logs/smve_log_%Y-%m-%d_%H-%M-%S-%f.log")
        .to_string();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    Ok(file)
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

                let file = File::open(&path)?;
                let mut reader = std::io::BufReader::new(file);
                let mut compressed_file = flate2::write::GzEncoder::new(
                    File::create(&compressed_file_path)?,
                    flate2::Compression::default(),
                );

                std::io::copy(&mut reader, &mut compressed_file)?;
                std::fs::remove_file(&path)?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "trace")]
/// Creates the tracing directory if it does not exist
fn initialize_tracing_directory() -> Result<(), std::io::Error> {
    if !std::path::Path::new("tracing").exists() {
        std::fs::create_dir("tracing")?;
    }

    Ok(())
}

#[cfg(feature = "log-to-file")]
/// Formatting for logging to files
///
/// Will format events in the following format:
/// \[\<timestamp>] \[\<level>] \[\<target>]: \<message>
///
/// Example:
/// \[2024-05-05T05:15:02.623Z] \[INFO] \[smve::client::core::window]: Entered event loop
struct FileFormatter;

#[cfg(feature = "log-to-file")]
impl<S, N> FormatEvent<S, N> for FileFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let metadata = event.normalized_metadata();
        let metadata = metadata.as_ref().unwrap_or(event.metadata());

        let time = humantime::format_rfc3339_millis(SystemTime::now());

        write!(
            &mut writer,
            "[{}] [{}] [{}]: ",
            time,
            metadata.level(),
            metadata.target()
        )?;

        // Format all the spans in the event's span context.
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}", span.name())?;

                // `FormattedFields` is a formatted representation of the span's
                // fields, which is stored in its extensions by the `fmt` layer's
                // `new_span` method. The fields will have been formatted
                // by the same field formatter that's provided to the event
                // formatter in the `FmtContext`.
                let ext = span.extensions();
                let fields = &ext
                    .get::<tracing_subscriber::fmt::FormattedFields<N>>()
                    .expect("will never be `None`");

                // Skip formatting the fields if the span had no fields.
                if !fields.is_empty() {
                    write!(writer, "{{{}}}", fields)?;
                }
                write!(writer, ": ")?;
            }
        }

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        writeln!(writer)
    }
}
