use std::collections::HashMap;
use std::time::Instant;

use rusqlite::Connection;

use crate::{tinfo, twarn, tdebug};
use crate::config::SqliteConfig;

use super::{BackendType, DatabaseBackend, ImportMode};

pub struct SqliteBackend {
    conn: Connection,
    table_name: String,
}

impl SqliteBackend {
    pub fn new(cfg: &SqliteConfig) -> Self {
        let conn = Connection::open(&cfg.path).expect("Failed to open database");
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .expect("Failed to set WAL mode");
        conn.execute_batch("PRAGMA synchronous=NORMAL")
            .expect("Failed to set synchronous mode");
        let table_name = cfg.table.clone();
        let backend = Self { conn, table_name };
        backend.init_db();
        backend
    }

    fn init_db(&self) {
        tinfo!("sqlite", "Checking database table structure...");

        // Check if old nested-format table exists
        let has_old: bool = {
            let mut stmt = self
                .conn
                .prepare_cached(
                    "SELECT COUNT(*) FROM sqlite_master \
                     WHERE type='table' AND name=?1",
                )
                .expect("Failed to prepare schema check");
            stmt.query_row([&self.table_name], |row| row.get::<_, i64>(0))
                .unwrap_or(0)
                > 0
        };

        if has_old {
            // Check if it's the old schema (has 'data' column but no 'key')
            let is_old: bool = {
                let mut stmt = self
                    .conn
                    .prepare_cached(&format!("PRAGMA table_info({})", self.table_name))
                    .expect("Failed to prepare pragma");
                let columns: Vec<String> = stmt
                    .query_map([], |row| row.get::<_, String>(1))
                    .expect("Failed to query pragma")
                    .filter_map(|r| r.ok())
                    .collect();
                columns.contains(&"data".to_string())
            };

            if is_old {
                tinfo!("sqlite", "Migrating old nested format to flat format...");
                let tmp_table = format!("{}_new", self.table_name);
                let create_sql = format!(
                    "CREATE TABLE {} (date TEXT, key TEXT, count INTEGER, \
                     PRIMARY KEY (date, key))",
                    tmp_table
                );
                self.conn.execute_batch(&create_sql)
                    .expect("Failed to create new table");

                // Copy old data: parse JSON blob and flatten
                let mut stmt = self
                    .conn
                    .prepare_cached(&format!(
                        "SELECT date, data FROM {}", self.table_name
                    ))
                    .expect("Failed to prepare old data SELECT");
                let results: Vec<(String, String)> = stmt
                    .query_map([], |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                    })
                    .expect("Failed to query old data")
                    .filter_map(|r| r.ok())
                    .collect();

                let mut insert_stmt = self
                    .conn
                    .prepare_cached(&format!(
                        "INSERT OR REPLACE INTO {} (date, key, count) VALUES (?1, ?2, ?3)",
                        tmp_table
                    ))
                    .expect("Failed to prepare insert");

                for (date, data_json) in &results {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, u64>>(data_json) {
                        for (key, count) in map {
                            insert_stmt
                                .execute([date.as_str(), &key, &count.to_string()])
                                .expect("Failed to insert migrated data");
                        }
                    }
                }

                // Drop old table, rename new
                self.conn
                    .execute_batch(&format!(
                        "DROP TABLE {}; ALTER TABLE {} RENAME TO {}",
                        self.table_name, tmp_table, self.table_name
                    ))
                    .expect("Failed to rename table");

                tinfo!("sqlite", "Migration complete.");
            }
        } else {
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (\
                 date TEXT, key TEXT, count INTEGER, \
                 PRIMARY KEY (date, key))",
                self.table_name
            );
            self.conn.execute_batch(&sql).expect("Failed to create table");
        }

        tinfo!("sqlite", "Database initialization complete.");
    }
}

