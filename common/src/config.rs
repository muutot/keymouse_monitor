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
    #[serde(default = "default_auth_source", rename = "authSource")]
    pub auth_source: String,
    #[serde(default = "default_ssl")]
    pub ssl: bool,
    #[serde(default, rename = "replicaSet")]
    pub replica_set: Option<String>,
    #[serde(default, rename = "appName")]
    pub app_name: Option<String>,
    #[serde(default)]
    pub hosts: Option<Vec<String>>,
    #[serde(default = "default_connect_timeout_ms", rename = "connectTimeoutMs")]
    pub connect_timeout_ms: u64,
    #[serde(default = "default_server_selection_timeout_ms", rename = "serverSelectionTimeoutMs")]
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
