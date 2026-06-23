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

}

pub trait DatabaseBackend: Send {
    fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64>;
    fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> HashMap<String, u64>;
    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>);
    fn merge_incremental_stats(&self, date_str: &str, data: &HashMap<String, u64>);
    fn export_to_json(&self, format: &str) -> String;
    fn import_from_json(&mut self, json_str: &str, mode: ImportMode);
    fn backend_type(&self) -> BackendType;
}

pub struct Database {
    inner: Box<dyn DatabaseBackend>,
}

impl Database {
    pub fn new_sqlite(cfg: &SqliteConfig, _use_server_aggregation: bool) -> Self {
        Database {
            inner: Box::new(sqlite::SqliteBackend::new(cfg)),
        }
    }

    pub fn new_mongodb(cfg: &MongoConfig, _use_server_aggregation: bool) -> Self {
        Database {
            inner: Box::new(mongodb::MongoBackend::new(cfg)),
        }
    }

    pub fn connect(db_cfg: &DatabaseConfig) -> Self {
        let backend = BackendType::from_str(&db_cfg.backend);
        match backend {
            BackendType::Sqlite => Self::new_sqlite(&db_cfg.sqlite, false),
            BackendType::MongoDb => Self::new_mongodb(&db_cfg.mongodb, false),
        }
    }

    #[allow(dead_code)]
    pub fn new(db_file: &str) -> Self {
        Self::new_sqlite(
            &SqliteConfig {
                path: db_file.to_string(),
                table: "daily_stats".to_string(),
            },
            false,
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

    pub fn merge_incremental_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        self.inner.merge_incremental_stats(date_str, data)
    }

    pub fn export_to_json(&self, format: &str) -> String {
        self.inner.export_to_json(format)
    }

    pub fn import_from_json(&mut self, json_str: &str, mode: ImportMode) {
        self.inner.import_from_json(json_str, mode)
    }

    #[allow(dead_code)]
    pub fn backend_type(&self) -> BackendType {
        self.inner.backend_type()
    }
}
