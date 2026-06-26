use std::fs;
use std::path::{Path, PathBuf};

use tracing_appender::rolling;
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
        write!(w, "{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

fn parse_rotation(s: &str) -> rolling::Rotation {
    match s.to_lowercase().as_str() {
        "hourly" => rolling::Rotation::HOURLY,
        "never" => rolling::Rotation::NEVER,
        "minutely" => rolling::Rotation::MINUTELY,
        _ => rolling::Rotation::DAILY,
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

    let log_dir = log_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_str()
        .unwrap_or(".");
    let log_name = log_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("monitor.log");

    let rotation = parse_rotation(&config.rotation);
    let file_appender = rolling::RollingFileAppender::new(rotation, log_dir, log_name);

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
        .with_writer(file_appender)
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