impl DatabaseBackend for SqliteBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Sqlite
    }

    fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        let sql = format!(
            "SELECT key, count FROM {} WHERE date = ?1",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&sql)
            .expect("Failed to prepare SELECT");
        let results = stmt
            .query_map([date_str], |row| {
                let key: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((key, count as u64))
            })
            .expect("Failed to query day stats");
        let mut map = HashMap::new();
        for r in results {
            if let Ok((key, count)) = r {
                map.insert(key, count);
            }
        }
        map
    }

    fn get_stats_for_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> HashMap<String, u64> {
        let sql = format!(
            "SELECT key, SUM(count) FROM {} \
             WHERE date BETWEEN ?1 AND ?2 GROUP BY key",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&sql)
            .expect("Failed to prepare aggregation SELECT");
        let results = stmt
            .query_map([start_date, end_date], |row| {
                let key: String = row.get(0)?;
                let value: i64 = row.get(1)?;
                Ok((key, value as u64))
            })
            .expect("Failed to query aggregation data");
        let mut aggregated = HashMap::new();
        for result in results {
            if let Ok((key, value)) = result {
                aggregated.insert(key, value);
            }
        }
        aggregated
    }

    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        let t0 = Instant::now();
        let key_count = data.len();

        if data.is_empty() {
            // Delete any lingering rows for empty data
            let delete_sql = format!("DELETE FROM {} WHERE date = ?1", self.table_name);
            self.conn.execute(&delete_sql, [date_str]).expect("Failed to delete");
            tdebug!("sqlite", "upsert_day_stats({}): delete only (empty), total={:?}", date_str, t0.elapsed());
            return;
        }

        let upsert_sql = format!(
            "INSERT OR REPLACE INTO {} (date, key, count) VALUES (?1, ?2, ?3)",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&upsert_sql)
            .expect("Failed to prepare upsert");
        for (key, count) in data {
            stmt.execute([date_str, key, &count.to_string()])
                .expect("Failed to upsert day stat");
        }
        let elapsed = t0.elapsed();

        tdebug!("sqlite",
            "upsert_day_stats({}): upsert={:?} ({} keys)",
            date_str,
            elapsed,
            key_count,
        );
    }

    fn export_to_json(&self, format: &str) -> String {
        let sql = format!(
            "SELECT date, key, count FROM {} ORDER BY date, key",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&sql)
            .expect("Failed to prepare export SELECT");
        let results: Vec<(String, String, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .expect("Failed to query export data")
            .filter_map(|r| r.ok())
            .collect();

        match format {
            "flat" => {
                let records: Vec<serde_json::Value> = results
                    .iter()
                    .map(|(date, key, count)| {
                        serde_json::json!({
                            "date": date,
                            "key": key,
                            "count": count,
                        })
                    })
                    .collect();

                let export = serde_json::json!({
                    "backend": "sqlite",
                    "exported_at": chrono::Local::now()
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string(),
                    "records": records,
                });
                serde_json::to_string_pretty(&export).expect("Failed to serialize export JSON")
            }
            _ => {
                let mut records = serde_json::Map::new();
                for (date, key, count) in &results {
                    let entry = records
                        .entry(date.clone())
                        .or_insert_with(|| serde_json::json!({}));
                    if let Some(obj) = entry.as_object_mut() {
                        obj.insert(key.clone(), serde_json::json!(count));
                    }
                }

                let export = serde_json::json!({
                    "backend": "sqlite",
                    "exported_at": chrono::Local::now()
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string(),
                    "records": records,
                });
                serde_json::to_string_pretty(&export).expect("Failed to serialize export JSON")
            }
        }
    }

    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .expect("Failed to parse import JSON");

        let records_map: HashMap<String, HashMap<String, u64>> = match value["records"] {
            serde_json::Value::Object(ref obj) => {
                let mut map = HashMap::new();
                for (date, data_value) in obj {
                    let data: HashMap<String, u64> =
                        serde_json::from_value(data_value.clone()).unwrap_or_default();
                    map.insert(date.clone(), data);
                }
                if map.is_empty() {
                    twarn!("sqlite", "Import JSON contains 0 records, skipping.");
                    return;
                }
                map
            }
            serde_json::Value::Array(ref arr) => {
                let mut map: HashMap<String, HashMap<String, u64>> = HashMap::new();
                for item in arr {
                    let date = item["date"].as_str().map(String::from);
                    let key = item["key"].as_str().map(String::from);
                    let count = item["count"].as_i64().unwrap_or(0) as u64;
                    if let (Some(date), Some(key)) = (date, key) {
                        map.entry(date).or_default().insert(key, count);
                    }
                }
                if map.is_empty() {
                    twarn!("sqlite", "Import JSON contains 0 records, skipping.");
                    return;
                }
                map
            }
            _ => {
                panic!("Import JSON 'records' must be an object (nested) or array (flat)");
            }
        };

        let total = records_map.len();
        let dates: Vec<&str> = records_map.keys().map(|s| s.as_str()).collect();

        // Pre-read existing for merge mode
        let existing_map: HashMap<String, HashMap<String, u64>> = if mode == ImportMode::Merge {
            let mut map = HashMap::new();
            for date in &dates {
                let data = self.get_stats_for_day(date);
                if !data.is_empty() {
                    map.insert(date.to_string(), data);
                }
            }
            map
        } else {
            HashMap::new()
        };

        let delete_sql = format!("DELETE FROM {} WHERE date = ?1", self.table_name);
        let insert_sql = format!(
            "INSERT INTO {} (date, key, count) VALUES (?1, ?2, ?3)",
            self.table_name
        );

        let tx = self.conn.transaction()
            .expect("Failed to begin transaction");

        {
            let mut del_stmt = tx.prepare_cached(&delete_sql)
                .expect("Failed to prepare DELETE");
            let mut ins_stmt = tx.prepare_cached(&insert_sql)
                .expect("Failed to prepare INSERT");

            for (date, incoming) in &records_map {
                del_stmt.execute([date.as_str()])
                    .expect("Failed to delete existing records");

                let data = if mode == ImportMode::Merge {
                    let mut merged = existing_map.get(date).cloned().unwrap_or_default();
                    for (k, v) in incoming {
                        *merged.entry(k.clone()).or_insert(0) += v;
                    }
                    merged
                } else {
                    incoming.clone()
                };

                for (key, count) in &data {
                    ins_stmt.execute([date.as_str(), key, &count.to_string()])
                        .expect("Failed to insert record");
                }
            }
        }

        tx.commit().expect("Failed to commit transaction");

        tinfo!("sqlite",
            "Imported {} date records from JSON (mode: {:?}).",
            total, mode
        );
    }
}
