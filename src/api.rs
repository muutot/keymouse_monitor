use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        Json,
    },
    routing::{get, post},
    Router,
};
use chrono::{Local, NaiveDate};
use futures::stream::Stream;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::{
    data::MonitorData,
    database::{Database, ExportProgress, ImportMode},
    tdebug, tinfo,
};

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<MonitorData>>,
    pub db: Arc<Mutex<Database>>,
    pub change_tx: watch::Sender<()>,
    pub client_count: Arc<AtomicUsize>,
    pub export_progress: watch::Sender<Option<(u64, u64)>>,
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
        .route("/api/export/progress", get(export_progress_handler))
        .route("/api/export/progress/stream", get(export_progress_sse_handler))
        .route("/api/import", post(import_handler))
        .route("/api/version", get(version_handler))
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
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    NaiveDate::parse_from_str(&params.start, "%Y-%m-%d").map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid date format, please use YYYY-MM-DD."})),
        )
    })?;
    NaiveDate::parse_from_str(&params.end, "%Y-%m-%d").map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid date format, please use YYYY-MM-DD."})),
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
            Json(json!({"error": "Database query failed."})),
        )
    })?;
    Ok(Json(json!(result)))
}

#[derive(Deserialize)]
pub struct ExportParams {
    pub format: Option<String>,
    pub pretty: Option<bool>,
    pub start: Option<String>,
    pub end: Option<String>,
}

async fn export_handler(
    State(state): State<AppState>,
    Query(params): Query<ExportParams>,
) -> Result<axum::response::Response, (StatusCode, Json<Value>)> {
    let fmt = params.format.as_deref().unwrap_or("nested");
    if fmt != "nested" && fmt != "flat" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid format, use 'nested' or 'flat'."})),
        ));
    }
    if params.start.as_deref().unwrap_or("") > params.end.as_deref().unwrap_or("") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "start must not be after end"})),
        ));
    }
    let pretty = params.pretty.unwrap_or(false);
    let db = state.db.clone();
    let fmt_owned = fmt.to_string();
    let start = params.start.clone();
    let end = params.end.clone();

    // Reset shared progress so a new export starts clean (no stale state)
    let _ = state.export_progress.send(None);

    tdebug!("export", "Starting export: format={}, start={:?}, end={:?}, pretty={}", fmt, start, end, pretty);

    let progress = Arc::new(ExportProgress::new());
    let tx = state.export_progress.clone();
    let tx2 = tx.clone();
    let poll_progress = progress.clone();
    tokio::spawn(async move {
        let mut last_logged = (u64::MAX, u64::MAX, false);
        loop {
            let done = poll_progress.done.load(Ordering::Relaxed);
            let total = poll_progress.total.load(Ordering::Relaxed);
            let current = poll_progress.current.load(Ordering::Relaxed);
            if (current, total, done) != last_logged {
                tdebug!("export", "progress tick: current={}, total={}, done={}", current, total, done);
                last_logged = (current, total, done);
            }
            if done || (total > 0 && current >= total) {
                tdebug!("export", "progress done, sending completion sentinel");
                let _ = tx2.send(Some((u64::MAX, u64::MAX)));
                break;
            }
            let _ = tx2.send(Some((current, total)));
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    let export_prog = progress.clone();
    let json_result = tokio::task::spawn_blocking(move || {
        let db = db.lock().unwrap();
        db.export_to_json(&fmt_owned, start.as_deref(), end.as_deref(), &export_prog)
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database export task panicked."})),
        )
    })?;

    tdebug!("export", "export_to_json returned {} bytes", json_result.len());
    progress.done.store(true, Ordering::Relaxed);
    let _ = tx.send(Some((u64::MAX, u64::MAX)));
    tdebug!("export", "done flag set, completion sentinel sent");

    if json_result.is_empty() {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Export returned no data — check server logs for details."})),
        ));
    }

    let body = if pretty {
        let value: Value = serde_json::from_str(&json_result).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("Export generated invalid JSON: {}", e)})),
            )
        })?;
        serde_json::to_string_pretty(&value).expect("Value came from valid JSON")
    } else {
        json_result
    };
    Ok(axum::response::Response::builder()
        .header("content-type", "application/json")
        .header("cache-control", "no-store, no-cache, must-revalidate")
        .body(axum::body::Body::from(body))
        .expect("static response builder"))
}

async fn export_progress_handler(
    State(state): State<AppState>,
) -> Json<Value> {
    let val = *state.export_progress.subscribe().borrow();
    progress_json(val)
}

