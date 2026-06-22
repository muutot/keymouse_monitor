use std::collections::HashMap;

use chrono::Local;

use crate::database::{Database, ImportMode};

pub struct MonitorData {
    pub base_counts: HashMap<String, u64>,
    pub incremental_counts: HashMap<String, u64>,
    pub today: String,
}

impl MonitorData {
    pub fn new(db: &Database) -> Self {
        let today_str = Local::now().format("%Y-%m-%d").to_string();
        let base = db.get_stats_for_day(&today_str);

        println!("Data loading...");
        if base.is_empty() {
            println!(
                "No data found for {} in database, starting from scratch.",
                today_str
            );
        } else {
            println!(
                "Successfully loaded base data for {} from database.",
                today_str
            );
        }

        Self {
            base_counts: base,
            incremental_counts: HashMap::new(),
            today: today_str,
        }
    }

    pub fn increase_count(&mut self, key_name: &str) {
        *self
            .incremental_counts
            .entry(key_name.to_string())
            .or_insert(0) += 1;
    }

    pub fn get_key_counts(&self) -> HashMap<String, u64> {
        let mut total = self.base_counts.clone();
        for (key, value) in &self.incremental_counts {
            *total.entry(key.clone()).or_insert(0) += value;
        }
        total
    }

    pub fn import_today_data(&mut self, data: &HashMap<String, u64>, mode: ImportMode) {
        match mode {
            ImportMode::Overwrite => {
                self.base_counts = data.clone();
            }
            ImportMode::Merge => {
                for (k, v) in data {
                    *self.base_counts.entry(k.clone()).or_insert(0) += v;
                }
            }
        }
    }

    pub fn save_to_db(&mut self, db: &Database) {
        let today_str = Local::now().format("%Y-%m-%d").to_string();

        if self.incremental_counts.is_empty() {
            return;
        }

        let total_counts = self.get_key_counts();
        db.upsert_day_stats(&today_str, &total_counts);

        if self.today != today_str {
            self.base_counts.clear();
            self.today = today_str;
        } else {
            self.base_counts = total_counts;
        }

        self.incremental_counts.clear();
        println!(
            "Data merged and saved to database. Time: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
    }

}

#[cfg(test)]
mod tests {
    use super::*;
use crate::database::Database;

    fn make_empty() -> MonitorData {
        MonitorData {
            base_counts: HashMap::new(),
            incremental_counts: HashMap::new(),
            today: "2026-06-22".to_string(),
        }
    }

    fn make_db() -> Database {
        Database::new(":memory:")
    }

    #[test]
    fn test_increase_count_new_key() {
        let mut data = make_empty();
        data.increase_count("a");
        assert_eq!(data.incremental_counts.get("a"), Some(&1));
    }

    #[test]
    fn test_increase_count_existing_key() {
        let mut data = make_empty();
        data.increase_count("a");
        data.increase_count("a");
        data.increase_count("a");
        assert_eq!(data.incremental_counts.get("a"), Some(&3));
    }

    #[test]
    fn test_increase_count_multiple_keys() {
        let mut data = make_empty();
        data.increase_count("a");
        data.increase_count("b");
        data.increase_count("a");
        assert_eq!(data.incremental_counts.get("a"), Some(&2));
        assert_eq!(data.incremental_counts.get("b"), Some(&1));
    }

    #[test]
    fn test_get_key_counts_empty() {
        let data = make_empty();
        let counts = data.get_key_counts();
        assert!(counts.is_empty());
    }

    #[test]
    fn test_get_key_counts_incremental_only() {
        let mut data = make_empty();
        data.increase_count("x");
        data.increase_count("x");
        let counts = data.get_key_counts();
        assert_eq!(counts.get("x"), Some(&2));
        assert_eq!(counts.len(), 1);
    }

    #[test]
    fn test_get_key_counts_merges_base_and_incremental() {
        let mut data = make_empty();
        data.base_counts.insert("existing".to_string(), 10);
        data.increase_count("new");
        data.increase_count("existing");

        let counts = data.get_key_counts();
        assert_eq!(counts.get("existing"), Some(&11));
        assert_eq!(counts.get("new"), Some(&1));
    }

    #[test]
    fn test_get_key_counts_does_not_mutate_base() {
        let mut data = make_empty();
        data.base_counts.insert("k".to_string(), 5);
        data.increase_count("k");
        let _ = data.get_key_counts();
        assert_eq!(data.base_counts.get("k"), Some(&5));
    }

    #[test]
    fn test_save_to_db_merges_and_clears_incremental() {
        let db = make_db();
        let mut data = make_empty();
        data.base_counts.insert("a".to_string(), 10);
        data.increase_count("a");
        data.increase_count("b");

        data.save_to_db(&db);

        assert!(data.incremental_counts.is_empty());
        assert_eq!(data.base_counts.get("a"), Some(&11));
        assert_eq!(data.base_counts.get("b"), Some(&1));

        let saved = db.get_stats_for_day("2026-06-22");
        assert_eq!(saved.get("a"), Some(&11));
        assert_eq!(saved.get("b"), Some(&1));
    }

    #[test]
    fn test_save_to_db_empty_incremental_does_nothing() {
        let db = make_db();
        let mut data = make_empty();
        data.base_counts.insert("k".to_string(), 7);

        data.save_to_db(&db);

        assert_eq!(data.base_counts.get("k"), Some(&7));
        let saved = db.get_stats_for_day("2026-06-22");
        assert!(saved.is_empty());
    }

    #[test]
    fn test_save_to_db_day_rollover_clears_base() {
        let db = make_db();
        let mut data = make_empty();
        data.base_counts.insert("old".to_string(), 99);
        data.today = "2026-06-21".to_string();
        data.increase_count("new_day");

        data.save_to_db(&db);

        assert_eq!(data.today, "2026-06-22");
        assert!(data.base_counts.is_empty(), "base cleared on day rollover");
        assert!(data.incremental_counts.is_empty());

        let saved = db.get_stats_for_day("2026-06-22");
        assert_eq!(saved.get("new_day"), Some(&1), "new data in today's db entry");
        assert_eq!(saved.get("old"), Some(&99), "old base merged into today's db entry");
    }
}
