//! Process-global prompt cache hit-rate metrics (mission-prompt-cache-hit-rate-observability).
//!
//! Anthropic Messages responses report cache usage in
//! `usage.cache_creation_input_tokens` (tokens that primed the prompt
//! cache) and `usage.cache_read_input_tokens` (tokens served from the
//! cache). Operators tuning prompt structure need a fleet-wide view of
//! cache hit rate without re-parsing raw logs.
//!
//! This module exposes a small process-global aggregator: callers in
//! `argo-provider` invoke [`record`] on every response, and the CLI
//! (`argo provider cache-stats`) reads [`snapshot`] to surface the
//! counters + derived hit rate per `(provider, model)` pair.
//!
//! Pattern mirrors `crates/argo-cli/src/handlers/gateway/metrics.rs`
//! which already uses static `OnceLock` + `AtomicU64` for the
//! gateway's Prometheus exposition counters. Single-binary metrics
//! aggregation is a standing exception to the "no global mutable
//! state" invariant (CLAUDE.md §invariants), since tracing /
//! Prometheus universally require process-global state.
//!
//! ```
//! use argo_telemetry::metrics_prompt_cache::{record, snapshot};
//! record("doctest-provider", "doctest-model", 100, 30, 20);
//! let row = snapshot()
//!     .into_iter()
//!     .find(|r| r.provider == "doctest-provider")
//!     .expect("recorded row");
//! assert_eq!(row.cache_read_input_tokens, 20);
//! ```
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

/// One row of cache metrics for a `(provider, model)` combination.
#[derive(Debug, Clone, PartialEq)]
pub struct PromptCacheStats {
    /// Provider id from the manifest (e.g. `anthropic`).
    pub provider: String,
    /// Model id resolved by the provider (e.g. `claude-opus-4-7`).
    pub model: String,
    /// Cumulative tokens used to create cache entries.
    pub cache_creation_input_tokens: u64,
    /// Cumulative tokens served from the cache.
    pub cache_read_input_tokens: u64,
    /// Cumulative regular (uncached) input tokens.
    pub uncached_input_tokens: u64,
    /// `cache_read / (cache_read + cache_creation + uncached)`. Zero
    /// when no tokens have been recorded (avoids `NaN`).
    pub hit_rate: f64,
}

#[derive(Default)]
struct Counters {
    cache_creation: AtomicU64,
    cache_read: AtomicU64,
    uncached: AtomicU64,
}

type Registry = RwLock<BTreeMap<(String, String), Counters>>;

fn registry() -> &'static Registry {
    static R: OnceLock<Registry> = OnceLock::new();
    R.get_or_init(|| RwLock::new(BTreeMap::new()))
}

/// Record one response's cache token usage.
///
/// `input_tokens` is the regular (uncached) input-token count as
/// reported by the provider. For Anthropic's Messages API, this is the
/// `usage.input_tokens` field; cache tokens are accounted separately
/// in `usage.cache_creation_input_tokens` and
/// `usage.cache_read_input_tokens` per
/// <https://docs.anthropic.com/en/api/messages>. All inputs are
/// added with `fetch_add` (`Ordering::Relaxed`) to a per-key counter.
///
/// Cardinality: keyed by `(provider, model)` — typically 10-20
/// distinct combinations in production, well within Prometheus
/// safe-cardinality limits.
pub fn record(
    provider: &str,
    model: &str,
    input_tokens: u32,
    cache_creation_input_tokens: u32,
    cache_read_input_tokens: u32,
) {
    let key = (provider.to_string(), model.to_string());
    let r = registry();
    {
        let guard = r.read();
        if let Some(entry) = guard.get(&key) {
            apply(
                entry,
                input_tokens,
                cache_creation_input_tokens,
                cache_read_input_tokens,
            );
            return;
        }
    }
    let mut guard = r.write();
    let entry = guard.entry(key).or_default();
    apply(
        entry,
        input_tokens,
        cache_creation_input_tokens,
        cache_read_input_tokens,
    );
}

fn apply(c: &Counters, uncached: u32, cache_creation: u32, cache_read: u32) {
    c.uncached.fetch_add(u64::from(uncached), Ordering::Relaxed);
    c.cache_creation
        .fetch_add(u64::from(cache_creation), Ordering::Relaxed);
    c.cache_read
        .fetch_add(u64::from(cache_read), Ordering::Relaxed);
}

