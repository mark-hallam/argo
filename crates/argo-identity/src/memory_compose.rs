//! Pure formatting helpers for `MemoryNs::compose_context`.
//!
//! Loaded via `#[path = "memory_compose.rs"] mod memory_compose;` from
//! `types.rs` — kept in a sibling file because `types.rs` is at the
//! 500-LOC ceiling. No DB access here; the orchestrator
//! (`argo-agent-runner::turn::build_system_prompt`) sources the rows
//! from the appropriate `MemoryStore` scope before calling.

use chrono::{DateTime, Utc};

pub(crate) const DEFAULT_TOP_N: usize = 8;
pub(crate) const TOP_N_ENV: &str = "ARGO_MEMORY_COMPOSE_TOP_N";

/// Render `(timestamp, content)` rows as a `## Memory` markdown block.
///
/// Returns the `top_n` most recent rows (newest first) formatted as
/// a bulleted list. Empty input or `top_n == 0` → empty string.
///
/// `top_n` is parameterized to keep this helper pure (parallel-test
/// safe). The public method `MemoryNs::compose_context` resolves the
/// env-var default and forwards here.
pub(crate) fn compose(rows: &[(DateTime<Utc>, String)], top_n: usize) -> String {
    if rows.is_empty() || top_n == 0 {
        return String::new();
    }
    let mut take: Vec<&(DateTime<Utc>, String)> = rows.iter().collect();
    take.sort_by_key(|r| std::cmp::Reverse(r.0));
    take.truncate(top_n);
    let count = take.len();
    let mut out = format!("## Memory (workspace-shared, last {count})\n");
    for (ts, content) in take {
        out.push_str(&format!("- {}: {}\n", ts.format("%Y-%m-%d"), content));
    }
    out
}

/// Resolve top-N from the `ARGO_MEMORY_COMPOSE_TOP_N` env var,
/// falling back to `DEFAULT_TOP_N` when missing or unparseable.
pub(crate) fn resolve_top_n() -> usize {
    std::env::var(TOP_N_ENV)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_TOP_N)
}

#[cfg(test)]
#[path = "memory_compose_tests.rs"]
mod tests;
