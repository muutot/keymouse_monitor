#![windows_subsystem = "windows"]

use std::collections::HashMap;
use std::sync::{atomic::AtomicUsize, Arc, OnceLock};
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use tokio::sync::{watch, Notify};

mod api;
mod config;
mod data;
mod database;
mod listener;
mod log;

use api::AppState;
use config::{Config, UpdateMode};
use data::MonitorData;
use database::Database;

#[cfg(windows)]
static OS_SHUTDOWN: OnceLock<Notify> = OnceLock::new();

#[cfg(windows)]
unsafe extern "system" fn console_ctrl_handler(_: u32) -> i32 {
    if let Some(n) = OS_SHUTDOWN.get() {
        // notify_one (not notify_waiters): if the waiter hasn't been
        // registered yet, a permit is stored and the next
        // notified().await will complete immediately.  notify_waiters
        // would drop the signal entirely in that race.
        n.notify_one();
    }
    1
}

#[cfg(windows)]
fn init_console() {
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

#[cfg(windows)]
fn should_show_console() -> bool {
    let args: Vec<String> = std::env::args().collect();
    args.iter().any(|a| a == "--console" || a == "-c")
}

fn check_help() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!(
            "keymouse-monitor — real-time keyboard & mouse click statistics

USAGE:
    keymouse-monitor.exe [OPTIONS]

OPTIONS:
    -c, --console    Attach a console window (hidden by default, Windows only)
    -h, --help       Print this help message and exit
"
        );
        std::process::exit(0);
    }
}

async fn wait_for_shutdown() {
    #[cfg(windows)]
    {
        let notify = OS_SHUTDOWN.get_or_init(Notify::new);
        unsafe {
            windows_sys::Win32::System::Console::SetConsoleCtrlHandler(
                Some(console_ctrl_handler),
                1,
            );
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {},
            _ = notify.notified() => {},
        }
    }
    #[cfg(not(windows))]
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}

#[tokio::main]
async fn main() {
    check_help();
    #[cfg(windows)]
    if should_show_console() {
        init_console();
    }

    let config = Config::load();

    log::init_logger(&config.log);

    tinfo!(
        "main",
        "Full-featured keyboard and mouse recorder backend starting..."
    );
    tinfo!("main", "Backend: {}", config.database.backend);
    tinfo!("main", "Open index.html in a browser to view.");

    // Initialize DB + MonitorData outside tokio runtime context.
    // MongoDB driver uses its own internal Runtime and must NOT be
    // called from inside a tokio async context.
    let db_cfg = config.database.clone();
    let (db, data): (Arc<Mutex<Database>>, Arc<RwLock<MonitorData>>) =
        tokio::task::spawn_blocking(move || {
            let database = Arc::new(Mutex::new(Database::connect(&db_cfg)));
            let monitor_data = Arc::new(RwLock::new(MonitorData::new(&database.lock())));
            (database, monitor_data)
        })
        .await
        .expect("Failed to initialize database");

    let client_count = Arc::new(AtomicUsize::new(0));
    let (change_tx, _) = watch::channel(());
    listener::start(
        listener::ListenerKind::from_str(&config.listener),
        Arc::clone(&data),
        change_tx.clone(),
        Arc::clone(&client_count),
    );

    let save_interval = config.save_interval_secs;
    let data_for_timer = Arc::clone(&data);
    let db_for_timer = Arc::clone(&db);
    let update_mode_timer = config.update_mode.clone();
    let update_mode_shutdown = config.update_mode.clone();

    let state = AppState {
        data: Arc::clone(&data),
        db: Arc::clone(&db),
        change_tx,
        client_count,
        export_sessions: Arc::new(RwLock::new(HashMap::new())),
    };

    // Cooperative shutdown: signal the timer to stop, then wait for it
    // to finish its current save before acquiring locks ourselves.
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let timer_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(save_interval));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // tokio::time::interval's first tick completes immediately, so this
        // pre-tick ensures the first real save happens *now* on startup
        // rather than after a full save_interval delay.  Without it, any
        // keypress in the first save_interval seconds would be lost on
        // a crash.
        interval.tick().await;
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown_rx.changed() => break,
            }
            let data = Arc::clone(&data_for_timer);
            let db = Arc::clone(&db_for_timer);
            let mode = update_mode_timer.clone();
            let _ = tokio::task::spawn_blocking(move || {
                // Try reconnecting fallback primary (e.g. MongoDB) each tick
                {
                    let mut db_guard = db.lock();
                    let _ = db_guard.try_reconnect();
                }
                enum Action {
                    Diff {
                        date: String,
                        delta: HashMap<String, u64>,
                    },
                    Full {
                        date: String,
                        snapshot: HashMap<String, u64>,
                    },
                    Rollover {
                        old_date: String,
                        yesterday: HashMap<String, u64>,
                        today: String,
                        today_base: HashMap<String, u64>,
                    },
                    Nothing,
                }
                let action: Action = {
                    let mut guard = data.write();
                    match guard.prepare_save() {
                        Some(result) if result.is_rollover => Action::Rollover {
                            old_date: result.date,
                            yesterday: result.yesterday_snapshot,
                            today: guard.today.clone(),
                            today_base: guard.base_counts.clone(),
                        },
                        Some(result) => match mode {
                            UpdateMode::Diff => Action::Diff {
                                date: result.date,
                                delta: result.delta,
                            },
                            UpdateMode::Full => Action::Full {
                                date: result.date,
                                snapshot: guard.get_key_counts(),
                            },
                        },
                        None => Action::Nothing,
                    }
                };
                let mut db_guard = db.lock();
                match action {
                    Action::Nothing => {}
                    Action::Diff { date, delta } => {
                        db_guard.merge_incremental_stats(&date, &delta);
                    }
                    Action::Full { date, snapshot } => {
                        db_guard.upsert_day_stats(&date, &snapshot);
                    }
                    Action::Rollover {
                        old_date,
                        yesterday,
                        today,
                        today_base,
                    } => {
                        db_guard.upsert_day_stats(&old_date, &yesterday);
                        if !today_base.is_empty() {
                            db_guard.upsert_day_stats(&today, &today_base);
                        }
                    }
                }
            })
            .await;
        }
    });

    let app = api::create_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    tinfo!("main", "Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("Failed to bind to {}: {}", addr, e));
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async { std::future::pending::<()>().await })
            .await;
    });

    wait_for_shutdown().await;
    tinfo!("main", "Shutdown signal received, saving data...");
    let _ = shutdown_tx.send(true);

    server_handle.abort();
    let _ = server_handle.await;

    // Wait for timer to finish its current save before touching data
    let _ = timer_task.await;

    // Must use spawn_blocking for MongoDB driver (it calls rt.block_on internally)
    let data_clone = Arc::clone(&data);
    let db_clone = Arc::clone(&db);
    let mode_shutdown = update_mode_shutdown;
    tokio::task::spawn_blocking(move || {
        let mut guard = data_clone.write();
        let mut db_guard = db_clone.lock();
        guard.save_to_db(&mut db_guard, &mode_shutdown);
    })
    .await
    .unwrap_or_else(|e| terror!("main", "Final save failed: {}", e));

    tinfo!("main", "Data saved. Goodbye!");

    // Exit immediately to avoid dropping MongoBackend.rt on a tokio worker
    // during runtime shutdown (which panics). OS reclaims all resources.
    std::process::exit(0);
}
