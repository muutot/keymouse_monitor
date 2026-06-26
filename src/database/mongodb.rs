use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::{DeleteManyModel, InsertOneModel, UpdateOneModel, WriteModel};

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::{config::MongoConfig, tdebug, tinfo, twarn};
use keymouse_common::database::{build_uri, redact_credentials};

use super::{BackendType, DatabaseBackend, ExportProgress, ImportMode};

#[derive(Debug, Serialize, Deserialize)]
struct FlatStat {
    date: String,
    key: String,
    count: u64,
}

pub struct MongoBackend {
    rt: Runtime,
    client: mongodb::Client,
    db_name: String,
    collection_name: String,
}

impl MongoBackend {
    pub fn new(cfg: &MongoConfig) -> Self {
        let uri = build_uri(cfg);
        tinfo!("mongodb", "URI: {}", redact_credentials(&uri));

        let rt = Runtime::new().expect("Failed to create tokio runtime for MongoDB");

        let client = match rt.block_on(async { mongodb::Client::with_uri_str(&uri).await }) {
            Ok(c) => c,
            Err(e) => {
                let err = format!("{e}");
                twarn!("mongodb", "\n⚠ Failed to create MongoDB client: {err}");
                twarn!("mongodb", "The application will retry on each data save.");
                let client = rt
                    .block_on(async {
                        mongodb::Client::with_uri_str(
                            "mongodb://127.0.0.1:1/?serverSelectionTimeoutMS=1",
                        )
                        .await
                    })
                    .expect("placeholder URI is valid");
                return Self {
                    rt,
                    client,
                    db_name: cfg.database.clone(),
                    collection_name: cfg.collection.clone(),
                };
            }
        };

        let db_name = cfg.database.clone();
        let collection_name = cfg.collection.clone();
        let backend = Self {
            rt,
            client,
            db_name,
            collection_name,
        };
        if let Err(e) = backend.init_db() {
            twarn!("mongodb", "\n⚠ MongoDB connection failed:");
            twarn!("mongodb", "  {e}");
            twarn!("mongodb", "  \nPossible causes:");
            twarn!(
                "mongodb",
                "    • Network/firewall blocking Atlas (check your VPN/proxy)"
            );
            twarn!("mongodb", "    • IP not whitelisted in Atlas console");
            twarn!("mongodb", "    • Wrong credentials in config.json");
            twarn!(
                "mongodb",
                "  \nThe application will retry on each data save.\n"
            );
        }
        backend
    }

