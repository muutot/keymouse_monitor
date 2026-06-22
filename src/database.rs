use std::collections::HashMap;

use rusqlite::Connection;
use serde_json;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_file: &str) -> Self {
        let conn = Connection::open(db_file).expect("Failed to open database");
        conn.execute_batch("PRAGMA journal_mode=WAL")
            .expect("Failed to set WAL mode");
        conn.execute_batch("PRAGMA synchronous=NORMAL")
            .expect("Failed to set synchronous mode");
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
            .prepare_cached("SELECT data FROM daily_stats WHERE date = ?1")
            .expect("Failed to prepare SELECT statement");
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

    pub fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) {
        let json_str = serde_json::to_string(data).expect("Failed to serialize stats to JSON");
        self.conn
            .execute(
                "INSERT OR REPLACE INTO daily_stats (date, data) VALUES (?1, ?2)",
                [date_str, &json_str],
            )
            .expect("Failed to upsert daily stats");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db() -> Database {
        Database::new(":memory:")
    }

    #[test]
    fn test_init_creates_table() {
        let db = make_db();
        let loaded = db.get_stats_for_day("2026-06-22");
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_upsert_and_get_day() {
        let db = make_db();
        let mut data = HashMap::new();
        data.insert("a".to_string(), 5);
        data.insert("b".to_string(), 3);

        db.upsert_day_stats("2026-06-22", &data);
        let loaded = db.get_stats_for_day("2026-06-22");

        assert_eq!(loaded.get("a"), Some(&5));
        assert_eq!(loaded.get("b"), Some(&3));
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_upsert_replaces_existing() {
        let db = make_db();
        let mut data = HashMap::new();
        data.insert("x".to_string(), 10);
        db.upsert_day_stats("2026-06-22", &data);

        let mut data2 = HashMap::new();
        data2.insert("x".to_string(), 99);
        data2.insert("y".to_string(), 7);
        db.upsert_day_stats("2026-06-22", &data2);

        let loaded = db.get_stats_for_day("2026-06-22");
        assert_eq!(loaded.get("x"), Some(&99));
        assert_eq!(loaded.get("y"), Some(&7));
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_get_stats_for_missing_day_returns_empty() {
        let db = make_db();
        let loaded = db.get_stats_for_day("2099-01-01");
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_get_stats_for_range_single_day() {
        let db = make_db();
        let mut data = HashMap::new();
        data.insert("k".to_string(), 42);
        db.upsert_day_stats("2026-06-22", &data);

        let result = db.get_stats_for_range("2026-06-22", "2026-06-22");
        assert_eq!(result.get("k"), Some(&42));
    }

    #[test]
    fn test_get_stats_for_range_multiple_days() {
        let db = make_db();
        let mut day1 = HashMap::new();
        day1.insert("a".to_string(), 1);
        db.upsert_day_stats("2026-06-01", &day1);

        let mut day2 = HashMap::new();
        day2.insert("a".to_string(), 2);
        day2.insert("b".to_string(), 3);
        db.upsert_day_stats("2026-06-02", &day2);

        let result = db.get_stats_for_range("2026-06-01", "2026-06-02");
        assert_eq!(result.get("a"), Some(&3));
        assert_eq!(result.get("b"), Some(&3));
    }

    #[test]
    fn test_get_stats_for_range_no_data() {
        let db = make_db();
        let result = db.get_stats_for_range("2000-01-01", "2000-01-02");
        assert!(result.is_empty());
    }

    #[test]
    fn test_upsert_and_get_empty_data() {
        let db = make_db();
        let data = HashMap::new();
        db.upsert_day_stats("2026-06-22", &data);
        let loaded = db.get_stats_for_day("2026-06-22");
        assert!(loaded.is_empty());
    }
}
