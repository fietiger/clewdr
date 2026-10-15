//! In-memory record of recent proxy traffic.
//!
//! Feeds the dashboard's traffic view: a bounded ring buffer of the requests
//! the proxy has served, plus the per-bucket counts the chart draws.
//!
//! Memory only, and deliberately so. The buffer is capped, so a long-running
//! process cannot grow without limit, and a restart starting from empty is the
//! correct answer rather than a loss — there is no durable traffic history to
//! lose.

use std::{
    collections::VecDeque,
    sync::{LazyLock, Mutex, PoisonError},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

/// How many requests the ring buffer keeps.
const CAPACITY: usize = 2000;

/// One recorded request.
#[derive(Debug, Clone, Serialize)]
pub struct RequestRecord {
    /// Epoch seconds, when the response completed.
    pub ts: u64,
    pub method: String,
    pub path: String,
    pub status: u16,
    pub latency_ms: u64,
}

/// One time bucket of request counts.
#[derive(Debug, Clone, Serialize)]
pub struct Bucket {
    /// Epoch seconds of the bucket's start.
    pub ts: u64,
    /// Responses with a status below 400.
    pub ok: u64,
    /// Responses with a status of 400 or above.
    pub error: u64,
}

/// The process-wide request recorder.
pub static RECORDER: LazyLock<Recorder> = LazyLock::new(Recorder::default);

/// Bounded, thread-safe store of recent requests.
#[derive(Debug)]
pub struct Recorder {
    records: Mutex<VecDeque<RequestRecord>>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            records: Mutex::new(VecDeque::with_capacity(CAPACITY)),
        }
    }
}

impl Recorder {
    /// Appends a record, evicting the oldest once the buffer is full.
    pub fn record(&self, record: RequestRecord) {
        let mut records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        if records.len() == CAPACITY {
            records.pop_front();
        }
        records.push_back(record);
    }

    /// The most recent records, newest first.
    #[must_use]
    pub fn recent(&self, limit: usize) -> Vec<RequestRecord> {
        let records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        records.iter().rev().take(limit).cloned().collect()
    }

    /// Per-bucket counts over the trailing `window_secs`, oldest bucket first.
    ///
    /// Buckets are aligned to `bucket_secs` boundaries so the axis stays put
    /// between polls, and empty buckets are kept so a quiet stretch reads as a
    /// gap rather than a compressed axis.
    #[must_use]
    pub fn buckets(&self, window_secs: u64, bucket_secs: u64, now: u64) -> Vec<Bucket> {
        let bucket_secs = bucket_secs.max(1);
        let count = (window_secs / bucket_secs).max(1);
        let offset = now % bucket_secs + bucket_secs.saturating_mul(count - 1);
        let start = now.saturating_sub(offset);

        let mut buckets: Vec<Bucket> = (0..count)
            .map(|i| Bucket {
                ts: start + i * bucket_secs,
                ok: 0,
                error: 0,
            })
            .collect();

        let records = self.records.lock().unwrap_or_else(PoisonError::into_inner);
        for record in records.iter().filter(|r| r.ts >= start) {
            let Ok(offset) = usize::try_from((record.ts - start) / bucket_secs) else {
                continue;
            };
            let Some(bucket) = buckets.get_mut(offset) else {
                continue;
            };
            if record.status < 400 {
                bucket.ok += 1;
            } else {
                bucket.error += 1;
            }
        }
        buckets
    }
}

/// Current wall-clock time in epoch seconds.
#[must_use]
pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
