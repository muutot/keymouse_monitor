use std::sync::Arc;
use std::sync::Mutex;

use chrono::Local;
use chrono::Timelike;
use tokio::time::{sleep, Duration};

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

fn next_min_interval() -> Duration {
    let now = Local::now();
    let next = (now + chrono::Duration::minutes(1))
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let secs = (next - now).num_seconds().max(1) as u64;
    Duration::from_secs(secs)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load();

    println!("Full-featured keyboard and mouse recorder backend starting...");
    println!(
        "Auto-save every {} clicks to {}",
        config.save_threshold, config.db_file
    );
    println!("Open index.html in a browser to view.");

    let db = Arc::new(Mutex::new(Database::new(&config.db_file)));
    let data = Arc::new(Mutex::new(MonitorData::new(&db.lock().unwrap())));

    listener::start(Arc::clone(&data), Arc::clone(&db), config.save_threshold);

    let state = AppState {
        data: Arc::clone(&data),
        db: Arc::clone(&db),
    };

    let data_for_timer = Arc::clone(&data);
    let db_for_timer = Arc::clone(&db);
    tokio::spawn(async move {
        loop {
            sleep(next_min_interval()).await;
            let mut guard = data_for_timer.lock().unwrap();
            guard.save_to_db(&db_for_timer.lock().unwrap());
        }
    });

    let app = api::create_router(state);
    let addr = format!("0.0.0.0:{}", config.port);
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
