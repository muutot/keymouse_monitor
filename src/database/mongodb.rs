use std::collections::HashMap;

use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, UpdateOptions};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::config::MongoConfig;

use super::{BackendType, DatabaseBackend, ImportMode};

#[derive(Debug, Serialize, Deserialize)]
struct DailyStat {
    date: String,
    data: HashMap<String, u64>,
}

pub struct MongoBackend {
    /// Dedicated runtime created once in a blocking context;
    /// all mongodb ops run inside this runtime via rt.block_on().
    rt: Runtime,
    client: mongodb::Client,
    db_name: String,
    collection_name: String,
}

fn redact_credentials(uri: &str) -> String {
    if let Some(at) = uri.find('@') {
        let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
        format!("{}<credentials>@{}", &uri[..scheme_end], &uri[at + 1..])
    } else {
        uri.to_string()
    }
}

fn build_uri(cfg: &MongoConfig) -> String {
    // Build URI from individual config fields
    let mut result = format!("{}://", cfg.protocol);

    // Add credentials if provided
    if let (Some(username), Some(password)) = (&cfg.username, &cfg.password) {
        if !username.is_empty() && !password.is_empty() {
            let encoded_user = url_encode(username);
            let encoded_pass = url_encode(password);
            result.push_str(&format!("{}:{}@", encoded_user, encoded_pass));
        }
    }

    // Add hosts
    if let Some(hosts) = &cfg.hosts {
        if !hosts.is_empty() {
            result.push_str(&hosts.join(","));
        }
    }

    // Add database
    let db = &cfg.database;
    if !db.is_empty() {
        result.push_str(&format!("/{}", db));
    }

    // Build query parameters
    let mut params = Vec::new();

    // SSL
    if cfg.ssl {
        params.push("ssl=true".to_string());
    }

    // Replica set
    if let Some(replica_set) = &cfg.replica_set {
        if !replica_set.is_empty() {
            params.push(format!("replicaSet={}", replica_set));
        }
    }

    // Auth source
    if !cfg.auth_source.is_empty() {
        params.push(format!("authSource={}", cfg.auth_source));
    }

    // App name
    if let Some(app_name) = &cfg.app_name {
        if !app_name.is_empty() {
            params.push(format!("appName={}", app_name));
        }
    }

    // Add query parameters to URI
    if !params.is_empty() {
        result.push_str(&format!("?{}", params.join("&")));
    }

    // Add timeouts
    if !result.contains("connectTimeoutMS") {
        let sep = if result.contains('?') { "&" } else { "?" };
        result.push_str(&format!("{}connectTimeoutMS={}", sep, cfg.connect_timeout_ms));
    }
    if !result.contains("serverSelectionTimeoutMS") {
        let sep = if result.contains('?') { "&" } else { "?" };
        result.push_str(&format!("{}serverSelectionTimeoutMS={}", sep, cfg.server_selection_timeout_ms));
    }

    result
}

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            '@' => "%40".to_string(),
            '#' => "%23".to_string(),
            '?' => "%3F".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            ' ' => "%20".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

