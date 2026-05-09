//! `argo latency` — read the per-executor-kind action latency
//! histogram and print p50/p95/p99/max per kind (Stream
//! `claude-latency-percentiles-tracking`).
//!
//! The dispatch wrapper installed by
//! `worker_actions_executor::resolve_executor` records every
//! action dispatch into
//! `argo_store::latency_tracker::global()`. This handler:
//!
//! 1. Drains any in-process samples (so a long-running shell that
//!    invoked `argo latency` directly after several actions sees
//!    them in the readout) and persists them into the
//!    `latency_buckets` table.
//! 2. Loads the cumulative bucket totals from the store.
//! 3. Computes per-kind percentiles by interpolation-within-bucket
//!    and prints either a markdown table (default) or a JSON
//!    object (`--json` / `--format json`).
//!
//! ### Output shape
//!
//! Markdown columns: `kind | n | p50 | p95 | p99 | max`. `n` is
//! derived from the bucket sum so the schema stays minimal. JSON
//! emits `{ "buckets": N, "rows": [{ "kind", "n", "p50_ms", ... }] }`.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::latency_tracker::{
    flush_to_store, global, load_persisted, max_observed, percentile, total_samples, N_BUCKETS,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

/// `argo latency` entrypoint. `format` is the literal CLI flag
/// value (`"markdown"` or `"json"`); empty / unrecognised values
/// fall back to markdown.
pub async fn run(format: &str) -> ArgoResult<()> {
    let store = open_store().await?;
    // Drain any in-process samples so callers that invoke
    // `argo latency` from the same long-running shell that just
    // dispatched actions see the freshest possible numbers. This
    // is also the only flush path until the gateway's 60s flush
    // task is wired (deferred — gateway boot is outside the
    // brief's may-touch list).
    let pending = global().drain();
    if !pending.is_empty() {
        flush_to_store(store.pool(), &pending).await?;
    }
    let buckets = load_persisted(store.pool()).await?;
    let rows = compute_rows(&buckets);
    let fmt = OutputFormat::parse(format);
    match fmt {
        OutputFormat::Json => print_json(&rows),
        OutputFormat::Markdown => print_markdown(&rows),
    }
    Ok(())
}

/// Output mode for the readout.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Markdown,
    Json,
}

impl OutputFormat {
    pub(crate) fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "json" => Self::Json,
            _ => Self::Markdown,
        }
    }
}

/// One row in the per-kind percentile readout.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct LatencyRow {
    pub(crate) kind: String,
    pub(crate) n: u64,
    pub(crate) p50_ms: Option<u64>,
    pub(crate) p95_ms: Option<u64>,
    pub(crate) p99_ms: Option<u64>,
    pub(crate) max_ms: Option<u64>,
}

/// Compute per-kind percentile rows from a kind→buckets map.
/// Sorted alphabetically by kind so the readout is deterministic
/// across runs.
pub(crate) fn compute_rows(
    snapshot: &HashMap<String, [u64; N_BUCKETS]>,
) -> Vec<LatencyRow> {
    let mut rows: Vec<LatencyRow> = snapshot
        .iter()
        .map(|(kind, buckets)| LatencyRow {
            kind: kind.clone(),
            n: total_samples(buckets),
            p50_ms: percentile(buckets, 0.50).map(dur_to_ms),
            p95_ms: percentile(buckets, 0.95).map(dur_to_ms),
            p99_ms: percentile(buckets, 0.99).map(dur_to_ms),
            max_ms: max_observed(buckets).map(dur_to_ms),
        })
        .collect();
    rows.sort_by(|a, b| a.kind.cmp(&b.kind));
    rows
}

fn dur_to_ms(d: Duration) -> u64 {
    let micros = d.as_micros();
    // Round to nearest ms for display.
    ((micros + 500) / 1000) as u64
}

/// Render the markdown table. Empty histograms get a friendly
/// placeholder line so the output never looks like a malformed
/// table.
pub(crate) fn render_markdown(rows: &[LatencyRow]) -> String {
    if rows.is_empty() {
        return "| kind | n | p50 | p95 | p99 | max |\n\
                |------|---|-----|-----|-----|-----|\n\
                | _(no samples recorded yet — dispatch some actions and re-run)_ ||||||\n"
            .to_string();
    }
    let mut out = String::new();
    out.push_str("| kind | n | p50 | p95 | p99 | max |\n");
    out.push_str("|------|---|-----|-----|-----|-----|\n");
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.kind,
            r.n,
            fmt_ms_cell(r.p50_ms),
            fmt_ms_cell(r.p95_ms),
            fmt_ms_cell(r.p99_ms),
            fmt_ms_cell(r.max_ms),
        ));
    }
    out
}

fn fmt_ms_cell(v: Option<u64>) -> String {
    match v {
        Some(ms) => format!("{ms}ms"),
        None => "—".to_string(),
    }
}

fn print_markdown(rows: &[LatencyRow]) {
    print!("{}", render_markdown(rows));
}

fn print_json(rows: &[LatencyRow]) {
    let payload = json!({
        "buckets": N_BUCKETS,
        "rows": rows,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
    );
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[cfg(test)]
#[path = "latency_tests.rs"]
mod tests;
