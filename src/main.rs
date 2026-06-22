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
mod maps;

use api::AppState;
use config::Config;
use data::MonitorData;
use database::Database;

#[cfg(windows)]
fn init_console() {
    unsafe {
        windows_sys::Win32::System::Console::AllocConsole();
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

    tracing_subscriber::fmt::init();

    let config = Config::load();

    println!("Full-featured keyboard and mouse recorder backend starting...");
    println!("Database: {}", config.db_file);
    println!("Open index.html in a browser to view.");

    let db = Arc::new(Mutex::new(Database::new(&config.db_file)));
    let data = Arc::new(RwLock::new(MonitorData::new(&db.lock().unwrap())));

    let client_count = Arc::new(AtomicUsize::new(0));
    let (change_tx, _) = watch::channel(());
    listener::start(Arc::clone(&data), change_tx.clone(), Arc::clone(&client_count));

    let state = AppState {
        data: Arc::clone(&data),
        db: Arc::clone(&db),
        change_tx,
        client_count,
    };

    let data_for_timer = Arc::clone(&data);
    let db_for_timer = Arc::clone(&db);
    tokio::task::spawn_blocking(move || loop {
        std::thread::sleep(Duration::from_secs(60));
        let mut guard = data_for_timer.write();
        guard.save_to_db(&db_for_timer.lock().unwrap());
    });

    let app = api::create_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
