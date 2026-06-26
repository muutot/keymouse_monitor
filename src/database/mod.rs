use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::config::{
    DatabaseConfig, FallbackConfig, FallbackSyncMode, MongoConfig, SqliteConfig,
};
use crate::{tinfo, twarn};

mod mongodb;
mod sqlite;

pub(crate) fn write_json_str(out: &mut String, s: &str) {
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

pub(crate) fn update_export_progress(
    progress: &ExportProgress,
    current: u64,
    total: u64,
    last_pct: &mut i32,
) -> i32 {
    let pct = if total > 0 {
        (current
            .checked_mul(100)
            .and_then(|v| v.checked_div(total))
            .unwrap_or(0)) as i32
    } else {
        0
    };
    if pct != *last_pct {
        progress.current.store(current, Ordering::Relaxed);
        *last_pct = pct;
    }
    pct
}

pub struct ExportProgress {
    pub total: AtomicU64,
    pub current: AtomicU64,
    pub done: AtomicBool,
}

impl ExportProgress {
    pub fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            current: AtomicU64::new(0),
            done: AtomicBool::new(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendType {
    Sqlite,
    MongoDb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportMode {
    Overwrite,
    Merge,
}

impl ImportMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "merge" | "叠加" => ImportMode::Merge,
            _ => ImportMode::Overwrite,
        }
    }
}

impl BackendType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mongodb" | "mongo" => BackendType::MongoDb,
            _ => BackendType::Sqlite,
        }
    }
}

pub trait DatabaseBackend: Send {
    fn get_stats_for_day(&self, date_str: &str) -> Result<HashMap<String, u64>, String>;
    fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> Result<HashMap<String, u64>, String>;
    fn upsert_day_stats(&self, date_str: &str, data: &HashMap<String, u64>) -> Result<(), String>;
    fn merge_incremental_stats(&self, date_str: &str, data: &HashMap<String, u64>) -> Result<(), String>;
    fn export_to_json(
        &self,
        format: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
        progress: &ExportProgress,
    ) -> Result<String, String>;
    fn import_from_json(&mut self, json_str: &str, mode: ImportMode) -> Result<(), String>;
    fn try_ping(&self) -> Result<(), String>;
    fn backend_type(&self) -> BackendType;
}

pub struct Database {
    inner: Box<dyn DatabaseBackend>,
    fallback: Option<sqlite::SqliteBackend>,
    is_mongodb: bool,
    sync_mode: FallbackSyncMode,
    disconnected: bool,
}

impl Database {
    pub fn new_sqlite(cfg: &SqliteConfig) -> Self {
        Database {
            inner: Box::new(sqlite::SqliteBackend::new(cfg)),
            fallback: None,
            is_mongodb: false,
            sync_mode: FallbackSyncMode::default(),
            disconnected: false,
        }
    }

    fn new_mongodb_with_fallback(
        mongo_cfg: &MongoConfig,
        fallback_cfg: Option<&FallbackConfig>,
    ) -> Self {
        let backend = Box::new(mongodb::MongoBackend::new(mongo_cfg));
        let (fallback, sync_mode) = fallback_cfg
            .filter(|f| f.enable)
            .map(|f| {
                let sqlite_cfg = SqliteConfig {
                    path: f.path.clone(),
                    table: f.table.clone(),
                };
                (Some(sqlite::SqliteBackend::new(&sqlite_cfg)), f.sync_mode.clone())
            })
            .unwrap_or((None, FallbackSyncMode::Immediate));

        Database {
            inner: backend,
            fallback,
            is_mongodb: true,
            sync_mode,
            disconnected: false,
        }
    }

    pub fn connect(db_cfg: &DatabaseConfig) -> Self {
        match BackendType::from_str(&db_cfg.backend) {
            BackendType::Sqlite => Self::new_sqlite(&db_cfg.sqlite),
            BackendType::MongoDb => {
                Self::new_mongodb_with_fallback(&db_cfg.mongodb, db_cfg.mongodb.fallback.as_ref())
            }
        }
    }

    #[allow(dead_code)]
    pub fn new(db_file: &str) -> Self {
        Self::new_sqlite(&SqliteConfig {
            path: db_file.to_string(),
            table: "daily_stats".to_string(),
        })
    }