    fn init_db(&self) -> Result<(), String> {
        tinfo!("mongodb", "Connecting to MongoDB...");
        let db = self.client.database(&self.db_name);
        let ping_result: Result<mongodb::bson::Document, String> = self.rt.block_on(async {
            db.run_command(doc! { "ping": 1 })
                .await
                .map_err(|e| format!("{e}"))
        });
        match ping_result {
            Ok(_) => {
                tinfo!("mongodb", "Connection established.");
                let collection = self.raw_collection();
                let index = mongodb::IndexModel::builder()
                    .keys(doc! { "date": 1, "key": 1 })
                    .build();
                let _ = self
                    .rt
                    .block_on(async { collection.create_index(index).await });
                self.migrate_old_format();
                tinfo!("mongodb", "Database initialization complete.");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn migrate_old_format(&self) {
        let raw = self.raw_collection();
        let has_old = self.rt.block_on(async {
            raw.find_one(doc! { "data": { "$exists": true } })
                .await
                .ok()
                .flatten()
                .is_some()
        });
        if !has_old {
            return;
        }

        tinfo!("mongodb", "Migrating old nested format to flat format...");
        self.rt.block_on(async {
            let mut cursor = raw
                .find(doc! { "data": { "$exists": true } })
                .await
                .expect("Failed to query old format docs");

            let mut flat_docs = Vec::new();
            while let Some(old_doc) = cursor.try_next().await.unwrap_or(None) {
                let date = old_doc.get_str("date").ok().map(String::from);
                let data = old_doc.get_document("data").ok().cloned();
                if let (Some(date), Some(data)) = (date, data) {
                    for (key, value) in data.iter() {
                        if let Some(count) = value.as_i64() {
                            flat_docs.push(doc! {
                                "date": &date,
                                "key": key,
                                "count": count,
                            });
                        }
                    }
                }
            }

            if !flat_docs.is_empty() {
                raw.insert_many(flat_docs)
                    .await
                    .expect("Failed to insert migrated flat docs");
                raw.delete_many(doc! { "data": { "$exists": true } })
                    .await
                    .expect("Failed to delete old format docs");
                tinfo!("mongodb", "Migration complete.");
            }
        });
    }

    fn raw_collection(&self) -> mongodb::Collection<Document> {
        self.client
            .database(&self.db_name)
            .collection(&self.collection_name)
    }

    fn flat_collection(&self) -> mongodb::Collection<FlatStat> {
        self.client
            .database(&self.db_name)
            .collection(&self.collection_name)
    }
}

fn write_json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = std::fmt::Write::write_fmt(&mut *out, format_args!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

impl DatabaseBackend for MongoBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::MongoDb
    }

    fn try_ping(&self) -> Result<(), String> {
        let db = self.client.database(&self.db_name);
        self.rt.block_on(async {
            db.run_command(doc! { "ping": 1 })
                .await
                .map(|_| ())
                .map_err(|e| format!("{e}"))
        })
    }

    fn get_stats_for_day(&self, date_str: &str) -> Result<HashMap<String, u64>, String> {
        let collection = self.flat_collection();
        let filter = doc! { "date": date_str };
        self.rt.block_on(async {
            let mut cursor = collection
                .find(filter)
                .await
                .map_err(|e| format!("query day stats: {e}"))?;
            let mut result = HashMap::new();
            while let Some(stat) = cursor
                .try_next()
                .await
                .map_err(|e| format!("cursor: {e}"))?
            {
                result.insert(stat.key, stat.count);
            }
            Ok(result)
        })
    }

    fn get_stats_for_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> Result<HashMap<String, u64>, String> {
        let raw = self.raw_collection();
        let pipeline = vec![
            doc! { "$match": { "date": { "$gte": start_date, "$lte": end_date } } },
            doc! { "$group": { "_id": "$key", "total": { "$sum": "$count" } } },
        ];
        self.rt.block_on(async {
            let t0 = Instant::now();
            let mut cursor = raw
                .aggregate(pipeline)
                .await
                .map_err(|e| format!("aggregate range: {e}"))?;
            let t1 = Instant::now();

            let mut aggregated = HashMap::new();
            let mut count = 0u64;
            let mut network_total = Duration::ZERO;
            let mut process_total = Duration::ZERO;
            loop {
                let fetch_start = Instant::now();
                let result = cursor
                    .try_next()
                    .await
                    .map_err(|e| format!("cursor: {e}"))?;
                network_total += fetch_start.elapsed();

                let Some(doc) = result else { break };
                count += 1;

                let proc_start = Instant::now();
                if let (Some(key), Some(value)) = (
                    doc.get_str("_id").ok(),
                    doc.get_i64("total").ok().map(|v| v as u64),
                ) {
                    aggregated.insert(key.to_string(), value);
                }
                process_total += proc_start.elapsed();
            }
            let t2 = Instant::now();

            tdebug!(
                "mongodb",
                "get_stats_for_range(start={}, end={}): \
                 aggregate(server)={:?}, iterate(network)={:?}, \
                 iterate(process)={:?} ({} docs), total={:?}",
                start_date,
                end_date,
                t1 - t0,
                network_total,
                process_total,
                count,
                t2 - t0,
            );
            Ok(aggregated)
        })
    }

    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) -> Result<(), String> {
        let raw = self.raw_collection();
        let ns = raw.namespace();
        let client = &self.client;
        let t0 = Instant::now();
        let key_count = data.len();
        let is_empty = data.is_empty();
        self.rt.block_on(async {
            if data.is_empty() {
                raw.delete_many(doc! { "date": date_str })
                    .await
                    .map_err(|e| format!("delete day stats: {e}"))?;
            } else {
                let mut models: Vec<WriteModel> = Vec::with_capacity(1 + key_count);

                models.push(
                    DeleteManyModel::builder()
                        .namespace(ns.clone())
                        .filter(doc! { "date": date_str })
                        .build()
                        .into(),
                );

                for (key, count) in data.iter() {
                    models.push(
                        InsertOneModel::builder()
                            .namespace(ns.clone())
                            .document(doc! {
                                "date": date_str,
                                "key": key,
                                "count": *count as i64,
                            })
                            .build()
                            .into(),
                    );
                }

                client
                    .bulk_write(models)
                    .await
                    .map_err(|e| format!("upsert day stats: {e}"))?;
            }
            let t1 = Instant::now();

            tdebug!(
                "mongodb",
                "upsert_day_stats({}): {:?} ({} keys{})",
                date_str,
                t1 - t0,
                key_count,
                if is_empty { ", empty" } else { "" },
            );
            Ok(())
        })
    }

    fn merge_incremental_stats(
        &self,
        date_str: &str,
        data: &HashMap<String, u64>,
    ) -> Result<(), String> {
        let raw = self.raw_collection();
        let ns = raw.namespace();
        let client = &self.client;
        let t0 = Instant::now();
        let key_count = data.len();

        if data.is_empty() {
            tdebug!(
                "mongodb",
                "merge_incremental_stats({}): empty, nothing to do",
                date_str
            );
            return Ok(());
        }

        self.rt.block_on(async {
            let models: Vec<WriteModel> = data
                .iter()
                .map(|(key, count)| {
                    UpdateOneModel::builder()
                        .namespace(ns.clone())
                        .filter(doc! { "date": date_str, "key": key })
                        .update(doc! { "$inc": { "count": *count as i64 } })
                        .upsert(true)
                        .build()
                        .into()
                })
                .collect();

            client
                .bulk_write(models)
                .await
                .map_err(|e| format!("merge inc stats: {e}"))?;
            let t1 = Instant::now();

            tdebug!(
                "mongodb",
                "merge_incremental_stats({}): bulk_write={:?} ({} keys)",
                date_str,
                t1 - t0,
                key_count,
            );
            Ok(())
        })
    }

    fn export_to_json(
        &self,
        format: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        progress: &ExportProgress,
    ) -> Result<String, String> {
        let raw = self.raw_collection();

        let mut filter = doc! {};
        if let (Some(s), Some(e)) = (start_date, end_date) {
            filter = doc! { "date": { "$gte": s, "$lte": e } };
        }

        self.rt.block_on(async {
            let total = raw
                .count_documents(filter.clone())
                .await
                .map_err(|e| format!("count: {e}"))?;
            progress.total.store(total, Ordering::Relaxed);

            let pipeline = vec![
                doc! { "$match": filter.clone() },
                doc! { "$sort": { "date": 1, "key": 1 } },
            ];
            let mut cursor = raw
                .aggregate(pipeline)
                .allow_disk_use(true)
                .batch_size(5000)
                .await
                .map_err(|e| format!("query export: {e}"))?;

            let mut out = String::with_capacity((total as usize).saturating_mul(80));
            let exported_at = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

            out.push('{');
            write_json_str(&mut out, "backend");
            out.push_str(":\"mongodb\",");
            write_json_str(&mut out, "exported_at");
            out.push(':');
            write_json_str(&mut out, &exported_at);
            out.push_str(",\"records\":");

            let processed = match format {
                "flat" => {
                    out.push('[');
                    let mut first = true;
                    let mut current = 0u64;
                    let mut last_pct = -1i32;
                    while let Some(doc) = cursor
                        .try_next()
                        .await
                        .map_err(|e| format!("cursor: {e}"))?
                    {
                        current += 1;
                        let pct = if total > 0 {
                            (current
                                .checked_mul(100)
                                .and_then(|v| v.checked_div(total))
                                .unwrap_or(0)) as i32
                        } else {
                            0
                        };
                        if pct != last_pct {
                            progress.current.store(current, Ordering::Relaxed);
                            tdebug!(
                                "mongodb",
                                "export progress: {}% ({}/{})",
                                pct,
                                current,
                                total
                            );
                            last_pct = pct;
                        }
                        if first {
                            first = false;
                        } else {
                            out.push(',');
                        }
                        let date = doc.get_str("date").unwrap_or("");
                        let key = doc.get_str("key").unwrap_or("");
                        let cnt = doc.get_i64("count").unwrap_or(0);
                        out.push_str("{\"date\":");
                        write_json_str(&mut out, date);
                        out.push_str(",\"key\":");
                        write_json_str(&mut out, key);
                        out.push_str(",\"count\":");
                        out.push_str(&cnt.to_string());
                        out.push('}');
                    }
                    out.push(']');
                    current
                }
                _ => {
                    out.push('{');
                    let mut first_date = true;
                    let mut current_date = String::new();
                    let mut current = 0u64;
                    let mut last_pct = -1i32;
                    while let Some(doc) = cursor
                        .try_next()
                        .await
                        .map_err(|e| format!("cursor: {e}"))?
                    {
                        current += 1;
                        let pct = if total > 0 {
                            (current
                                .checked_mul(100)
                                .and_then(|v| v.checked_div(total))
                                .unwrap_or(0)) as i32
                        } else {
                            0
                        };
                        if pct != last_pct {
                            progress.current.store(current, Ordering::Relaxed);
                            tdebug!(
                                "mongodb",
                                "export progress: {}% ({}/{})",
                                pct,
                                current,
                                total
                            );
                            last_pct = pct;
                        }
                        let date = doc.get_str("date").unwrap_or("");
                        let key = doc.get_str("key").unwrap_or("");
                        let cnt = doc.get_i64("count").unwrap_or(0);

                        if date != current_date {
                            if !first_date {
                                out.push('}');
                            }
                            if first_date {
                                first_date = false;
                            } else {
                                out.push(',');
                            }
                            current_date = date.to_string();
                            write_json_str(&mut out, &current_date);
                            out.push_str(":{");
                        } else {
                            out.push(',');
                        }
                        write_json_str(&mut out, key);
                        out.push(':');
                        out.push_str(&cnt.to_string());
                    }
                    if !first_date {
                        out.push('}');
                    }
                    out.push('}');
                    current
                }
            };
            tdebug!(
                "mongodb",
                "export cursor exhausted: processed={}, expected={}",
                processed,
                total
            );
            out.push('}');
            Ok(out)
        })
    }

    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) -> Result<(), String> {
        let value: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("parse json: {e}"))?;

