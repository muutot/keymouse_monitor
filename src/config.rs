use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    pub path: String,
    #[serde(default = "default_sqlite_table")]
    pub table: String,
}

fn default_sqlite_table() -> String {
    "daily_stats".to_string()
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: "monitor.sqlite".to_string(),
            table: default_sqlite_table(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoConfig {
    #[serde(default = "default_mongo_protocol")]
    pub protocol: String,
    pub database: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "default_auth_source")]
    pub auth_source: String,
    #[serde(default = "default_ssl")]
    pub ssl: bool,
    #[serde(default)]
    pub replica_set: Option<String>,
    #[serde(default)]
    pub app_name: Option<String>,
    #[serde(default)]
    pub hosts: Option<Vec<String>>,
    #[serde(default = "default_connect_timeout_ms")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_server_selection_timeout_ms")]
    pub server_selection_timeout_ms: u64,
    #[serde(default = "default_mongo_collection")]
    pub collection: String,
}

fn default_mongo_collection() -> String {
    "daily_stats".to_string()
}

fn default_mongo_protocol() -> String {
    "mongodb".to_string()
}

fn default_auth_source() -> String {
    "admin".to_string()
}

fn default_ssl() -> bool {
    true
}

fn default_connect_timeout_ms() -> u64 {
    15000
}

fn default_server_selection_timeout_ms() -> u64 {
    30000
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            protocol: default_mongo_protocol(),
            database: "keymouse_monitor".to_string(),
            username: None,
            password: None,
            auth_source: default_auth_source(),
            ssl: default_ssl(),
            replica_set: None,
            app_name: None,
            hosts: None,
            connect_timeout_ms: default_connect_timeout_ms(),
            server_selection_timeout_ms: default_server_selection_timeout_ms(),
            collection: default_mongo_collection(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Which backend to use: "sqlite" or "mongodb"
    pub backend: String,
    pub sqlite: SqliteConfig,
    pub mongodb: MongoConfig,
    #[serde(default = "default_server_aggregation")]
    pub use_server_aggregation: bool,
}

fn default_server_aggregation() -> bool {
    true
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".to_string(),
            sqlite: SqliteConfig::default(),
            mongodb: MongoConfig::default(),
            use_server_aggregation: default_server_aggregation(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub port: u16,
    pub listener: String,
    #[serde(default = "default_save_interval")]
    pub save_interval_secs: u64,
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
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Path::new("config.json");
        if path.exists() {
            let content = std::fs::read_to_string("config.json").unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|e| {
                let cfg = Self::default();
                eprintln!("Failed to parse config.json ({}), using defaults", e);
                cfg
            })
        } else {
            let cfg = Self::default();
            println!("No config.json found, using default configuration");
            println!("  backend: {}", cfg.database.backend);
            println!("  sqlite.path: {}", cfg.database.sqlite.path);
            println!("  mongodb.protocol: {}", cfg.database.mongodb.protocol);
            println!("  mongodb.database: {}", cfg.database.mongodb.database);
            println!("  port: {}", cfg.port);
            cfg
        }
    }
}
