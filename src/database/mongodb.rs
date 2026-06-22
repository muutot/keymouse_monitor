use std::collections::HashMap;

use futures::TryStreamExt;
use mongodb::bson::doc;
use mongodb::options::{FindOptions, UpdateOptions};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::config::MongoConfig;

use super::{BackendType, DatabaseBackend};

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
}

fn ensure_scheme(uri: &str) -> String {
    if uri.contains("://") {
        return uri.to_string();
    }
    if uri.contains("mongodb.net") || uri.contains("mongodb-dev.net") {
        format!("mongodb+srv://{}", uri)
    } else {
        format!("mongodb://{}", uri)
    }
}

const CONNECT_TIMEOUT_MS: &str = "15000";
const SERVER_SELECT_TIMEOUT_MS: &str = "30000";

fn append_timeout(uri: &str) -> String {
    let has_ct = uri.contains("connectTimeoutMS");
    let has_sst = uri.contains("serverSelectionTimeoutMS");
    if has_ct && has_sst {
        return uri.to_string();
    }
    let mut s = uri.to_string();
    if !has_ct {
        let sep = if s.contains('?') { "&" } else { "?" };
        s.push_str(&format!("{}connectTimeoutMS={}", sep, CONNECT_TIMEOUT_MS));
    }
    if !has_sst {
        let sep = if s.contains('?') { "&" } else { "?" };
        s.push_str(&format!("{}serverSelectionTimeoutMS={}", sep, SERVER_SELECT_TIMEOUT_MS));
    }
    s
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
    let uri = ensure_scheme(&cfg.uri);

    // Credentials already embedded in URI
    if uri.contains('@') {
        return append_timeout(&uri);
    }

    let db = &cfg.database;

    // Build from separate config fields
    if let (Some(username), Some(password)) = (&cfg.username, &cfg.password) {
        if !username.is_empty() && !password.is_empty() {
            let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
            let host = &uri[scheme_end..];
            let scheme = &uri[..scheme_end];
            let encoded_user = url_encode(username);
            let encoded_pass = url_encode(password);
            let mut result = format!("{}{}:{}@{}/{}", scheme, encoded_user, encoded_pass, host, db);
            if let Some(src) = &cfg.auth_source {
                if !src.is_empty() {
                    result.push_str(&format!("?authSource={}", src));
                }
            }
            return append_timeout(&result);
        }
    }

    let trimmed = uri.trim_end_matches('/');
    if trimmed.contains('/') {
        append_timeout(trimmed)
    } else {
        append_timeout(&format!("{}/{}", trimmed, db))
    }
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
    pub fn new(cfg: &MongoConfig) -> Self {
        let uri = build_uri(cfg);
        println!("[mongodb] URI: {}", redact_credentials(&uri));

        let rt = Runtime::new().expect("Failed to create tokio runtime for MongoDB");

        let client = rt.block_on(async {
            mongodb::Client::with_uri_str(&uri)
                .await
                .unwrap_or_else(|e| panic!("Failed to create MongoDB client: {e}"))
        });

        let db_name = cfg.database.clone();
        let backend = Self { rt, client, db_name };
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
                let collection = db.collection::<DailyStat>("daily_stats");
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
        self.client.database(&self.db_name).collection("daily_stats")
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
            let mut cursor = collection.find(filter, None).await.expect("Failed to query range");
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


}
