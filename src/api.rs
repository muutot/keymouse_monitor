use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::Json;
use axum::Router;
use axum::routing::{get, post};
use chrono::NaiveDate;
use futures::stream::Stream;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::data::MonitorData;
use crate::database::Database;

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<MonitorData>>,
    pub db: Arc<Mutex<Database>>,
    pub change_tx: watch::Sender<()>,
    pub client_count: Arc<AtomicUsize>,
}

struct SseConnectionGuard {
    count: Arc<AtomicUsize>,
}

impl SseConnectionGuard {
    fn new(count: Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::Relaxed);
        SseConnectionGuard { count }
    }
}

impl Drop for SseConnectionGuard {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Deserialize)]
pub struct HistoryParams {
    pub start: String,
    pub end: String,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/keycounts", get(get_keycounts))
        .route("/history", get(get_history))
        .route("/events", get(sse_handler))
        .route("/api/export", get(export_handler))
        .route("/api/import", post(import_handler))
        .nest_service("/static", ServeDir::new("static"))
        .fallback_service(ServeDir::new(".").append_index_html_on_directories(true))
        .layer(cors)
        .with_state(state)
}

async fn get_keycounts(State(state): State<AppState>) -> Json<Value> {
    let guard = state.data.read();
    let counts = guard.get_key_counts();
    Json(json!(counts))
}

async fn get_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<Value>, (StatusCode, String)> {
    NaiveDate::parse_from_str(&params.start, "%Y-%m-%d")
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid date format, please use YYYY-MM-DD.".to_string(),
            )
        })?;
    NaiveDate::parse_from_str(&params.end, "%Y-%m-%d")
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Invalid date format, please use YYYY-MM-DD.".to_string(),
            )
        })?;

    let db = state.db.clone();
    let start = params.start.clone();
    let end = params.end.clone();
    let result = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.get_stats_for_range(&start, &end)
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database query failed.".to_string(),
        )
    })?;
    Ok(Json(json!(result)))
}

async fn export_handler(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let db = state.db.clone();
    let json_str = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.export_to_json()
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database export failed.".to_string(),
        )
    })?;
    let value: Value = serde_json::from_str(&json_str).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to serialize export: {}", e),
        )
    })?;
    Ok(Json(value))
}

async fn import_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let json_str = serde_json::to_string(&payload).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Invalid JSON body: {}", e),
        )
    })?;
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || {
        let mut db = db.lock().unwrap();
        db.import_from_json(&json_str);
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database import failed.".to_string(),
        )
    })?;
    Ok(Json(serde_json::json!({ "status": "ok", "message": "Import successful" })))
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let data = state.data.clone();
    let rx = state.change_tx.subscribe();
    let guard = SseConnectionGuard::new(state.client_count.clone());

    let stream = futures::stream::unfold(
        (data, rx, true, guard),
        |(data, mut rx, first, guard)| async move {
            if !first {
                let _ = rx.changed().await;
            }
            let json = {
                let guard = data.read();
                serde_json::to_string(&guard.get_key_counts()).unwrap()
            };
            Some((Ok(Event::default().data(json)), (data, rx, false, guard)))
        },
    );

    Sse::new(stream)
}
