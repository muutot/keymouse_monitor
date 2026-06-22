use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfig {
    pub path: String,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            path: "monitor.sqlite".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MongoConfig {
    pub uri: String,
    pub database: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub auth_source: Option<String>,
}

impl Default for MongoConfig {
    fn default() -> Self {
        Self {
            uri: "mongodb://localhost:27017".to_string(),
            database: "keymouse_monitor".to_string(),
            username: None,
            password: None,
            auth_source: None,
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
            println!("  mongodb.uri: {}", cfg.database.mongodb.uri);
            println!("  mongodb.database: {}", cfg.database.mongodb.database);
            println!("  port: {}", cfg.port);
            cfg
        }
    }
}
