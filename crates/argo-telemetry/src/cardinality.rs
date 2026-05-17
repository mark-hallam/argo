//! Cardinality cap for Prometheus metric families.
//!
//! Operators billed per-series (Grafana Cloud, Datadog) face a real
//! financial risk: a buggy code path emitting `tenant_id` as a label
//! can mint thousands of label sets per minute and surprise the
//! operator with a four- or five-figure scrape bill.
//! [`CardinalityTracker`] caps the distinct label-set count per
//! family. Above cap, new label sets are dropped (existing series keep
//! recording) and `argo_metric_cardinality_capped_total{metric="…"}`
//! increments. Logs at 80% (`WARN`) and at hard cap (`ERROR`) are
//! rate-limited to one event per metric per 5-minute window.
//!
//! ```
//! use argo_telemetry::cardinality::{CardinalityTracker, Caps, CardinalityDecision};
//! let tracker = CardinalityTracker::new(Caps::defaults());
//! assert!(matches!(tracker.observe("argo_demo", &["a"]), CardinalityDecision::Allow));
//! ```

use parking_lot::Mutex;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Default per-family cap for histogram and summary metrics.
pub const DEFAULT_HISTOGRAM_CAP: u32 = 1000;
/// Default per-family cap for counter and gauge metrics.
pub const DEFAULT_COUNTER_GAUGE_CAP: u32 = 5000;
const LOG_WINDOW: Duration = Duration::from_secs(300);
const HLL_PRECISION: u32 = 10;

/// Allow / drop decision returned by [`CardinalityTracker::observe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardinalityDecision {
    /// Label set is within the per-family cap; record it.
    Allow,
    /// Label set is over the hard cap; drop. Never blocks, never
    /// panics.
    Drop,
}

/// Metric-family kind shorthand. Drives which default cap applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyKind {
    /// Histogram or summary family — tighter default (1000).
    Histogram,
    /// Counter or gauge family — looser default (5000).
    CounterGauge,
}

/// Per-family caps; defaults from the brief, operator overrides via
/// `[telemetry.prometheus.cardinality_cap]`.
#[derive(Debug, Clone)]
pub struct Caps {
    histogram_default: u32,
    counter_default: u32,
    per_metric: HashMap<String, u32>,
}

impl Caps {
    /// Defaults: 1000 for histograms, 5000 for counter/gauge.
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            histogram_default: DEFAULT_HISTOGRAM_CAP,
            counter_default: DEFAULT_COUNTER_GAUGE_CAP,
            per_metric: HashMap::new(),
        }
    }
    /// Per-family override.
    #[must_use]
    pub fn with_override(mut self, metric: impl Into<String>, cap: u32) -> Self {
        self.per_metric.insert(metric.into(), cap);
        self
    }
    /// Override the histogram-family default.
    #[must_use]
    pub fn with_histogram_default(mut self, cap: u32) -> Self {
        self.histogram_default = cap;
        self
    }
    /// Override the counter/gauge-family default.
    #[must_use]
    pub fn with_counter_default(mut self, cap: u32) -> Self {
        self.counter_default = cap;
        self
    }
    fn cap_for(&self, metric: &str, kind: FamilyKind) -> u32 {
        if let Some(c) = self.per_metric.get(metric) {
            return *c;
        }
        match kind {
            FamilyKind::Histogram => self.histogram_default,
            FamilyKind::CounterGauge => self.counter_default,
        }
    }
}

/// Per-family triage row used by `/metrics/cardinality`.
#[derive(Debug, Clone)]
pub struct FamilyReport {
    /// Metric family name.
    pub metric: String,
    /// HLL-estimated distinct label-set count.
    pub current_count: u32,
    /// Effective per-family cap.
    pub cap: u32,
    /// Times a new label set was dropped at the cap.
    pub capped_total: u64,
}

#[derive(Debug)]
struct FamilyState {
    hll: Hll,
    capped_total: u64,
    last_warn_at: Option<Instant>,
    last_error_at: Option<Instant>,
    kind: FamilyKind,
}

/// Thread-safe cardinality tracker. Mutex held for O(1) HLL update.
pub struct CardinalityTracker {
    caps: Mutex<Caps>,
    families: Mutex<HashMap<String, FamilyState>>,
}

