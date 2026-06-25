use std::collections::HashMap;
use std::time::Instant;

use rusqlite::Connection;

use crate::{tinfo, twarn, tdebug, config::SqliteConfig};

use super::{BackendType, DatabaseBackend, ImportMode};

pub struct SqliteBackend {
    conn: Connection,
    pub(crate) table_name: String,
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

    /// Get distinct dates that have data (used for fallback sync).
    pub fn get_dates(&self) -> Result<Vec<String>, String> {
        let sql = format!("SELECT DISTINCT date FROM {} ORDER BY date", self.table_name);
        let mut stmt = self.conn.prepare_cached(&sql)
            .map_err(|e| format!("prepare dates: {e}"))?;
        let dates = stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| format!("query dates: {e}"))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(dates)
    }

    /// Delete all data (used after syncing fallback to primary).
    pub fn clear_all(&self) -> Result<(), String> {
        let sql = format!("DELETE FROM {}", self.table_name);
        self.conn.execute(&sql, [])
            .map_err(|e| format!("clear fallback: {e}"))?;
        Ok(())
    }
}

impl DatabaseBackend for SqliteBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::Sqlite
    }

    fn try_ping(&self) -> Result<(), String> {
        self.conn.execute_batch("SELECT 1")
            .map_err(|e| format!("sqlite ping: {e}"))
    }

    fn get_stats_for_day(&self, date_str: &str) -> Result<HashMap<String, u64>, String> {
        let sql = format!(
            "SELECT key, count FROM {} WHERE date = ?1",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&sql)
            .map_err(|e| format!("prepare select: {e}"))?;
        let results = stmt
            .query_map([date_str], |row| {
                let key: String = row.get(0)?;
                let count: i64 = row.get(1)?;
                Ok((key, count as u64))
            })
            .map_err(|e| format!("query day stats: {e}"))?;
        let mut map = HashMap::new();
        for r in results {
            let (key, count) = r.map_err(|e| format!("row: {e}"))?;
            map.insert(key, count);
        }
        Ok(map)
    }

    fn get_stats_for_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<HashMap<String, u64>, String> {
        let sql = format!(
            "SELECT key, SUM(count) FROM {} \
             WHERE date BETWEEN ?1 AND ?2 GROUP BY key",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&sql)
            .map_err(|e| format!("prepare agg: {e}"))?;
        let results = stmt
            .query_map([start_date, end_date], |row| {
                let key: String = row.get(0)?;
                let value: i64 = row.get(1)?;
                Ok((key, value as u64))
            })
            .map_err(|e| format!("query agg: {e}"))?;
        let mut aggregated = HashMap::new();
        for r in results {
            let (key, value) = r.map_err(|e| format!("row: {e}"))?;
            aggregated.insert(key, value);
        }
        Ok(aggregated)
    }

    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) -> Result<(), String> {
        let t0 = Instant::now();
        let key_count = data.len();

        if data.is_empty() {
            let delete_sql = format!("DELETE FROM {} WHERE date = ?1", self.table_name);
            self.conn.execute(&delete_sql, [date_str])
                .map_err(|e| format!("delete empty: {e}"))?;
            tdebug!("sqlite", "upsert_day_stats({}): delete only (empty), total={:?}", date_str, t0.elapsed());
            return Ok(());
        }

        let tx = self.conn.unchecked_transaction()
            .map_err(|e| format!("begin tx: {e}"))?;
        let delete_sql = format!("DELETE FROM {} WHERE date = ?1", self.table_name);
        tx.execute(&delete_sql, [date_str])
            .map_err(|e| format!("delete: {e}"))?;
        let upsert_sql = format!(
            "INSERT OR REPLACE INTO {} (date, key, count) VALUES (?1, ?2, ?3)",
            self.table_name
        );
        {
            let mut stmt = tx.prepare_cached(&upsert_sql)
                .map_err(|e| format!("prepare upsert: {e}"))?;
            for (key, count) in data {
                stmt.execute([date_str, key, &count.to_string()])
                    .map_err(|e| format!("upsert day stat: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("commit: {e}"))?;
        let elapsed = t0.elapsed();

        tdebug!("sqlite",
            "upsert_day_stats({}): tx={:?} ({} keys)",
            date_str,
            elapsed,
            key_count,
        );
        Ok(())
    }

    fn merge_incremental_stats(&self, date_str: &str, data: &HashMap<String, u64>) -> Result<(), String> {
        let t0 = Instant::now();
        let key_count = data.len();

        if data.is_empty() {
            tdebug!("sqlite", "merge_incremental_stats({}): empty, nothing to do", date_str);
            return Ok(());
        }

        let tx = self.conn.unchecked_transaction()
            .map_err(|e| format!("begin tx: {e}"))?;
        let upsert_sql = format!(
            "INSERT INTO {} (date, key, count) VALUES (?1, ?2, ?3) \
             ON CONFLICT(date, key) DO UPDATE SET count = count + excluded.count",
            self.table_name
        );
        {
            let mut stmt = tx.prepare_cached(&upsert_sql)
                .map_err(|e| format!("prepare inc upsert: {e}"))?;
            for (key, count) in data {
                stmt.execute([date_str, key, &count.to_string()])
                    .map_err(|e| format!("merge inc stat: {e}"))?;
            }
        }
        tx.commit().map_err(|e| format!("commit: {e}"))?;
        let elapsed = t0.elapsed();

        tdebug!("sqlite",
            "merge_incremental_stats({}): tx={:?} ({} keys)",
            date_str,
            elapsed,
            key_count,
        );
        Ok(())
    }

    fn export_to_json(&self, format: &str) -> Result<String, String> {
        let sql = format!(
            "SELECT date, key, count FROM {} ORDER BY date, key",
            self.table_name
        );
        let mut stmt = self
            .conn
            .prepare_cached(&sql)
            .map_err(|e| format!("prepare export: {e}"))?;
        let results: Vec<(String, String, i64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| format!("query export: {e}"))?
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
                serde_json::to_string_pretty(&export)
                    .map_err(|e| format!("serialize: {e}"))
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
                serde_json::to_string_pretty(&export)
                    .map_err(|e| format!("serialize: {e}"))
            }
        }
    }

    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) -> Result<(), String> {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| format!("parse json: {e}"))?;

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
                    return Ok(());
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
                    return Ok(());
                }
                map
            }
            _ => {
                return Err("Import JSON 'records' must be an object (nested) or array (flat)".to_string());
            }
        };

        let total = records_map.len();
        let dates: Vec<&str> = records_map.keys().map(|s| s.as_str()).collect();

        let existing_map: HashMap<String, HashMap<String, u64>> = if mode == ImportMode::Merge {
            if dates.is_empty() {
                HashMap::new()
            } else {
                let placeholders = std::iter::repeat("?").take(dates.len()).collect::<Vec<_>>().join(",");
                let sql = format!(
                    "SELECT date, key, count FROM {} WHERE date IN ({})",
                    self.table_name, placeholders
                );
                let mut stmt = self.conn.prepare_cached(&sql)
                    .map_err(|e| format!("prepare import existing: {e}"))?;
                let params: Vec<&dyn rusqlite::ToSql> =
                    dates.iter().map(|d| d as &dyn rusqlite::ToSql).collect();
                let mut map: HashMap<String, HashMap<String, u64>> = HashMap::new();
                let rows = stmt
                    .query_map(params.as_slice(), |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    })
                    .map_err(|e| format!("query import existing: {e}"))?;
                for r in rows {
                    let (date, key, count) = r.map_err(|e| format!("row: {e}"))?;
                    map.entry(date).or_default().insert(key, count as u64);
                }
                map
            }
        } else {
            HashMap::new()
        };

        let delete_sql = format!("DELETE FROM {} WHERE date = ?1", self.table_name);
        let insert_sql = format!(
            "INSERT INTO {} (date, key, count) VALUES (?1, ?2, ?3)",
            self.table_name
        );

        let tx = self.conn.transaction()
            .map_err(|e| format!("begin tx: {e}"))?;

        {
            let mut del_stmt = tx.prepare_cached(&delete_sql)
                .map_err(|e| format!("prepare delete: {e}"))?;
            let mut ins_stmt = tx.prepare_cached(&insert_sql)
                .map_err(|e| format!("prepare insert: {e}"))?;

            for (date, incoming) in &records_map {
                del_stmt.execute([date.as_str()])
                    .map_err(|e| format!("delete: {e}"))?;

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
                        .map_err(|e| format!("insert: {e}"))?;
                }
            }
        }

        tx.commit().map_err(|e| format!("commit: {e}"))?;

        tinfo!("sqlite",
            "Imported {} date records from JSON (mode: {:?}).",
            total, mode
        );
        Ok(())
    }
}
