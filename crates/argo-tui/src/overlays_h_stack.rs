//! H-stack overlays state — sessions / handoffs / budget panels
//! that read from the gateway's orchestrator + budget HTTP routes
//! (Stream `claude-tui-h-stack-panels`).
//!
//! Each panel keeps a `PanelState` enum that tracks the three
//! observable conditions the operator cares about:
//! - `Loading` — fetch in flight, panel renders a spinner row.
//! - `Ok(rows)` — fetch succeeded, panel renders the rows.
//! - `Unreachable(msg)` — gateway unreachable / 404 / timeout, panel
//!   renders the error inline so the operator can see why it's
//!   empty without crashing.
//!
//! The HTTP routes targeted here (`GET /orchestrator/sessions`,
//! `GET /orchestrator/handoffs`, `GET /budget/summary`) ship via
//! the H-orchestrator-routes brief. Until that lands, every fetch
//! returns `Unreachable("Gateway unreachable …")` cleanly.
use serde_json::Value;
use std::time::Duration;

/// Default gateway base URL when `$ARGO_GATEWAY_URL` is unset.
pub const DEFAULT_GATEWAY_URL: &str = "http://localhost:8787";

/// Per-panel observable state. Mirrors what each `draw_*` fn
/// expects at render time so the overlay always renders something
/// — never crashes on missing routes.
#[derive(Debug, Clone)]
pub enum PanelState {
    /// Fetch is in flight; render a spinner.
    Loading,
    /// Rows came back from the gateway. Stored as raw JSON so the
    /// overlay isn't coupled to a specific schema (the
    /// orchestrator-routes brief might refine the shape).
    Ok(Vec<Value>),
    /// Single-object response (used by the budget panel).
    OkOne(Value),
    /// Gateway unreachable / 404 / timeout / non-200. The string
    /// is rendered inline so the operator sees the cause.
    Unreachable(String),
}

impl PanelState {
    /// Number of rows this state would render. `Loading` counts
    /// as 1 (the spinner line). `Unreachable` counts as 1 (the
    /// error line). `OkOne` counts as 1.
    #[must_use]
    pub fn row_count(&self) -> usize {
        match self {
            PanelState::Loading | PanelState::Unreachable(_) | PanelState::OkOne(_) => 1,
            PanelState::Ok(rows) => rows.len(),
        }
    }
}

/// Resolve the gateway base URL — `$ARGO_GATEWAY_URL` first,
/// otherwise the loopback default.
#[must_use]
pub fn gateway_url() -> String {
    std::env::var("ARGO_GATEWAY_URL").unwrap_or_else(|_| DEFAULT_GATEWAY_URL.to_string())
}

/// Fetch JSON from `<base>/<path>` with a short timeout. Maps any
/// connection / status / parse error into `PanelState::Unreachable`
/// so the panel renders cleanly instead of crashing.
///
/// `expect_array = true` → response must be a top-level JSON array,
/// returns `PanelState::Ok(...)` on success. `false` → response
/// becomes `PanelState::OkOne(...)`.
pub async fn fetch_panel(base: &str, path: &str, expect_array: bool) -> PanelState {
    let url = format!("{}{path}", base.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => return PanelState::Unreachable(format!("client build: {e}")),
    };
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return PanelState::Unreachable(format!(
                "Gateway unreachable ({url}): {}",
                short_err(&e.to_string())
            ));
        }
    };
    let status = resp.status();
    if !status.is_success() {
        return PanelState::Unreachable(format!(
            "Gateway {} {} (waiting on H-orchestrator-routes)",
            status.as_u16(),
            status.canonical_reason().unwrap_or(""),
        ));
    }
    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => return PanelState::Unreachable(format!("body read: {e}")),
    };
    let v: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return PanelState::Unreachable(format!("invalid JSON: {e}")),
    };
    if expect_array {
        match v {
            Value::Array(arr) => PanelState::Ok(arr),
            other => PanelState::Unreachable(format!(
                "expected JSON array, got {}",
                short_kind(&other)
            )),
        }
    } else {
        PanelState::OkOne(v)
    }
}

fn short_err(msg: &str) -> String {
    // Trim long reqwest chains so the panel renders one line.
    msg.split("source: ")
        .next()
        .unwrap_or(msg)
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_string()
}

fn short_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_state_row_counts() {
        assert_eq!(PanelState::Loading.row_count(), 1);
        assert_eq!(PanelState::Unreachable("x".into()).row_count(), 1);
        assert_eq!(PanelState::OkOne(Value::Null).row_count(), 1);
        assert_eq!(
            PanelState::Ok(vec![Value::Null, Value::Null]).row_count(),
            2
        );
    }

    #[test]
    fn gateway_url_falls_back_to_loopback_when_unset() {
        // SAFETY: this test runs in serial with cargo test -p argo-tui
        // (no nextest); each test function gets fresh env state on
        // process start, but to avoid leaking we restore.
        let prev = std::env::var("ARGO_GATEWAY_URL").ok();
        std::env::remove_var("ARGO_GATEWAY_URL");
        assert_eq!(gateway_url(), DEFAULT_GATEWAY_URL);
        if let Some(p) = prev {
            std::env::set_var("ARGO_GATEWAY_URL", p);
        }
    }

    #[test]
    fn short_err_trims_long_chains() {
        // `short_err` splits on " source: " and strips a single
        // trailing ':' so "foo:" becomes "foo".
        assert_eq!(short_err("foo: source: bar"), "foo");
        assert_eq!(short_err("foo source: bar"), "foo");
        assert_eq!(short_err("standalone"), "standalone");
    }

    #[tokio::test]
    async fn fetch_panel_returns_unreachable_on_unbound_port() {
        // Port 1 — unprivileged + unbound on every test runner.
        let state = fetch_panel("http://127.0.0.1:1", "/x", true).await;
        match state {
            PanelState::Unreachable(msg) => assert!(
                msg.contains("Gateway unreachable") || msg.contains("error"),
                "got: {msg}"
            ),
            other => panic!("expected Unreachable, got {other:?}"),
        }
    }
}
