//! HTTP log shipping target (Mission `mission-log-shipping-target-docs`).
//!
//! Optional [`tracing_subscriber::Layer`] that POSTs JSON-serialized
//! events to a remote endpoint in fixed-size batches. Best-effort:
//! HTTP errors warn-not-crash, queue overflow drops the OLDEST event
//! (new events matter more for "what's happening now"), and [`Drop`]
//! flushes pending events with a short timeout. The Layer never
//! blocks the calling `tracing` emit — a parking-lot mutex guards a
//! bounded [`VecDeque`] and the POST runs in a `tokio::spawn` flusher.
//!
//! Designed for operators who cannot run a sidecar log-agent
//! (heroku-style hosting, FaaS); sidecar deployments should use the
//! recipes in `docs/skycot/runbook/log-shipping.md` instead.
//!
//! Self-emitted warnings carry [`INTERNAL_TARGET`] so the Layer's own
//! `on_event` skips them and cannot recurse on a permanent failure.
use parking_lot::Mutex;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, InvalidHeaderValue};
use reqwest::Url;
use serde_json::{Map, Value};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::field::{Field, Visit};
use tracing::Event;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Target used by Layer-internal warnings so its own `on_event`
/// skips them and avoids an export-failure recursion.
pub const INTERNAL_TARGET: &str = "argo_telemetry::http_target";

const DROP_FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

/// Bounded JSON-event queue + HTTP shipping worker.
pub struct HttpLogTarget {
    endpoint: Url,
    headers: HeaderMap,
    batch_size: usize,
    flush_interval: Duration,
    queue_max: usize,
    queue: Mutex<VecDeque<String>>,
    notify: Notify,
    shutdown: AtomicBool,
    client: reqwest::Client,
}

impl HttpLogTarget {
    /// Build a target. `auth` is sent as the `Authorization` header
    /// value verbatim (`Bearer …`, `Splunk …`, etc.).
    pub fn new(
        endpoint: Url,
        auth: Option<String>,
        batch_size: usize,
        flush_interval: Duration,
        timeout: Duration,
        queue_max: usize,
    ) -> Result<Self, InvalidHeaderValue> {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        if let Some(a) = auth {
            headers.insert(HeaderName::from_static("authorization"), HeaderValue::from_str(&a)?);
        }
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Ok(Self {
            endpoint,
            headers,
            batch_size: batch_size.max(1),
            flush_interval,
            queue_max: queue_max.max(1),
            queue: Mutex::new(VecDeque::new()),
            notify: Notify::new(),
            shutdown: AtomicBool::new(false),
            client,
        })
    }

    /// Enqueue one event; drop OLDEST on overflow; notify the flusher
    /// when post-push depth has reached `batch_size`.
    pub fn push(&self, json: String) {
        let len = {
            let mut q = self.queue.lock();
            if q.len() >= self.queue_max {
                q.pop_front();
            }
            q.push_back(json);
            q.len()
        };
        if len >= self.batch_size {
            self.notify.notify_one();
        }
    }

    #[cfg(test)]
    pub(crate) fn queue_depth(&self) -> usize {
        self.queue.lock().len()
    }

    /// Drain up to `batch_size` and POST as a JSON array. HTTP errors
    /// warn at [`INTERNAL_TARGET`]; never panic.
    pub async fn flush_once(&self) -> usize {
        let batch: Vec<String> = {
            let mut q = self.queue.lock();
            let take = q.len().min(self.batch_size);
            q.drain(..take).collect()
        };
        if batch.is_empty() {
            return 0;
        }
        let sent = batch.len();
        let body = format!("[{}]", batch.join(","));
        match self.client.post(self.endpoint.clone()).headers(self.headers.clone()).body(body).send().await {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => tracing::warn!(target: INTERNAL_TARGET, status = r.status().as_u16(), "log-shipping non-2xx; events discarded"),
            Err(e) => tracing::warn!(target: INTERNAL_TARGET, error = %e, "log-shipping POST failed; events discarded"),
        }
        sent
    }

