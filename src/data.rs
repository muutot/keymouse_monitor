use std::collections::HashMap;
use std::mem;

use chrono::Local;

use crate::{
    config::UpdateMode,
    database::{Database, ImportMode},
    tinfo,
};

/// Result of `prepare_save`.  `delta` is always populated for the
/// current day; on a day-rollover `is_rollover` is set and the caller
/// must persist yesterday's snapshot (accessible via `yesterday_snapshot`)
/// before letting the in-memory state move on.  In `Diff` mode (the
/// common case) callers only need `delta` and can ignore
/// `yesterday_snapshot`.
pub struct SaveResult {
    pub date: String,
    pub delta: HashMap<String, u64>,
    /// Snapshot of the *previous* day on rollover, otherwise empty.
    pub yesterday_snapshot: HashMap<String, u64>,
    pub is_rollover: bool,
}

pub struct MonitorData {
    pub base_counts: HashMap<String, u64>,
    pub incremental_counts: HashMap<String, u64>,
    pub today: String,
}

impl MonitorData {
    pub fn new(db: &Database) -> Self {
        let today_str = Local::now().format("%Y-%m-%d").to_string();
        let base = db.get_stats_for_day(&today_str);

        tinfo!("data", "Data loading...");
        if base.is_empty() {
            tinfo!("data",
                "No data found for {} in database, starting from scratch.",
                today_str
            );
        } else {
            tinfo!("data",
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

    /// Merges incremental into base and returns a SaveResult.  In the
    /// common `Diff` save path only `delta` is populated — the full
    /// snapshot is no longer cloned on every tick.  On rollover, the
    /// yesterday snapshot is taken once and stashed in the result.
    pub fn prepare_save(&mut self) -> Option<SaveResult> {
        let today_str = Local::now().format("%Y-%m-%d").to_string();

        if self.today != today_str {
            // Day rollover: base_counts belong to yesterday, so capture
            // them and reset.
            let yesterday_snapshot = mem::take(&mut self.base_counts);
            let old_today = mem::replace(&mut self.today, today_str);
            for (key, value) in self.incremental_counts.drain() {
                *self.base_counts.entry(key).or_insert(0) += value;
            }
            return Some(SaveResult {
                date: old_today,
                delta: HashMap::new(),
                yesterday_snapshot,
                is_rollover: true,
            });
        }

        if self.incremental_counts.is_empty() {
            return None;
        }

        let delta = mem::take(&mut self.incremental_counts);
        for (key, value) in &delta {
            *self.base_counts.entry(key.clone()).or_insert(0) += value;
        }
        Some(SaveResult {
            date: self.today.clone(),
            delta,
            yesterday_snapshot: HashMap::new(),
            is_rollover: false,
        })
    }

    pub fn save_to_db(&mut self, db: &mut Database, update_mode: &UpdateMode) {
        if let Some(result) = self.prepare_save() {
            if result.is_rollover {
                db.upsert_day_stats(&result.date, &result.yesterday_snapshot);
                if !self.base_counts.is_empty() {
                    db.upsert_day_stats(&self.today, &self.base_counts);
                }
            } else {
                match update_mode {
                    UpdateMode::Diff => db.merge_incremental_stats(&result.date, &result.delta),
                    UpdateMode::Full => db.upsert_day_stats(&result.date, &self.get_key_counts()),
                }
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_empty() -> MonitorData {
        let today = Local::now().format("%Y-%m-%d").to_string();
        MonitorData {
            base_counts: HashMap::new(),
            incremental_counts: HashMap::new(),
            today,
        }
    }

    fn make_today() -> String {
        Local::now().format("%Y-%m-%d").to_string()
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
        let mut db = make_db();
        let today = make_today();

        // Pre-populate DB with base data
        let mut base = HashMap::new();
        base.insert("a".to_string(), 10);
        db.upsert_day_stats(&today, &base);

        // Load from DB and add incremental
        let mut data = MonitorData {
            base_counts: db.get_stats_for_day(&today),
            incremental_counts: HashMap::new(),
            today: today.clone(),
        };
        data.increase_count("a");
        data.increase_count("b");

        data.save_to_db(&mut db, &UpdateMode::Diff);

        assert!(data.incremental_counts.is_empty());
        assert_eq!(data.base_counts.get("a"), Some(&11));
        assert_eq!(data.base_counts.get("b"), Some(&1));

        // DB should reflect merged values (base from earlier + delta now)
        let saved = db.get_stats_for_day(&today);
        assert_eq!(saved.get("a"), Some(&11));
        assert_eq!(saved.get("b"), Some(&1));
    }

    #[test]
    fn test_save_to_db_empty_incremental_does_nothing() {
        let mut db = make_db();
        let mut data = make_empty();
        data.base_counts.insert("k".to_string(), 7);

        data.save_to_db(&mut db, &UpdateMode::Full);

        assert_eq!(data.base_counts.get("k"), Some(&7));
        let saved = db.get_stats_for_day("2026-06-22");
        assert!(saved.is_empty());
    }

    #[test]
    fn test_save_to_db_day_rollover_clears_base() {
        let mut db = make_db();
        let mut data = make_empty();
        let today = make_today();
        data.base_counts.insert("old".to_string(), 99);
        data.today = "2000-01-01".to_string();
        data.increase_count("new_day");

        // save_to_db handles both saves: flushes yesterday, persists today
        data.save_to_db(&mut db, &UpdateMode::Full);

        assert_eq!(data.today, today);
        assert!(data.incremental_counts.is_empty());
        assert_eq!(data.base_counts.get("new_day"), Some(&1), "new day's data in base");
        assert!(!data.base_counts.contains_key("old"), "yesterday's data cleared on rollover");

        let saved = db.get_stats_for_day(&today);
        assert_eq!(saved.get("new_day"), Some(&1), "new data in today's db entry");
        let old_saved = db.get_stats_for_day("2000-01-01");
        assert_eq!(old_saved.get("old"), Some(&99), "old base saved to yesterday's date");
    }

    #[test]
    fn test_prepare_save_returns_delta() {
        let mut data = make_empty();
        let today = make_today();
        data.today = today.clone();
        data.increase_count("a");
        data.increase_count("b");

        let result = data.prepare_save();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.date, today);
        assert!(!result.is_rollover);
        assert_eq!(result.delta.get("a"), Some(&1));
        assert_eq!(result.delta.get("b"), Some(&1));
        let snapshot = data.get_key_counts();
        assert_eq!(snapshot.get("a"), Some(&1));
        assert_eq!(snapshot.get("b"), Some(&1));
    }

    #[test]
    fn test_prepare_save_rollover_has_empty_delta() {
        let mut data = make_empty();
        data.today = "2000-01-01".to_string();
        data.base_counts.insert("old".to_string(), 42);

        let result = data.prepare_save();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.date, "2000-01-01");
        assert!(result.is_rollover);
        assert!(result.delta.is_empty());
        assert_eq!(result.yesterday_snapshot.get("old"), Some(&42));
    }

    #[test]
    fn test_save_to_db_merges_incrementally() {
        let mut db = make_db();
        let today = make_today();

        // Pre-populate DB with base data (simulates previous session)
        let mut base = HashMap::new();
        base.insert("a".to_string(), 10);
        db.upsert_day_stats(&today, &base);

        // Load from DB (simulates MonitorData::new)
        let mut data = MonitorData {
            base_counts: db.get_stats_for_day(&today),
            incremental_counts: HashMap::new(),
            today: today.clone(),
        };
        assert_eq!(data.base_counts.get("a"), Some(&10));

        // New keypresses since last save
        data.increase_count("a");
        data.increase_count("b");
        data.save_to_db(&mut db, &UpdateMode::Diff);

        // Verify: incremental merge should have added to existing
        let saved = db.get_stats_for_day(&today);
        assert_eq!(saved.get("a"), Some(&11), "a was 10, incremented by 1 -> 11");
        assert_eq!(saved.get("b"), Some(&1), "b new -> 1");

        // Another round of diffs
        data.increase_count("a");
        data.increase_count("c");
        data.save_to_db(&mut db, &UpdateMode::Diff);

        let saved = db.get_stats_for_day(&today);
        assert_eq!(saved.get("a"), Some(&12), "a was 11, incremented by 1 -> 12");
        assert_eq!(saved.get("b"), Some(&1), "b unchanged");
        assert_eq!(saved.get("c"), Some(&1), "c new -> 1");
    }
}
