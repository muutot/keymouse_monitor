use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Local;
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::Registry;

use crate::config::LogConfig;

struct LocalTimer;
impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Rotation {
    Never,
    Minutely,
    Hourly,
    Daily,
}

fn parse_rotation(s: &str) -> Rotation {
    match s.to_lowercase().as_str() {
        "hourly" => Rotation::Hourly,
        "never" => Rotation::Never,
        "minutely" => Rotation::Minutely,
        _ => Rotation::Daily,
    }
}

fn date_suffix(rotation: Rotation) -> String {
    let now = Local::now();
    match rotation {
        Rotation::Never => String::new(),
        Rotation::Daily => format!(".{}", now.format("%Y-%m-%d")),
        Rotation::Hourly => format!(".{}", now.format("%Y-%m-%d-%H")),
        Rotation::Minutely => format!(".{}", now.format("%Y-%m-%d-%H-%M")),
    }
}

#[derive(Clone)]
struct RollingFileWriter {
    inner: std::sync::Arc<Mutex<RollingState>>,
}

struct RollingState {
    dir: PathBuf,
    stem: String,
    ext: String,
    rotation: Rotation,
    file: Option<(String, fs::File)>,
}

impl RollingFileWriter {
    fn new(rotation: Rotation, dir: PathBuf, stem: String, ext: String) -> Self {
        Self {
            inner: std::sync::Arc::new(Mutex::new(RollingState {
                dir,
                stem,
                ext,
                rotation,
                file: None,
            })),
        }
    }
}

impl Write for RollingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self.inner.lock().unwrap();
        let suffix = date_suffix(state.rotation);

        if state.file.as_ref().map_or(true, |(cur, _)| *cur != suffix) {
            let filename = if suffix.is_empty() {
                format!("{}.{}", state.stem, state.ext)
            } else {
                format!("{}{}.{}", state.stem, suffix, state.ext)
            };
            let path = state.dir.join(&filename);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;

            state.file = Some((suffix, file));
        }

        state.file.as_mut().unwrap().1.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self.inner.lock().unwrap();
        if let Some((_, ref mut file)) = state.file {
            file.flush()
        } else {
            Ok(())
        }
    }
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn init_logger(config: &LogConfig) {
    let log_path = if Path::new(&config.file).is_relative() {
        exe_dir().join(&config.file)
    } else {
        PathBuf::from(&config.file)
    };

    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let rotation = parse_rotation(&config.rotation);
    let dir = log_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = log_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("monitor")
        .to_string();
    let ext = log_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("log")
        .to_string();

    let file_appender = RollingFileWriter::new(rotation, dir, stem, ext);
    let make_writer = move || file_appender.clone();

    let use_app_only = config.level.to_lowercase() != "trace";
    let default_directive = if use_app_only { "info" } else { "trace" };
    let filter = EnvFilter::builder()
        .with_default_directive(default_directive.parse().unwrap())
        .from_env_lossy();
    let filter = if use_app_only {
        filter.add_directive(
            format!("keymouse_monitor={}", config.level)
                .parse()
                .unwrap(),
        )
    } else {
        filter
    };

    let file_layer = fmt::layer()
        .with_timer(LocalTimer)
        .with_target(false)
        .with_writer(make_writer)
        .with_ansi(false)
        .with_filter(filter.clone());

    let subscriber = Registry::default().with(file_layer);

    if config.console {
        let console_layer = fmt::layer()
            .with_timer(LocalTimer)
            .with_target(false)
            .with_writer(std::io::stdout)
            .with_filter(filter);

        subscriber.with(console_layer).init();
    } else {
        subscriber.init();
    }
}

#[macro_export]
macro_rules! tinfo {
    ($module:expr, $($arg:tt)*) => {
        ::tracing::info!("[{}] {}", $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! terror {
    ($module:expr, $($arg:tt)*) => {
        ::tracing::error!("[{}] {}", $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! twarn {
    ($module:expr, $($arg:tt)*) => {
        ::tracing::warn!("[{}] {}", $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! tdebug {
    ($module:expr, $($arg:tt)*) => {
        ::tracing::debug!("[{}] {}", $module, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! ttrace {
    ($module:expr, $($arg:tt)*) => {
        ::tracing::trace!("[{}] {}", $module, format_args!($($arg)*))
    };
}