/// Snapshot every recorded `(provider, model)` row, sorted by key.
///
/// Each row carries the three counters plus the derived hit rate. Hit
/// rate is `0.0` when the row's total is zero (defense-in-depth — the
/// `record` path never inserts an all-zero row, but a future caller
/// might).
#[must_use]
pub fn snapshot() -> Vec<PromptCacheStats> {
    let r = registry();
    let guard = r.read();
    guard
        .iter()
        .map(|((p, m), c)| {
            let cc = c.cache_creation.load(Ordering::Relaxed);
            let cr = c.cache_read.load(Ordering::Relaxed);
            let un = c.uncached.load(Ordering::Relaxed);
            let total = cc.saturating_add(cr).saturating_add(un);
            let hit_rate = if total == 0 {
                0.0
            } else {
                // Cast is safe: counters are u64, ratio is bounded [0,1].
                #[allow(clippy::cast_precision_loss)]
                let r = (cr as f64) / (total as f64);
                r
            };
            PromptCacheStats {
                provider: p.clone(),
                model: m.clone(),
                cache_creation_input_tokens: cc,
                cache_read_input_tokens: cr,
                uncached_input_tokens: un,
                hit_rate,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pick(rows: &[PromptCacheStats], provider: &str, model: &str) -> Option<PromptCacheStats> {
        rows.iter()
            .find(|r| r.provider == provider && r.model == model)
            .cloned()
    }

    // Each test uses a unique `(provider, model)` key so parallel test
    // execution doesn't race on the process-global registry — there is
    // no `reset_for_tests` because counters are monotonic by contract.

    #[test]
    fn counter_increments_correctly() {
        record("test-incr", "model-a", 100, 30, 20);
        record("test-incr", "model-a", 50, 0, 10);
        let rows = snapshot();
        let row = pick(&rows, "test-incr", "model-a").expect("row present");
        assert_eq!(row.uncached_input_tokens, 150);
        assert_eq!(row.cache_creation_input_tokens, 30);
        assert_eq!(row.cache_read_input_tokens, 30);
    }

    #[test]
    fn uncached_computed_from_total() {
        // Anthropic semantics: `usage.input_tokens` is already the
        // uncached portion; `cache_creation_*` and `cache_read_*` are
        // separate fields. The "uncached" counter is therefore the
        // running sum of the `input_tokens` value passed in.
        record("test-uncached", "model-b", 80, 20, 50);
        let rows = snapshot();
        let row = pick(&rows, "test-uncached", "model-b").expect("row");
        assert_eq!(row.uncached_input_tokens, 80);
        assert_eq!(row.cache_creation_input_tokens, 20);
        assert_eq!(row.cache_read_input_tokens, 50);
        let total = 80 + 20 + 50;
        #[allow(clippy::cast_precision_loss)]
        let expected = 50.0 / total as f64;
        assert!((row.hit_rate - expected).abs() < 1e-9);
    }

    #[test]
    fn hit_rate_zero_when_no_cache() {
        record("test-no-cache", "model-c", 200, 0, 0);
        let rows = snapshot();
        let row = pick(&rows, "test-no-cache", "model-c").expect("row");
        assert_eq!(row.hit_rate, 0.0);
    }

    #[test]
    fn hit_rate_handles_divide_by_zero() {
        // Edge case: a zero-token record creates a key but no math
        // should produce NaN/Infinity.
        record("test-zero", "model-d", 0, 0, 0);
        let rows = snapshot();
        let row = pick(&rows, "test-zero", "model-d").expect("row");
        assert_eq!(row.hit_rate, 0.0);
        assert!(row.hit_rate.is_finite());
    }

    #[test]
    fn keys_are_per_provider_and_model() {
        record("test-multi", "model-x", 10, 1, 1);
        record("test-multi", "model-y", 20, 2, 2);
        record("test-multi-other", "model-x", 30, 3, 3);
        let rows = snapshot();
        let a_x = pick(&rows, "test-multi", "model-x").expect("a_x");
        let a_y = pick(&rows, "test-multi", "model-y").expect("a_y");
        let o_x = pick(&rows, "test-multi-other", "model-x").expect("o_x");
        assert_eq!(a_x.uncached_input_tokens, 10);
        assert_eq!(a_y.uncached_input_tokens, 20);
        assert_eq!(o_x.uncached_input_tokens, 30);
    }

    #[test]
    fn integration_smoke_three_responses_aggregate_correctly() {
        // Simulate three Anthropic responses landing in sequence: a
        // cold-cache miss (creation only), a warm-cache hit (cache_read
        // dominant), and an uncached top-up. Then assert the snapshot
        // surfaces the right cumulative counts and a hit rate that
        // matches manual arithmetic.
        let provider = "test-smoke";
        let model = "claude-haiku-4-5";
        // Response 1: cold cache create.
        record(provider, model, 10, 200, 0);
        // Response 2: warm cache hit, small new prefix.
        record(provider, model, 5, 0, 200);
        // Response 3: uncached short follow-up.
        record(provider, model, 50, 0, 0);
        let row = pick(&snapshot(), provider, model).expect("smoke row");
        assert_eq!(row.cache_creation_input_tokens, 200);
        assert_eq!(row.cache_read_input_tokens, 200);
        assert_eq!(row.uncached_input_tokens, 65);
        let total = 200 + 200 + 65;
        #[allow(clippy::cast_precision_loss)]
        let expected = 200.0 / total as f64;
        assert!((row.hit_rate - expected).abs() < 1e-9);
    }
}
