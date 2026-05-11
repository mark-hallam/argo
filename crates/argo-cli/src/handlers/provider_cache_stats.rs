//! `argo provider cache-stats` — print prompt-cache hit-rate metrics
//! aggregated per `(provider, model)` from the in-process counters
//! populated by `argo-provider`'s response handlers
//! (mission-prompt-cache-hit-rate-observability).
//!
//! Reads `argo_telemetry::metrics_prompt_cache::snapshot()`. Counters
//! are process-global and cumulative for the lifetime of the running
//! process, so the most useful invocation is against a long-running
//! gateway (`argo gateway start`) rather than a fresh CLI shell.
use argo_telemetry::metrics_prompt_cache::{snapshot, PromptCacheStats};
use argo_types::ArgoResult;

/// Render cache-stats. `provider` / `model` are optional case-sensitive
/// equality filters; `json` switches to a JSON array body.
pub fn run(provider: Option<&str>, model: Option<&str>, json: bool) -> ArgoResult<()> {
    let rows: Vec<PromptCacheStats> = snapshot()
        .into_iter()
        .filter(|r| provider.is_none_or(|p| r.provider == p))
        .filter(|r| model.is_none_or(|m| r.model == m))
        .collect();
    if json {
        print_json(&rows);
    } else {
        print_text(&rows);
    }
    Ok(())
}

fn print_json(rows: &[PromptCacheStats]) {
    let arr: Vec<serde_json::Value> = rows.iter().map(row_to_json).collect();
    let body = serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string());
    println!("{body}");
}

fn row_to_json(r: &PromptCacheStats) -> serde_json::Value {
    serde_json::json!({
        "provider": r.provider,
        "model": r.model,
        "cache_creation_input_tokens": r.cache_creation_input_tokens,
        "cache_read_input_tokens": r.cache_read_input_tokens,
        "uncached_input_tokens": r.uncached_input_tokens,
        "hit_rate": r.hit_rate,
    })
}

fn print_text(rows: &[PromptCacheStats]) {
    if rows.is_empty() {
        println!(
            "(no cache-stats recorded — counters increment on each Anthropic response in this process)"
        );
        return;
    }
    println!(
        "{:<24} {:<32} {:>14} {:>14} {:>14} {:>10}",
        "PROVIDER", "MODEL", "CACHE_CREATE", "CACHE_READ", "UNCACHED", "HIT_RATE"
    );
    for r in rows {
        println!(
            "{:<24} {:<32} {:>14} {:>14} {:>14} {:>9.2}%",
            truncate(&r.provider, 24),
            truncate(&r.model, 32),
            r.cache_creation_input_tokens,
            r.cache_read_input_tokens,
            r.uncached_input_tokens,
            r.hit_rate * 100.0,
        );
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_telemetry::metrics_prompt_cache::record;

    #[test]
    fn run_no_filter_emits_no_error() {
        record("test-handler", "model-h", 100, 30, 20);
        // The handler writes to stdout; we only assert it returns Ok.
        assert!(run(None, None, false).is_ok());
        assert!(run(None, None, true).is_ok());
    }

    #[test]
    fn provider_filter_narrows_rows() {
        record("test-handler-pf", "model-i", 100, 30, 20);
        record("other-handler-pf", "model-i", 200, 0, 0);
        let s = snapshot();
        let provider = "test-handler-pf";
        let filtered: Vec<_> = s
            .into_iter()
            .filter(|r| r.provider == provider)
            .collect();
        assert!(!filtered.is_empty());
        assert!(filtered.iter().all(|r| r.provider == provider));
    }
}
