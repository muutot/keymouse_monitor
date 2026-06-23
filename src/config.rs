use std::path::PathBuf;

use crate::{tinfo, twarn};
use serde::{Deserialize, Serialize};
pub use keymouse_common::config::{DatabaseConfig, MongoConfig, SqliteConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub port: u16,
    pub listener: String,
    #[serde(default = "default_save_interval")]
    pub save_interval_secs: u64,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_file")]
    pub file: String,
    #[serde(default = "default_log_rotation")]
    pub rotation: String,
    #[serde(default = "default_log_console")]
    pub console: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_file() -> String {
    "logs/monitor.log".to_string()
}
fn default_log_rotation() -> String {
    "daily".to_string()
}
fn default_log_console() -> bool {
    true
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: default_log_file(),
            rotation: default_log_rotation(),
            console: default_log_console(),
        }
    }
}

fn default_save_interval() -> u64 {
    60
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            port: 5000,
            #[cfg(windows)]
            listener: "rawinput".to_string(),
            #[cfg(not(windows))]
            listener: "rdev".to_string(),
            save_interval_secs: default_save_interval(),
            log: LogConfig::default(),
        }
    }
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Config {
    pub fn load() -> Self {
        let path = exe_dir().join("config.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|e| {
                let cfg = Self::default();
                twarn!("config", "Failed to parse config.json ({}), using defaults", e);
                cfg
            })
        } else {
            let cfg = Self::default();
            tinfo!("config", "No config.json found, using default configuration");
            tinfo!("config", "  backend: {}", cfg.database.backend);
            tinfo!("config", "  sqlite.path: {}", cfg.database.sqlite.path);
            tinfo!("config", "  mongodb.protocol: {}", cfg.database.mongodb.protocol);
            tinfo!("config", "  mongodb.database: {}", cfg.database.mongodb.database);
            tinfo!("config", "  port: {}", cfg.port);
            cfg
        }
    }
}
