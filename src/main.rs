#![windows_subsystem = "windows"]

use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

use parking_lot::RwLock;
use tokio::sync::watch;

mod api;
mod config;
mod data;
mod database;
mod listener;
mod log;
mod maps;

use api::AppState;
use config::Config;
use data::MonitorData;
use database::Database;

#[cfg(windows)]
fn init_console() {
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}

fn should_show_console() -> bool {
    let args: Vec<String> = std::env::args().collect();
    args.iter().any(|a| a == "--console" || a == "-c")
}

#[tokio::main]
async fn main() {
    #[cfg(windows)]
    if should_show_console() {
        init_console();
    }

    let config = Config::load();

    log::init_logger(&config.log);

    tinfo!("main", "Full-featured keyboard and mouse recorder backend starting...");
    tinfo!("main", "Backend: {}", config.database.backend);
    tinfo!("main", "Open index.html in a browser to view.");

    // Initialize DB + MonitorData outside tokio runtime context.
    // MongoDB driver uses its own internal Runtime and must NOT be
    // called from inside a tokio async context.
    let db_cfg = config.database.clone();
    let (db, data): (Arc<Mutex<Database>>, Arc<RwLock<MonitorData>>) =
        tokio::task::spawn_blocking(move || {
            let database = Arc::new(Mutex::new(Database::connect(&db_cfg)));
            let monitor_data = Arc::new(RwLock::new(MonitorData::new(
                &database.lock().unwrap(),
            )));
            (database, monitor_data)
        })
        .await
        .expect("Failed to initialize database");

    let client_count = Arc::new(AtomicUsize::new(0));
    let (change_tx, _) = watch::channel(());
    listener::start(&config.listener, Arc::clone(&data), change_tx.clone(), Arc::clone(&client_count));

    let state = AppState {
        data: Arc::clone(&data),
        db: Arc::clone(&db),
        change_tx,
        client_count,
    };

    // Cooperative shutdown: signal the timer to stop, then wait for it
    // to finish its current save before acquiring locks ourselves.
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    let save_interval = config.save_interval_secs;
    let data_for_timer = Arc::clone(&data);
    let db_for_timer = Arc::clone(&db);
    let timer_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(save_interval));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = shutdown_rx.changed() => break,
            }
            let data = Arc::clone(&data_for_timer);
            let db = Arc::clone(&db_for_timer);
            let _ = tokio::task::spawn_blocking(move || {
                // Step 1: extract snapshot under data lock (fast, microseconds)
                let snapshot = {
                    let mut guard = data.write();
                    guard.prepare_save()
                };
                // Step 2: save to DB without holding the data lock (slow, network)
                if let Some((date, counts)) = snapshot {
                    let db = db.lock().unwrap();
                    db.upsert_day_stats(&date, &counts);
                }
            }).await;
        }
    });

    let app = api::create_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    tinfo!("main", "Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
            tinfo!("main", "\nShutdown signal received, saving data...");
            let _ = shutdown_tx.send(true);
        })
        .await
        .unwrap();

    // Wait for timer to finish its current save before touching data
    let _ = timer_task.await;

    let mut guard = data.write();
    guard.save_to_db(&db.lock().unwrap());
    tinfo!("main", "Data saved. Goodbye!");
}