    /// Background loop; wakes on `flush_interval` OR batch-cap notify.
    pub async fn run_flusher(self: Arc<Self>) {
        while !self.shutdown.load(Ordering::Acquire) {
            tokio::select! {
                _ = tokio::time::sleep(self.flush_interval) => {}
                () = self.notify.notified() => {}
            }
            while !self.queue.lock().is_empty() {
                if self.flush_once().await == 0 {
                    break;
                }
            }
        }
    }

    /// Best-effort drain bounded by [`DROP_FLUSH_TIMEOUT`].
    pub async fn drain_with_timeout(&self) {
        let drain = async {
            while !self.queue.lock().is_empty() {
                if self.flush_once().await == 0 {
                    break;
                }
            }
        };
        let _ = tokio::time::timeout(DROP_FLUSH_TIMEOUT, drain).await;
    }
}

impl Drop for HttpLogTarget {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        self.notify.notify_waiters();
        let queue: VecDeque<String> = std::mem::take(&mut *self.queue.lock());
        if queue.is_empty() {
            return;
        }
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let (endpoint, headers, client) = (self.endpoint.clone(), self.headers.clone(), self.client.clone());
            handle.spawn(async move {
                let body = format!("[{}]", queue.into_iter().collect::<Vec<_>>().join(","));
                let _ = tokio::time::timeout(
                    DROP_FLUSH_TIMEOUT,
                    client.post(endpoint).headers(headers).body(body).send(),
                ).await;
            });
        }
    }
}

/// `tracing-subscriber::Layer` that serializes each event and
/// enqueues it on the wrapped [`HttpLogTarget`].
pub struct HttpLogTargetLayer {
    target: Arc<HttpLogTarget>,
}

impl HttpLogTargetLayer {
    /// Wrap a target. If `spawn_flusher` is `true`, also start the
    /// background flusher task (requires a running tokio runtime).
    /// Tests pass `false` and drive flushes manually for determinism.
    #[must_use]
    pub fn new(target: Arc<HttpLogTarget>, spawn_flusher: bool) -> Self {
        if spawn_flusher {
            let f = target.clone();
            tokio::spawn(async move { f.run_flusher().await });
        }
        Self { target }
    }
}

impl<S> Layer<S> for HttpLogTargetLayer
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        if meta.target() == INTERNAL_TARGET {
            return;
        }
        let mut fields = Map::new();
        fields.insert("level".into(), Value::String(meta.level().to_string()));
        fields.insert("target".into(), Value::String(meta.target().to_string()));
        event.record(&mut JsonFieldVisitor { out: &mut fields });
        if let Ok(s) = serde_json::to_string(&Value::Object(fields)) {
            self.target.push(s);
        }
    }
}

struct JsonFieldVisitor<'a> {
    out: &'a mut Map<String, Value>,
}

impl Visit for JsonFieldVisitor<'_> {
    fn record_str(&mut self, f: &Field, v: &str) { self.out.insert(f.name().into(), Value::String(v.into())); }
    fn record_bool(&mut self, f: &Field, v: bool) { self.out.insert(f.name().into(), Value::Bool(v)); }
    fn record_i64(&mut self, f: &Field, v: i64) { self.out.insert(f.name().into(), Value::Number(v.into())); }
    fn record_u64(&mut self, f: &Field, v: u64) { self.out.insert(f.name().into(), Value::Number(v.into())); }
    fn record_f64(&mut self, f: &Field, v: f64) {
        let n = serde_json::Number::from_f64(v).unwrap_or_else(|| 0.into());
        self.out.insert(f.name().into(), Value::Number(n));
    }
    fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
        self.out.insert(f.name().into(), Value::String(format!("{v:?}")));
    }
}

#[cfg(test)]
#[path = "http_target_tests.rs"]
mod tests;
