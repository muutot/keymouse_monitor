use std::sync::Arc;
use std::sync::Mutex;

use axum::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Json;
use axum::Router;
use axum::routing::get;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::data::MonitorData;
use crate::database::Database;

#[derive(Clone)]
pub struct AppState {
    pub data: Arc<Mutex<MonitorData>>,
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
        .nest_service("/static", ServeDir::new("static"))
        .fallback_service(ServeDir::new(".").append_index_html_on_directories(true))
        .layer(cors)
        .with_state(state)
}

async fn get_keycounts(State(state): State<AppState>) -> Json<Value> {
    let guard = state.data.lock().unwrap();
    let counts = guard.get_key_counts();
    Json(json!(counts))
}

async fn get_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> Result<Json<Value>, (StatusCode, String)> {
    // Validate date format
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
