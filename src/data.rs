use std::collections::HashMap;

use chrono::Local;

use crate::database::Database;

pub struct MonitorData {
    pub base_counts: HashMap<String, u64>,
    pub incremental_counts: HashMap<String, u64>,
    pub today: String,
    pub total_since_save: u64,
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
            total_since_save: 0,
        }
    }

    pub fn increase_count(&mut self, key_name: &str) {
        *self
            .incremental_counts
            .entry(key_name.to_string())
            .or_insert(0) += 1;
        self.total_since_save += 1;
    }

    pub fn get_key_counts(&self) -> HashMap<String, u64> {
        let mut total = self.base_counts.clone();
        for (key, value) in &self.incremental_counts {
            *total.entry(key.clone()).or_insert(0) += value;
        }
        total
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
        self.total_since_save = 0;
        println!(
            "Data merged and saved to database. Time: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
    }
}
