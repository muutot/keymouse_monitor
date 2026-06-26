use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
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
use parking_lot::{Mutex, RwLock};
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

/// Per-session export progress channels. Key = session id,
/// value = sender that the export handler pushes to.
type ExportSessions = Arc<RwLock<HashMap<String, watch::Sender<Option<(u64, u64, bool)>>>>>;

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<MonitorData>>,
    pub db: Arc<Mutex<Database>>,
    pub change_tx: watch::Sender<()>,
    pub client_count: Arc<AtomicUsize>,
    pub export_sessions: ExportSessions,
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

/// Ensures the per-session entry is cleaned up when the SSE stream
/// is dropped (client disconnect) or completes normally.
struct SessionGuard {
    sessions: ExportSessions,
    sid: String,
    cleaned: bool,
}

impl SessionGuard {
    fn new(sessions: ExportSessions, sid: String) -> Self {
        Self {
            sessions,
            sid,
            cleaned: false,
        }
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if !self.cleaned {
            self.sessions.write().remove(&self.sid);
        }
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
        .route(
            "/api/export/progress/stream",
            get(export_progress_sse_handler),
        )
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
        let db = db.lock();
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
    pub session: Option<String>,
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
    if let (Some(start), Some(end)) = (params.start.as_deref(), params.end.as_deref()) {
        if start > end {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "start must not be after end"})),
            ));
        }
    }
    let pretty = params.pretty.unwrap_or(false);
    let db = state.db.clone();
    let fmt_owned = fmt.to_string();
    let start = params.start.clone();
    let end = params.end.clone();
    let session_id = params.session.clone().unwrap_or_default();

    // Create a per-session progress channel so SSE clients can subscribe
    let tx = if !session_id.is_empty() {
        Some(
            state
                .export_sessions
                .write()
                .entry(session_id.clone())
                .or_insert_with(|| watch::channel(None).0)
                .clone(),
        )
    } else {
        None
    };

    tdebug!(
        "export",
        "Starting export: format={}, start={:?}, end={:?}, pretty={}, session={}",
        fmt,
        start,
        end,
        pretty,
        session_id,
    );

    let progress = Arc::new(ExportProgress::new());

    // Spawn progress poller only when a session channel exists for reporting
    if let Some(tx) = tx.clone() {
        let poll_progress = progress.clone();
        tokio::spawn(async move {
            let mut last_logged = (u64::MAX, u64::MAX, false);
            loop {
                let done = poll_progress.done.load(Ordering::Relaxed);
                let total = poll_progress.total.load(Ordering::Relaxed);
                let current = poll_progress.current.load(Ordering::Relaxed);
                if (current, total, done) != last_logged {
                    tdebug!(
                        "export",
                        "progress tick: current={}, total={}, done={}",
                        current,
                        total,
                        done
                    );
                    last_logged = (current, total, done);
                }
                if done || (total > 0 && current >= total) {
                    tdebug!("export", "progress done, sending completion sentinel");
                    let _ = tx.send(Some((total, total, true)));
                    break;
                }
                if tx.send(Some((current, total, false))).is_err() {
                    tdebug!("export", "no SSE receivers, stopping progress poller");
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }

    let export_prog = progress.clone();

    // Mark export as done and clean up the session map entry.
    // The poller (above) is responsible for sending the completion sentinel.
    let signal_done = |progress: &ExportProgress, sessions: &ExportSessions, sid: &str| {
        progress.done.store(true, Ordering::Relaxed);
        tdebug!("export", "done flag set");
        if !sid.is_empty() {
            sessions.write().remove(sid);
            tdebug!("export", "session {} cleaned up", sid);
        }
    };

    let json_result = match tokio::task::spawn_blocking(move || {
        let db = db.lock();
        db.export_to_json(&fmt_owned, start.as_deref(), end.as_deref(), &export_prog)
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            signal_done(&progress, &state.export_sessions, &session_id);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database export task panicked."})),
            ));
        }
    };

    tdebug!(
        "export",
        "export_to_json returned {} bytes",
        json_result.len()
    );
    signal_done(&progress, &state.export_sessions, &session_id);

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
    Query(params): Query<HashMap<String, String>>,
) -> Json<Value> {
    let val = (|| -> Option<(u64, u64, bool)> {
        let sid = params.get("session")?;
        let sessions = state.export_sessions.read();
        let tx = sessions.get(sid)?;
        *tx.subscribe().borrow()
    })();
    progress_json(val)
}

fn progress_json(val: Option<(u64, u64, bool)>) -> Json<Value> {
    Json(progress_value_inner(val))
}

fn progress_value_inner(val: Option<(u64, u64, bool)>) -> Value {
    match val {
        Some((_current, _total, true)) => {
            json!({ "current": 0, "total": 0, "done": true, "idle": false })
        }
        Some((current, total, false)) => {
            let done = total > 0 && current >= total;
            json!({
                "current": current.min(total),
                "total": total,
                "done": done,
            })
        }
        None => json!({ "current": 0, "total": 0, "done": false, "idle": true }),
    }
}

async fn export_progress_sse_handler(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Sse<impl futures::stream::Stream<Item = Result<Event, Infallible>>> {
    let session_id = params.get("session").cloned().unwrap_or_default();
    tdebug!("export", "SSE client connected, session={}", session_id);

    // If the export handler already created a channel (started before SSE
    // connected), reuse it; otherwise create one now.
    let rx = if !session_id.is_empty() {
        let mut map = state.export_sessions.write();
        let tx = map
            .entry(session_id.clone())
            .or_insert_with(|| watch::channel(None).0);
        tx.subscribe()
    } else {
        let (_, rx) = watch::channel(None);
        rx
    };

    let guard = SessionGuard::new(state.export_sessions.clone(), session_id.clone());

    let stream = futures::stream::unfold(
        (rx, guard, false),
        |(mut rx, mut guard, mut done)| async move {
            if done {
                guard.cleaned = true;
                let _ = guard.sessions.write().remove(&guard.sid);
                return None;
            }
            rx.changed().await.ok()?;
            let val = *rx.borrow_and_update();
            let json = progress_value(val);
            if let Some((_, _, true)) = val {
                done = true;
            }
            Some((Ok(Event::default().data(json)), (rx, guard, done)))
        },
    );

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

fn progress_value(val: Option<(u64, u64, bool)>) -> String {
    progress_value_inner(val).to_string()
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
        let (has_today, today_counts): (bool, HashMap<String, u64>) =
            serde_json::from_str(&json_str)
                .ok()
                .and_then(|v: Value| {
                    v.get("records").and_then(|r| r.as_object()).map(|records| {
                        let counts = records
                            .get(&today)
                            .cloned()
                            .and_then(|v| serde_json::from_value(v).ok())
                            .unwrap_or_default();
                        (records.contains_key(&today), counts)
                    })
                })
                .unwrap_or_default();

        {
            let mut db_guard = db.lock();
            db_guard.import_from_json(&json_str, mode)?;
        }
        if has_today || !today_counts.is_empty() {
            let mut guard = data.write();
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
            // Coalesce rapid key events into a single push
            if !first {
                tokio::time::sleep(Duration::from_millis(50)).await;
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
            Some((
                Ok(Event::default().data(json)),
                (data, rx, false, last, guard),
            ))
        },
    );

    Sse::new(stream)
}