fn progress_json(val: Option<(u64, u64)>) -> Json<Value> {
    match val {
        Some((_, total)) if total == u64::MAX => {
            // Sentinel completion signal — never leak u64::MAX to JS
            Json(json!({ "current": 0, "total": 0, "done": true, "idle": false }))
        }
        Some((current, total)) => {
            let done = total > 0 && current >= total;
            Json(json!({
                "current": current.min(total),
                "total": total,
                "done": done,
            }))
        }
        None => Json(json!({ "current": 0, "total": 0, "done": false, "idle": true })),
    }
}

async fn export_progress_sse_handler(
    State(state): State<AppState>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>> {
    tdebug!("export", "SSE client connected");
    // Reset so a new client never sees stale completion from a prior export
    let _ = state.export_progress.send(None);
    let rx = state.export_progress.subscribe();

    let stream = futures::stream::unfold(
        (rx, None::<(u64, u64)>),
        |(mut rx, last)| async move {
            rx.changed().await.ok()?;
            let val = *rx.borrow_and_update();
            let json = progress_value(val);
            let raw = val.unwrap_or((0, 0));
            if Some(raw) != last {
                tdebug!("export", "SSE pushing: {} (raw={:?})", json, raw);
            }
            Some((Ok(Event::default().data(json)), (rx, Some(raw))))
        },
    );

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

fn progress_value(val: Option<(u64, u64)>) -> String {
    match val {
        Some((_, total)) if total == u64::MAX => {
            serde_json::json!({"current": 0, "total": 0, "done": true, "idle": false}).to_string()
        }
        Some((current, total)) => {
            let done = total > 0 && current >= total;
            serde_json::json!({
                "current": current.min(total),
                "total": total,
                "done": done,
            })
            .to_string()
        }
        None => serde_json::json!({"current": 0, "total": 0, "done": false, "idle": true}).to_string(),
    }
}

#[derive(Deserialize)]
pub struct ImportParams {
    pub mode: Option<String>,
}

async fn import_handler(
    State(state): State<AppState>,
    Query(params): Query<ImportParams>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let mode = ImportMode::from_str(params.mode.as_deref().unwrap_or("overwrite"));
    let json_str = serde_json::to_string(&payload).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Invalid JSON body: {}", e)})),
        )
    })?;
    let db = state.db.clone();
    let data = state.data.clone();
    let today = Local::now().format("%Y-%m-%d").to_string();
    let start = Instant::now();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let today_counts: HashMap<String, u64> = serde_json::from_str(&json_str)
            .ok()
            .and_then(|v: Value| {
                v.get("records")
                    .and_then(|r| r.as_object())
                    .and_then(|records| records.get(&today).cloned())
                    .and_then(|v| serde_json::from_value(v).ok())
            })
            .unwrap_or_default();

        let mut guard = data.write();
        let mut db_guard = db.lock().unwrap();
        db_guard.import_from_json(&json_str, mode)?;
        if !today_counts.is_empty() {
            guard.import_today_data(&today_counts, mode);
        }
        Ok(())
    })
    .await
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Database import task panicked."})),
        )
    })?
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("Import failed: {}", e)})),
        )
    })?;
    let duration_ms = start.elapsed().as_millis();
    tinfo!(
        "api",
        "Import successful (mode: {:?}, duration: {}ms).",
        mode,
        duration_ms
    );
    Ok(Json(
        json!({ "status": "ok", "message": "Import successful", "mode": format!("{:?}", mode), "duration_ms": duration_ms }),
    ))
}

async fn version_handler() -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
    }))
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let data = state.data.clone();
    let rx = state.change_tx.subscribe();
    let guard = SseConnectionGuard::new(state.client_count.clone());

    let stream = futures::stream::unfold(
        (data, rx, true, HashMap::<String, u64>::new(), guard),
        |(data, mut rx, first, mut last, guard)| async move {
            if !first && rx.changed().await.is_err() {
                return None;
            }
            let json = {
                let current = {
                    let guard = data.read();
                    guard.get_key_counts()
                };
                if first {
                    // Initial connection: send the entire snapshot.
                    last = current.clone();
                    serde_json::to_string(&current).unwrap()
                } else {
                    // Compute the delta of changed keys since the last push.
                    let mut delta = serde_json::Map::new();
                    for (key, count) in &current {
                        if last.get(key) != Some(count) {
                            delta.insert(key.clone(), serde_json::json!(count));
                        }
                    }
                    // Detect keys that disappeared (shouldn't happen in our
                    // data model, but defensively).
                    for key in last.keys() {
                        if !current.contains_key(key) {
                            delta.insert(key.clone(), serde_json::json!(0));
                        }
                    }
                    last = current;
                    serde_json::to_string(&delta).unwrap()
                }
            };
            Some((Ok(Event::default().data(json)), (data, rx, false, last, guard)))
        },
    );

    Sse::new(stream)
}