        let records_map: HashMap<String, HashMap<String, u64>> = match value["records"] {
            serde_json::Value::Object(ref obj) => {
                let mut map = HashMap::new();
                for (date, data_value) in obj {
                    let data: HashMap<String, u64> =
                        serde_json::from_value(data_value.clone()).unwrap_or_default();
                    map.insert(date.clone(), data);
                }
                if map.is_empty() {
                    twarn!("mongodb", "Import JSON contains 0 records, skipping.");
                    return Ok(());
                }
                map
            }
            serde_json::Value::Array(ref arr) => {
                let mut map: HashMap<String, HashMap<String, u64>> = HashMap::new();
                for item in arr {
                    let date = item["date"].as_str().map(String::from);
                    let key = item["key"].as_str().map(String::from);
                    let count = item["count"]
                        .as_i64()
                        .and_then(|v| if v >= 0 { Some(v as u64) } else { None })
                        .unwrap_or(0);
                    if let (Some(date), Some(key)) = (date, key) {
                        map.entry(date).or_default().insert(key, count);
                    }
                }
                if map.is_empty() {
                    twarn!("mongodb", "Import JSON contains 0 records, skipping.");
                    return Ok(());
                }
                map
            }
            _ => {
                return Err(
                    "Import JSON 'records' must be an object (nested) or array (flat)".to_string(),
                );
            }
        };

