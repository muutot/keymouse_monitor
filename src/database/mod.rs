use std::collections::HashMap;

use crate::config::{DatabaseConfig, MongoConfig, SqliteConfig};

mod mongodb;
mod sqlite;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendType {
    Sqlite,
    MongoDb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportMode {
    Overwrite,
    Merge,
}

impl ImportMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "merge" | "叠加" => ImportMode::Merge,
            _ => ImportMode::Overwrite,
        }
    }
}

impl BackendType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mongodb" | "mongo" => BackendType::MongoDb,
            _ => BackendType::Sqlite,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BackendType::Sqlite => "sqlite",
            BackendType::MongoDb => "mongodb",
        }
    }
}

pub trait DatabaseBackend: Send {
    fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64>;
    fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> HashMap<String, u64>;
    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>);
    fn export_to_json(&self) -> String;
    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) {
        let value: serde_json::Value =
            serde_json::from_str(json_str).expect("Failed to parse import JSON");
        let records = value
            .get("records")
            .and_then(|v| v.as_object())
            .expect("Import JSON missing 'records' object");
        for (date, data_value) in records {
            let data: HashMap<String, u64> =
                serde_json::from_value(data_value.clone()).unwrap_or_default();
            match mode {
                ImportMode::Overwrite => self.upsert_day_stats(date, &data),
                ImportMode::Merge => {
                    let mut existing = self.get_stats_for_day(date);
                    for (k, v) in data {
                        *existing.entry(k).or_insert(0) += v;
                    }
                    self.upsert_day_stats(date, &existing);
                }
            }
        }
        println!(
            "[{}] Imported {} date records from JSON (mode: {:?}).",
            self.backend_type().as_str(),
            records.len(),
            mode
        );
    }
    #[allow(dead_code)]
    fn backend_type(&self) -> BackendType;
}

pub struct Database {
    inner: Box<dyn DatabaseBackend>,
}

impl Database {
    pub fn new_sqlite(cfg: &SqliteConfig, use_server_aggregation: bool) -> Self {
        Database {
            inner: Box::new(sqlite::SqliteBackend::new(cfg, use_server_aggregation)),
        }
    }

    pub fn new_mongodb(cfg: &MongoConfig, use_server_aggregation: bool) -> Self {
        Database {
            inner: Box::new(mongodb::MongoBackend::new(cfg, use_server_aggregation)),
        }
    }

    pub fn connect(db_cfg: &DatabaseConfig) -> Self {
        let backend = BackendType::from_str(&db_cfg.backend);
        let use_agg = db_cfg.use_server_aggregation;
        match backend {
            BackendType::Sqlite => Self::new_sqlite(&db_cfg.sqlite, use_agg),
            BackendType::MongoDb => Self::new_mongodb(&db_cfg.mongodb, use_agg),
        }
    }

    /// Create a SQLite database from a file path (for tests and convenience).
    #[allow(dead_code)]
    pub fn new(db_file: &str) -> Self {
        Self::new_sqlite(
            &SqliteConfig {
                path: db_file.to_string(),
                table: "daily_stats".to_string(),
            },
            true,
        )
    }

    pub fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        self.inner.get_stats_for_day(date_str)
    }

    pub fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> HashMap<String, u64> {
        self.inner.get_stats_for_range(start_date, end_date)
    }

    pub fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        self.inner.upsert_day_stats(date_str, data)
    }

    pub fn export_to_json(&self) -> String {
        self.inner.export_to_json()
    }

    pub fn import_from_json(&mut self, json_str: &str, mode: ImportMode) {
        self.inner.import_from_json(json_str, mode)
    }

    #[allow(dead_code)]
    pub fn backend_type(&self) -> BackendType {
        self.inner.backend_type()
    }
}
