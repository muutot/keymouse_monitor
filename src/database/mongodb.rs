use std::collections::HashMap;
use std::time::{Duration, Instant};

use futures::TryStreamExt;
use mongodb::bson::{doc, Document};
use mongodb::options::{DeleteManyModel, InsertOneModel, WriteModel};

use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::{tinfo, twarn, tdebug, config::MongoConfig};
use keymouse_common::database::{build_uri, redact_credentials};

use super::{BackendType, DatabaseBackend, ImportMode};

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

        let client = rt.block_on(async {
            mongodb::Client::with_uri_str(&uri)
                .await
                .unwrap_or_else(|e| panic!("Failed to create MongoDB client: {e}"))
        });

        let db_name = cfg.database.clone();
        let collection_name = cfg.collection.clone();
        let backend = Self { rt, client, db_name, collection_name };
        if let Err(e) = backend.init_db() {
            twarn!("mongodb", "\n⚠ MongoDB connection failed:");
            twarn!("mongodb", "  {e}");
            twarn!("mongodb", "  \nPossible causes:");
            twarn!("mongodb", "    • Network/firewall blocking Atlas (check your VPN/proxy)");
            twarn!("mongodb", "    • IP not whitelisted in Atlas console");
            twarn!("mongodb", "    • Wrong credentials in config.json");
            twarn!("mongodb", "  \nThe application will retry on each data save.\n");
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
                let data = old_doc
                    .get_document("data")
                    .ok()
                    .cloned();
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
        self.client.database(&self.db_name).collection(&self.collection_name)
    }

    fn flat_collection(&self) -> mongodb::Collection<FlatStat> {
        self.client.database(&self.db_name).collection(&self.collection_name)
    }
}

impl DatabaseBackend for MongoBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::MongoDb
    }

    fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        let collection = self.flat_collection();
        let filter = doc! { "date": date_str };
        self.rt.block_on(async {
            let mut cursor = collection
                .find(filter)
                .await
                .expect("Failed to query day stats");
            let mut result = HashMap::new();
            while let Some(stat) = cursor.try_next().await.unwrap_or(None) {
                result.insert(stat.key, stat.count);
            }
            result
        })
    }

    fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> HashMap<String, u64> {
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
                .expect("Failed to aggregate range");
            let t1 = Instant::now();

            let mut aggregated = HashMap::new();
            let mut count = 0u64;
            let mut network_total = Duration::ZERO;
            let mut process_total = Duration::ZERO;
            loop {
                let fetch_start = Instant::now();
                let result = cursor.try_next().await.unwrap_or(None);
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

            tdebug!("mongodb",
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
            aggregated
        })
    }

    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        let raw = self.raw_collection();
        let ns = raw.namespace();
        let client = &self.client;
        let t0 = Instant::now();
        let key_count = data.len();
        let is_empty = data.is_empty();
        self.rt.block_on(async {
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
                .expect("Failed to upsert day stats via bulk_write");
            let t1 = Instant::now();

            tdebug!("mongodb",
                "upsert_day_stats({}): bulk_write={:?} ({} keys{})",
                date_str,
                t1 - t0,
                key_count,
                if is_empty { ", empty" } else { "" },
            );
        });
    }

    fn export_to_json(&self, format: &str) -> String {
        let raw = self.raw_collection();
        self.rt.block_on(async {
            let mut cursor = raw
                .find(doc! {})
                .sort(doc! { "date": 1, "key": 1 })
                .await
                .expect("Failed to query export data");

            let mut flat_rows: Vec<Document> = Vec::new();
            while let Some(doc) = cursor.try_next().await.unwrap_or(None) {
                flat_rows.push(doc);
            }

            match format {
                "flat" => {
                    let records: Vec<serde_json::Value> = flat_rows
                        .iter()
                        .filter_map(|d| {
                            let date = d.get_str("date").ok()?;
                            let key = d.get_str("key").ok()?;
                            let count = d.get_i64("count").unwrap_or(0);
                            Some(serde_json::json!({
                                "date": date,
                                "key": key,
                                "count": count,
                            }))
                        })
                        .collect();

                    let export = serde_json::json!({
                        "backend": "mongodb",
                        "exported_at": chrono::Local::now()
                            .format("%Y-%m-%dT%H:%M:%S")
                            .to_string(),
                        "records": records,
                    });
                    serde_json::to_string_pretty(&export).expect("Failed to serialize export JSON")
                }
                _ => {
                    // nested format (default, backward compatible)
                    let mut records = serde_json::Map::new();
                    for d in &flat_rows {
                        let date = d.get_str("date").unwrap_or("");
                        let key = d.get_str("key").unwrap_or("");
                        let count = d.get_i64("count").unwrap_or(0) as u64;
                        let entry = records
                            .entry(date.to_string())
                            .or_insert_with(|| serde_json::json!({}));
                        if let Some(obj) = entry.as_object_mut() {
                            obj.insert(key.to_string(), serde_json::json!(count));
                        }
                    }

                    let export = serde_json::json!({
                        "backend": "mongodb",
                        "exported_at": chrono::Local::now()
                            .format("%Y-%m-%dT%H:%M:%S")
                            .to_string(),
                        "records": records,
                    });
                    serde_json::to_string_pretty(&export).expect("Failed to serialize export JSON")
                }
            }
        })
    }

    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .expect("Failed to parse import JSON");

        // Parse records into per-date HashMap
        let records_map: HashMap<String, HashMap<String, u64>> = match value["records"] {
            serde_json::Value::Object(ref obj) => {
                // Old nested format: { "records": { "2026-06-23": { "a": 10 } } }
                let mut map = HashMap::new();
                for (date, data_value) in obj {
                    let data: HashMap<String, u64> =
                        serde_json::from_value(data_value.clone()).unwrap_or_default();
                    map.insert(date.clone(), data);
                }
                if map.is_empty() {
                    twarn!("mongodb", "Import JSON contains 0 records, skipping.");
                    return;
                }
                map
            }
            serde_json::Value::Array(ref arr) => {
                // New flat format: { "records": [ { "date": "...", "key": "...", "count": N } ] }
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
                    twarn!("mongodb", "Import JSON contains 0 records, skipping.");
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
        let raw = self.raw_collection();

        self.rt.block_on(async {
            if mode == ImportMode::Overwrite {
                raw.delete_many(doc! { "date": { "$in": &dates } })
                    .await
                    .expect("Failed to delete existing records");

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
                        .expect("Failed to insert records");
                }
            } else {
                // Merge mode: read existing, merge, rewrite
                let filter = doc! { "date": { "$in": &dates } };
                let mut existing_map: HashMap<String, HashMap<String, u64>> = HashMap::new();
                {
                    let mut cursor = raw
                        .find(filter)
                        .await
                        .expect("Failed to query existing records for merge");
                    while let Some(d) = cursor.try_next().await.unwrap_or(None) {
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
                    .expect("Failed to delete existing records for merge");

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
                        .expect("Failed to insert merged records");
                }
            }
        });

        tinfo!("mongodb",
            "Imported {} date records from JSON (mode: {:?}).",
            total, mode
        );
    }
}
