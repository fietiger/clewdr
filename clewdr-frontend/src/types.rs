pub use clewdr_types::{
    ConfigApi as ConfigData, CookieStatusApi as CookieStatus,
    CookieStatusInfoApi as CookieStatusInfo, Reason, UsageBreakdown,
    UselessCookieApi as UselessCookie,
};

use serde::Deserialize;

/// One time bucket of request counts, from `GET /api/stats`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TrafficBucket {
    /// Epoch seconds of the bucket's start.
    pub ts: u64,
    pub ok: u64,
    pub error: u64,
}

impl TrafficBucket {
    /// Responses in this bucket, successful or not.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.ok + self.error
    }
}

/// Totals for the selected window.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TrafficTotals {
    pub total: u64,
    pub ok: u64,
    pub error: u64,
}

/// `GET /api/stats` response.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TrafficStats {
    #[serde(default)]
    pub window: String,
    #[serde(default)]
    pub bucket_seconds: u64,
    #[serde(default)]
    pub totals: TrafficTotals,
    #[serde(default)]
    pub buckets: Vec<TrafficBucket>,
}

/// One entry from `GET /api/requests`.
#[derive(Debug, Clone, Deserialize)]
pub struct RequestEntry {
    pub ts: u64,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u64,
}

/// `GET /api/requests` response.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RequestLog {
    #[serde(default)]
    pub requests: Vec<RequestEntry>,
}