    /// Try primary; on failure write to fallback and mark disconnected.
    fn write_with_fallback<F>(&mut self, op_name: &str, primary: F)
    where
        F: Fn(&dyn DatabaseBackend) -> Result<(), String>,
    {
        match primary(self.inner.as_ref()) {
            Ok(()) => {
                if self.disconnected {
                    self.on_reconnect();
                }
            }
            Err(e) => {
                self.disconnected = true;
                twarn!("database", "Primary {} failed: {}", op_name, e);
                if let Some(ref fb) = self.fallback {
                    twarn!("database", "Falling back to local SQLite for {}", op_name);
                    let _ = primary(fb);
                }
            }
        }
    }

    fn read_with_fallback<F, T>(&self, op_name: &str, primary: F) -> T
    where
        F: Fn(&dyn DatabaseBackend) -> Result<T, String>,
        T: Default,
    {
        match primary(self.inner.as_ref()) {
            Ok(v) => v,
            Err(e) => {
                twarn!("database", "Primary {} failed: {}", op_name, e);
                if let Some(ref fb) = self.fallback {
                    twarn!("database", "Reading from fallback SQLite for {}", op_name);
                    primary(fb).unwrap_or_default()
                } else {
                    T::default()
                }
            }
        }
    }

    fn on_reconnect(&mut self) {
        tinfo!("database", "Primary reconnected");
        self.disconnected = false;
        if let Some(ref fb) = self.fallback {
            let sync_immediate = matches!(self.sync_mode, FallbackSyncMode::Immediate);
            if sync_immediate {
                self.sync_from_fallback(fb);
            }
        }
    }

    fn sync_from_fallback(&self, fb: &sqlite::SqliteBackend) {
        tinfo!("database", "Syncing fallback data to primary...");
        let dates = match fb.get_dates() {
            Ok(d) => d,
            Err(e) => {
                twarn!("database", "Failed to read dates from fallback: {}", e);
                return;
            }
        };
        let mut total = 0usize;
        for date in &dates {
            if let Ok(data) = fb.get_stats_for_day(date) {
                if !data.is_empty() {
                    if let Err(e) = self.inner.upsert_day_stats(date, &data) {
                        twarn!("database", "Sync failed for {}: {}", date, e);
                        return;
                    }
                    total += data.len();
                }
            }
        }
        tinfo!("database", "Sync complete: synced {} keys across {} dates", total, dates.len());
        let _ = fb.clear_all();
    }

    /// Attempt to reconnect the primary and sync fallback.
    /// Returns true if reconnection and sync succeeded.
    pub fn try_reconnect(&mut self) -> bool {
        if !self.is_mongodb {
            return true;
        }
        if !self.disconnected {
            return true;
        }
        match self.inner.try_ping() {
            Ok(()) => {
                self.on_reconnect();
                true
            }
            Err(e) => {
                twarn!("database", "Reconnect ping failed: {}", e);
                false
            }
        }
    }

    pub fn get_stats_for_day(&self, date_str: &str) -> HashMap<String, u64> {
        self.read_with_fallback("get_stats_for_day", |b| b.get_stats_for_day(date_str))
    }

    pub fn get_stats_for_range(&self, start_date: &str, end_date: &str) -> HashMap<String, u64> {
        self.read_with_fallback("get_stats_for_range", |b| {
            b.get_stats_for_range(start_date, end_date)
        })
    }

    pub fn upsert_day_stats(&mut self, date_str: &str, data: &HashMap<String, u64>) {
        self.write_with_fallback("upsert_day_stats", |b| b.upsert_day_stats(date_str, data))
    }

    pub fn merge_incremental_stats(&mut self, date_str: &str, data: &HashMap<String, u64>) {
        self.write_with_fallback("merge_incremental_stats", |b| {
            b.merge_incremental_stats(date_str, data)
        })
    }

    pub fn export_to_json(
        &self,
        format: &str,
        start: Option<&str>,
        end: Option<&str>,
        progress: &ExportProgress,
    ) -> String {
        self.read_with_fallback("export_to_json", |b| {
            b.export_to_json(format, start, end, progress)
        })
    }

    pub fn import_from_json(&mut self, json_str: &str, mode: ImportMode) -> Result<(), String> {
        self.inner.import_from_json(json_str, mode)
    }

}
