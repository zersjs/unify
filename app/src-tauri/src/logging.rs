use std::path::Path;
use std::time::Duration;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, reload, EnvFilter, Registry};
use unify_core::config::LogLevel;

const MAX_LOG_FILES: usize = 7;
const PANIC_FLUSH_DELAY: Duration = Duration::from_millis(200);

pub struct LogControl {
    handle: reload::Handle<EnvFilter, Registry>,
    _guard: WorkerGuard,
}

fn filter_for(level: LogLevel) -> EnvFilter {
    EnvFilter::new(format!("{},mdns_sd=warn,tao=warn,wry=warn,rustls=warn", level.as_filter()))
}

fn install_panic_log() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(panic = %info, "Unify panicked");
        std::thread::sleep(PANIC_FLUSH_DELAY);
        previous(info);
    }));
}

impl LogControl {
    pub fn init(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(dir)?;
        let appender = Builder::new()
            .rotation(Rotation::DAILY)
            .filename_prefix("unify")
            .filename_suffix("log")
            .max_log_files(MAX_LOG_FILES)
            .build(dir)?;
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let (filter, handle) = reload::Layer::new(filter_for(LogLevel::Info));
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json().with_ansi(false).with_writer(writer).with_current_span(false))
            .try_init()?;
        install_panic_log();
        Ok(Self { handle, _guard: guard })
    }

    pub fn set_level(&self, level: LogLevel) {
        if let Err(err) = self.handle.reload(filter_for(level)) {
            tracing::warn!(error = %err, "log level could not be changed");
        }
    }
}
