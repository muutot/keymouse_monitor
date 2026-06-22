use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub db_file: String,
    pub port: u16,
    pub listener: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_file: "monitor.sqlite".to_string(),
            port: 5000,
            #[cfg(windows)]
            listener: "rawinput".to_string(),
            #[cfg(not(windows))]
            listener: "rdev".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Path::new("config.json");
        if path.exists() {
            let content = std::fs::read_to_string("config.json").unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_else(|_| {
                let cfg = Self::default();
                eprintln!("Failed to parse config.json, using defaults");
                cfg
            })
        } else {
            let cfg = Self::default();
            println!("No config.json found, using default configuration");
            println!("  db_file: {}", cfg.db_file);
            println!("  port: {}", cfg.port);
            cfg
        }
    }
}
