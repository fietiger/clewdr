use std::time::Instant;

use axum::{
    Json,
    extract::{Query, Request},
    middleware::Next,
    response::Response,
};
use axum_auth::AuthBearer;
use serde::Deserialize;
use serde_json::{Value, json};

use super::error::ApiError;
use crate::{
    config::CLEWDR_CONFIG,
    services::traffic::{RECORDER, RequestRecord, now_epoch},
};

/// Records every proxied request so the dashboard can chart traffic.
///
/// Mounted only on the inference routers. The dashboard's own polling lives
/// under `/api`, and recording that would make the chart count itself — the
/// traffic page would show a steady floor of its own refresh requests.
pub async fn record_traffic(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let started = Instant::now();
    let response = next.run(req).await;
    RECORDER.record(RequestRecord {
        ts: now_epoch(),
        method,
        path,
        status: response.status().as_u16(),
        latency_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
    });
    response
}

/// Query parameters for `/api/stats`.
#[derive(Deserialize)]
pub struct StatsQuery {
    window: Option<String>,
}

/// Maps a window name to `(window seconds, bucket seconds)`.
///
/// An unknown name falls back to the 24-hour shape rather than erroring: the
/// chart is a read-only view, and a stale bookmark should still render.
fn window_shape(window: &str) -> (u64, u64) {
    match window {
        "1h" => (3600, 60),
        "7d" => (7 * 86_400, 6 * 3600),
        _ => (86_400, 3600),
    }
}

/// Traffic counts per time bucket.
///
/// # Errors
/// [`ApiError::unauthorized`] if the bearer token is not the admin password.
pub async fn api_get_stats(
    AuthBearer(t): AuthBearer,
    Query(query): Query<StatsQuery>,
) -> Result<Json<Value>, ApiError> {
    if !CLEWDR_CONFIG.load().admin_auth(&t) {
        return Err(ApiError::unauthorized());
    }
    let window = query.window.unwrap_or_else(|| "24h".into());
    let (window_secs, bucket_secs) = window_shape(&window);
    let now = now_epoch();
    let buckets = RECORDER.buckets(window_secs, bucket_secs, now);
    let error: u64 = buckets.iter().map(|b| b.error).sum();
    let total: u64 = buckets.iter().map(|b| b.ok + b.error).sum();

    Ok(Json(json!({
        "window": window,
        "window_seconds": window_secs,
        "bucket_seconds": bucket_secs,
        "server_time": now,
        "totals": { "total": total, "ok": total - error, "error": error },
        "buckets": buckets,
    })))
}

/// Query parameters for `/api/requests`.
#[derive(Deserialize)]
pub struct RequestsQuery {
    limit: Option<usize>,
}

/// The most recent proxied requests, newest first.
///
/// # Errors
/// [`ApiError::unauthorized`] if the bearer token is not the admin password.
pub async fn api_get_requests(
    AuthBearer(t): AuthBearer,
    Query(query): Query<RequestsQuery>,
) -> Result<Json<Value>, ApiError> {
    if !CLEWDR_CONFIG.load().admin_auth(&t) {
        return Err(ApiError::unauthorized());
    }
    let limit = query.limit.unwrap_or(50).clamp(1, 500);
    Ok(Json(json!({ "requests": RECORDER.recent(limit) })))
}
