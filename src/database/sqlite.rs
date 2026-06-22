use std::collections::HashMap;

use rusqlite::Connection;
use serde_json;

use crate::config::SqliteConfig;

use super::{BackendType, DatabaseBackend};

pub struct SqliteBackend {
    conn: Connection,
}

impl SqliteBackend {
    pub fn new(cfg: &SqliteConfig) -> Self {
        let conn = Connection::open(&cfg.path).expect("Failed to open database");
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .expect("Failed to set WAL mode");
        conn.execute_batch("PRAGMA synchronous=NORMAL")
            .expect("Failed to set synchronous mode");
        let backend = Self { conn };
        backend.init_db();
        backend
    }

    fn init_db(&self) {
        println!("[sqlite] Checking database table structure...");
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS daily_stats (
                date TEXT PRIMARY KEY,
                data TEXT NOT NULL
            )",
            )
            .expect("Failed to create table");
        println!("[sqlite] Database initialization complete.");
    }
}

impl DatabaseBackend for SqliteBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Sqlite
    }

    fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT data FROM daily_stats WHERE date = ?1")
            .expect("Failed to prepare SELECT statement");
        let result: Option<String> = stmt.query_row([date_str], |row| row.get(0)).ok();
        match result {
            Some(json_str) => serde_json::from_str(&json_str).unwrap_or_default(),
            None => HashMap::new(),
        }
    }

    fn get_stats_for_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> HashMap<String, u64> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT data FROM daily_stats WHERE date BETWEEN ?1 AND ?2")
            .expect("Failed to prepare range SELECT");
        let results = stmt
            .query_map([start_date, end_date], |row| row.get::<_, String>(0))
            .expect("Failed to query range data");

        let mut aggregated = HashMap::new();
        for result in results {
            let Ok(json_str) = result else { continue };
            let day_data: HashMap<String, u64> =
                serde_json::from_str(&json_str).unwrap_or_default();
            for (key, value) in day_data {
                *aggregated.entry(key).or_insert(0) += value;
            }
        }
        aggregated
    }

    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        let json_str = serde_json::to_string(data).expect("Failed to serialize stats to JSON");
        self.conn
            .execute(
                "INSERT OR REPLACE INTO daily_stats (date, data) VALUES (?1, ?2)",
                [date_str, &json_str],
            )
            .expect("Failed to upsert daily stats");
    }

    fn export_to_json(&self) -> String {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT date, data FROM daily_stats ORDER BY date")
            .expect("Failed to prepare export SELECT");
        let results = stmt
            .query_map([], |row| {
                let date: String = row.get(0)?;
                let data: String = row.get(1)?;
                Ok((date, data))
            })
            .expect("Failed to query export data");

        let mut records = serde_json::Map::new();
        for result in results {
            let Ok((date, json_str)) = result else { continue };
            let parsed: serde_json::Value =
                serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);
            records.insert(date, parsed);
        }

        let export = serde_json::json!({
            "backend": "sqlite",
            "exported_at": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            "records": records,
        });

        serde_json::to_string_pretty(&export).expect("Failed to serialize export JSON")
    }

    fn import_from_json(&mut self, json_str: &str) {
        let value: serde_json::Value =
            serde_json::from_str(json_str).expect("Failed to parse import JSON");

        let records = value
            .get("records")
            .and_then(|v| v.as_object())
            .expect("Import JSON missing 'records' object");

        for (date, data_value) in records {
            let data: HashMap<String, u64> =
                serde_json::from_value(data_value.clone()).unwrap_or_default();
            self.upsert_day_stats(date, &data);
        }

        println!(
            "[sqlite] Imported {} date records from JSON.",
            records.len()
        );
    }
}