impl CardinalityTracker {
    /// Construct with the given caps.
    #[must_use]
    pub fn new(caps: Caps) -> Self {
        Self {
            caps: Mutex::new(caps),
            families: Mutex::new(HashMap::new()),
        }
    }
    /// Swap caps at runtime (e.g. config reload).
    pub fn set_caps(&self, caps: Caps) {
        *self.caps.lock() = caps;
    }
    /// Observe a label set against the counter/gauge default.
    pub fn observe(&self, metric: &str, labels: &[&str]) -> CardinalityDecision {
        self.observe_kind(metric, labels, FamilyKind::CounterGauge)
    }
    /// Observe a label set with an explicit kind hint.
    pub fn observe_kind(&self, metric: &str, labels: &[&str], kind: FamilyKind) -> CardinalityDecision {
        let cap = self.caps.lock().cap_for(metric, kind);
        let mut families = self.families.lock();
        let state = families
            .entry(metric.to_string())
            .or_insert_with(|| FamilyState {
                hll: Hll::new(),
                capped_total: 0,
                last_warn_at: None,
                last_error_at: None,
                kind,
            });
        state.hll.add(labels);
        let count = state.hll.count();
        if count > cap {
            state.capped_total = state.capped_total.saturating_add(1);
            if should_log(state.last_error_at) {
                state.last_error_at = Some(Instant::now());
                tracing::error!(
                    target: "argo_telemetry::cardinality",
                    metric = %metric, count, cap,
                    "metric `{metric}` at cardinality hard cap ({count}/{cap}); dropping new label set"
                );
            }
            return CardinalityDecision::Drop;
        }
        let warn_floor = cap.saturating_mul(4) / 5;
        if count >= warn_floor && should_log(state.last_warn_at) {
            state.last_warn_at = Some(Instant::now());
            tracing::warn!(
                target: "argo_telemetry::cardinality",
                metric = %metric, count, cap,
                "metric `{metric}` approaching cardinality cap ({count}/{cap})"
            );
        }
        CardinalityDecision::Allow
    }
    /// Snapshot per-family state for `/metrics/cardinality`.
    #[must_use]
    pub fn snapshot(&self) -> Vec<FamilyReport> {
        let caps = self.caps.lock();
        let families = self.families.lock();
        let mut out: Vec<FamilyReport> = families
            .iter()
            .map(|(n, s)| FamilyReport {
                metric: n.clone(),
                current_count: s.hll.count(),
                cap: caps.cap_for(n, s.kind),
                capped_total: s.capped_total,
            })
            .collect();
        out.sort_by(|a, b| a.metric.cmp(&b.metric));
        out
    }
}

fn should_log(last: Option<Instant>) -> bool {
    match last {
        None => true,
        Some(t) => t.elapsed() >= LOG_WINDOW,
    }
}

static GLOBAL: OnceLock<CardinalityTracker> = OnceLock::new();

/// Process-global tracker. First call inits with [`Caps::defaults`];
/// operators replace via [`CardinalityTracker::set_caps`] post-boot.
#[must_use]
pub fn global_tracker() -> &'static CardinalityTracker {
    GLOBAL.get_or_init(|| CardinalityTracker::new(Caps::defaults()))
}

/// Inline HLL (p=10 → 1024 registers, ~3% std error). 1024 bytes per
/// family — bounded regardless of cap size.
#[derive(Debug)]
struct Hll {
    registers: Box<[u8; 1 << HLL_PRECISION]>,
}

impl Hll {
    fn new() -> Self {
        Self {
            registers: Box::new([0_u8; 1 << HLL_PRECISION]),
        }
    }
    fn add(&mut self, labels: &[&str]) {
        let mut hasher = DefaultHasher::new();
        for part in labels {
            part.hash(&mut hasher);
            0_u8.hash(&mut hasher); // separator: ["a","bc"] ≠ ["ab","c"]
        }
        let h = hasher.finish();
        let idx = (h >> (64 - HLL_PRECISION)) as usize;
        let w = h << HLL_PRECISION;
        let rho = ((w.leading_zeros() + 1) as u8).min((64 - HLL_PRECISION) as u8 + 1);
        if rho > self.registers[idx] {
            self.registers[idx] = rho;
        }
    }
    fn count(&self) -> u32 {
        let m = self.registers.len() as f64;
        let alpha = 0.7213 / (1.0 + 1.079 / m);
        let sum: f64 = self
            .registers
            .iter()
            .map(|&r| 2_f64.powi(-(i32::from(r))))
            .sum();
        let raw = alpha * m * m / sum;
        let zeros = self.registers.iter().filter(|&&r| r == 0).count() as f64;
        let est = if raw < 2.5 * m && zeros > 0.0 {
            m * (m / zeros).ln()
        } else {
            raw
        };
        if est.is_finite() && est >= 0.0 {
            est.min(f64::from(u32::MAX)) as u32
        } else {
            0
        }
    }
}

#[cfg(test)]
#[path = "cardinality_tests.rs"]
mod tests;
