use std::error::Error;
use std::sync::{Mutex, OnceLock};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::time::ChronoLocal;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt};

use crate::LoggerConfig;

// Keep WorkerGuard alive until the process exits.
//
// The tracing subscriber can only be initialized once.
// Do not initialize it again when the Android Service restarts.
static TRACING_GUARD: OnceLock<Mutex<Option<WorkerGuard>>> = OnceLock::new();

fn tracing_guard() -> &'static Mutex<Option<WorkerGuard>> {
    TRACING_GUARD.get_or_init(|| Mutex::new(None))
}

// Initialize the tracing subscriber.
//
// Initialize only once on both desktop and Android.
pub fn initialize(config: LoggerConfig) -> Result<(), Box<dyn Error>> {
    let mut guard_slot = tracing_guard()
        .lock()
        .map_err(|_| std::io::Error::other("tracing guard lock poisoned"))?;

    // Skip if already initialized.
    if guard_slot.is_some() {
        return Ok(());
    }

    // Create a daily rolling log file.
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(config.max_log_files)
        .filename_prefix(&config.filename_prefix)
        .filename_suffix("log")
        .build(&config.log_dir)?;

    let (non_blocking, worker_guard) = tracing_appender::non_blocking(file_appender);

    let timer = ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".to_string());

    // Create separate filters for console and file logs.
    let console_filter = EnvFilter::try_new(&config.console_filter)?;
    let file_filter = EnvFilter::try_new(&config.file_filter)?;

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_timer(timer.clone())
        .with_filter(console_filter);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_timer(timer)
        .with_filter(file_filter);

    let subscriber = tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    // Keep WorkerGuard alive for file logging.
    *guard_slot = Some(worker_guard);

    Ok(())
}