        for date in records_map.keys() {
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date '{}': expected YYYY-MM-DD format", date))?;
        }

        let total = records_map.len();
        let dates: Vec<&str> = records_map.keys().map(|s| s.as_str()).collect();
        let raw = self.raw_collection();

        self.rt.block_on(async {
            if mode == ImportMode::Overwrite {
                raw.delete_many(doc! { "date": { "$in": &dates } })
                    .await
                    .map_err(|e| format!("delete: {e}"))?;

                let docs: Vec<Document> = records_map
                    .iter()
                    .flat_map(|(date, data)| {
                        data.iter().map(move |(key, count)| {
                            doc! {
                                "date": date,
                                "key": key,
                                "count": *count as i64,
                            }
                        })
                    })
                    .collect();

                if !docs.is_empty() {
                    raw.insert_many(docs)
                        .await
                        .map_err(|e| format!("insert: {e}"))?;
                }
            } else {
                let filter = doc! { "date": { "$in": &dates } };
                let mut existing_map: HashMap<String, HashMap<String, u64>> = HashMap::new();
                {
                    let mut cursor = raw
                        .find(filter)
                        .await
                        .map_err(|e| format!("query for merge: {e}"))?;
                    while let Some(d) = cursor
                        .try_next()
                        .await
                        .map_err(|e| format!("cursor: {e}"))?
                    {
                        if let (Some(date), Some(key), Some(count)) = (
                            d.get_str("date").ok().map(String::from),
                            d.get_str("key").ok().map(String::from),
                            d.get_i64("count").ok().map(|v| v as u64),
                        ) {
                            existing_map.entry(date).or_default().insert(key, count);
                        }
                    }
                }

                raw.delete_many(doc! { "date": { "$in": &dates } })
                    .await
                    .map_err(|e| format!("delete for merge: {e}"))?;

                let docs: Vec<Document> = records_map
                    .iter()
                    .flat_map(|(date, incoming)| {
                        let mut merged = existing_map.remove(date.as_str()).unwrap_or_default();
                        for (k, v) in incoming {
                            *merged.entry(k.clone()).or_insert(0) += v;
                        }
                        merged.into_iter().map(move |(key, count)| {
                            doc! {
                                "date": date,
                                "key": key,
                                "count": count as i64,
                            }
                        })
                    })
                    .collect();

                if !docs.is_empty() {
                    raw.insert_many(docs)
                        .await
                        .map_err(|e| format!("insert merged: {e}"))?;
                }
            }
            Ok::<(), String>(())
        })?;

        tinfo!(
            "mongodb",
            "Imported {} date records from JSON (mode: {:?}).",
            total,
            mode
        );
        Ok(())
    }
}