impl MongoBackend {
    /// MUST be called from a **non‑tokio** thread (e.g. spawn_blocking).
    pub fn new(cfg: &MongoConfig, _use_server_aggregation: bool) -> Self {
        let uri = build_uri(cfg);
        println!("[mongodb] URI: {}", redact_credentials(&uri));

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
            eprintln!("\n\x1b[31m⚠ MongoDB connection failed:\x1b[0m");
            eprintln!("  {e}");
            eprintln!("  \nPossible causes:");
            eprintln!("    • Network/firewall blocking Atlas (check your VPN/proxy)");
            eprintln!("    • IP not whitelisted in Atlas console");
            eprintln!("    • Wrong credentials in config.json");
            eprintln!("  \nThe application will retry on each data save.\n");
        }
        backend
    }

    fn init_db(&self) -> Result<(), String> {
        println!("[mongodb] Connecting to MongoDB...");
        let db = self.client.database(&self.db_name);
        let ping_result: Result<mongodb::bson::Document, String> = self.rt.block_on(async {
            db.run_command(doc! { "ping": 1 }, None)
                .await
                .map_err(|e| format!("{e}"))
        });
        match ping_result {
            Ok(_) => {
                println!("[mongodb] Connection established.");
                let collection = db.collection::<DailyStat>(&self.collection_name);
                let index = mongodb::IndexModel::builder()
                    .keys(doc! { "date": 1 })
                    .build();
                let _ = self
                    .rt
                    .block_on(async { collection.create_index(index, None).await });
                println!("[mongodb] Database initialization complete.");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn collection(&self) -> mongodb::Collection<DailyStat> {
        self.client.database(&self.db_name).collection(&self.collection_name)
    }
}

impl DatabaseBackend for MongoBackend {
    fn backend_type(&self) -> BackendType {
        BackendType::MongoDb
    }

    fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        let collection = self.collection();
        let filter = doc! { "date": date_str };
        self.rt.block_on(async {
            collection
                .find_one(filter, None)
                .await
                .ok()
                .flatten()
                .map(|d| d.data)
                .unwrap_or_default()
        })
    }

    fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> HashMap<String, u64> {
        let collection = self.collection();
        let filter = doc! {
            "date": { "$gte": start_date, "$lte": end_date }
        };
        self.rt.block_on(async {
            let mut cursor = collection
                .find(filter, None)
                .await
                .expect("Failed to query range");
            let mut aggregated = HashMap::new();
            while let Some(stat) = cursor.try_next().await.unwrap_or(None) {
                for (key, value) in stat.data {
                    *aggregated.entry(key).or_insert(0) += value;
                }
            }
            aggregated
        })
    }

    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        let collection = self.collection();
        let filter = doc! { "date": date_str };
        let update = doc! { "$set": { "data": mongodb::bson::to_bson(data).unwrap() } };
        let opts = UpdateOptions::builder().upsert(true).build();
        self.rt.block_on(async {
            collection
                .update_one(filter, update, opts)
                .await
                .expect("Failed to upsert daily stats");
        });
    }

    fn export_to_json(&self) -> String {
        let collection = self.collection();
        let opts = FindOptions::builder()
            .sort(doc! { "date": 1 })
            .build();
        self.rt.block_on(async {
            let mut cursor = collection
                .find(doc! {}, opts)
                .await
                .expect("Failed to query export data");

            let mut records = serde_json::Map::new();
            while let Some(stat) = cursor.try_next().await.unwrap_or(None) {
                let value =
                    serde_json::to_value(&stat.data).unwrap_or(serde_json::Value::Null);
                records.insert(stat.date, value);
            }

            let export = serde_json::json!({
                "backend": "mongodb",
                "exported_at": chrono::Local::now()
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string(),
                "records": records,
            });
            serde_json::to_string_pretty(&export).expect("Failed to serialize export JSON")
        })
    }

    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) {
        let value: serde_json::Value = serde_json::from_str(json_str)
            .expect("Failed to parse import JSON");
        let records = value
            .get("records")
            .and_then(|v| v.as_object())
            .expect("Import JSON missing 'records' object");

        let total = records.len();
        if total == 0 {
            println!("[mongodb] Import JSON contains 0 records, skipping.");
            return;
        }

        let collection = self.collection();
        let dates: Vec<&str> = records.keys().map(|s| s.as_str()).collect();

        self.rt.block_on(async {
            if mode == ImportMode::Overwrite {
                collection
                    .delete_many(doc! { "date": { "$in": &dates } }, None)
                    .await
                    .expect("Failed to delete existing records");

                let docs: Vec<DailyStat> = records
                    .iter()
                    .map(|(date, data_value)| {
                        let data: HashMap<String, u64> =
                            serde_json::from_value(data_value.clone()).unwrap_or_default();
                        DailyStat {
                            date: date.clone(),
                            data,
                        }
                    })
                    .collect();

                collection
                    .insert_many(docs, None)
                    .await
                    .expect("Failed to insert records");
            } else {
                let existing_map: HashMap<String, HashMap<String, u64>> = {
                    let filter = doc! { "date": { "$in": &dates } };
                    let mut cursor = collection
                        .find(filter, None)
                        .await
                        .expect("Failed to query existing records for merge");
                    let mut map = HashMap::new();
                    while let Some(stat) = cursor.try_next().await.unwrap_or(None) {
                        map.insert(stat.date, stat.data);
                    }
                    map
                };

                collection
                    .delete_many(doc! { "date": { "$in": &dates } }, None)
                    .await
                    .expect("Failed to delete existing records");

                let docs: Vec<DailyStat> = records
                    .iter()
                    .map(|(date, data_value)| {
                        let incoming: HashMap<String, u64> =
                            serde_json::from_value(data_value.clone()).unwrap_or_default();
                        let mut merged =
                            existing_map.get(date.as_str()).cloned().unwrap_or_default();
                        for (k, v) in incoming {
                            *merged.entry(k).or_insert(0) += v;
                        }
                        DailyStat {
                            date: date.clone(),
                            data: merged,
                        }
                    })
                    .collect();

                collection
                    .insert_many(docs, None)
                    .await
                    .expect("Failed to insert merged records");
            }
        });

        println!(
            "[mongodb] Imported {} date records from JSON (mode: {:?}).",
            total, mode
        );
    }


}
