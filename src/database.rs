use std::collections::HashMap;

use rusqlite::Connection;
use serde_json;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_file: &str) -> Self {
        let conn = Connection::open(db_file).expect("Failed to open database");
        conn.execute_batch("PRAGMA journal_mode=WAL").ok();
        let db = Self { conn };
        db.init_db();
        db
    }

    fn init_db(&self) {
        println!("Checking database table structure...");
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS daily_stats (
                date TEXT PRIMARY KEY,
                data TEXT NOT NULL
            )",
            )
            .expect("Failed to create table");
        println!("Database initialization complete.");
    }

    pub fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM daily_stats WHERE date = ?1")
            .unwrap();
        let result: Option<String> = stmt.query_row([date_str], |row| row.get(0)).ok();
        match result {
            Some(json_str) => serde_json::from_str(&json_str).unwrap_or_default(),
            None => HashMap::new(),
        }
    }

    pub fn get_stats_for_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> HashMap<String, u64> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM daily_stats WHERE date BETWEEN ?1 AND ?2")
            .unwrap();
        let results: Vec<String> = stmt
            .query_map([start_date, end_date], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let mut aggregated = HashMap::new();
        for json_str in results {
            let day_data: HashMap<String, u64> =
                serde_json::from_str(&json_str).unwrap_or_default();
            for (key, value) in day_data {
                *aggregated.entry(key).or_insert(0) += value;
            }
        }
        aggregated
    }

    pub fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        let json_str = serde_json::to_string(data).unwrap();
        self.conn
            .execute(
                "INSERT OR REPLACE INTO daily_stats (date, data) VALUES (?1, ?2)",
                [date_str, &json_str],
            )
            .unwrap();
    }
}
