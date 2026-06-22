use std::convert::Infallible;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::Json;
use axum::Router;
use axum::routing::get;
use chrono::NaiveDate;
use futures::stream::Stream;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::data::MonitorData;
use crate::database::Database;

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<RwLock<MonitorData>>,
    pub db: Arc<Mutex<Database>>,
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

    let db = state.db.lock().unwrap();
    let result = db.get_stats_for_range(&params.start, &params.end);
    Ok(Json(json!(result)))
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let data = state.data.clone();

    let stream = futures::stream::unfold(data, |data| async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let json = {
            let guard = data.read();
            serde_json::to_string(&guard.get_key_counts()).unwrap()
        };
        Some((Ok(Event::default().data(json)), data))
    });

    Sse::new(stream)
}
