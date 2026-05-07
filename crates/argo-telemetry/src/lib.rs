//! `argo-telemetry` — span + metric taxonomy (§18).
//!
//! Phase-7 ship: a minimal trait-based seam plus the canonical span
//! and metric names from §18.1 / §18.2 as constants. Concrete
//! exporters (OTLP, Prometheus) live in their own follow-up.
//!
//! ```
//! use argo_telemetry::{metrics, spans, NoopTelemetry, TelemetrySink};
//! let t: &dyn TelemetrySink = &NoopTelemetry;
//! t.counter(metrics::ROUTING_DECISIONS_TOTAL, 1, &[("tier", "simple")]);
//! let _span = t.span(spans::AGENT_TURN, &[("session", "abc")]);
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod sink;
mod tracing_sink;

#[cfg(feature = "otlp")]
pub mod otlp_exporter;

#[cfg(test)]
mod tests;

pub use sink::{
    InMemoryTelemetry, NoopTelemetry, RecordedCounter, RecordedHistogram, RecordedSpan, SpanGuard,
    TelemetrySink,
};
pub use tracing_sink::TracingTelemetry;

/// Span names from §18.1.
pub mod spans {
    /// Top of the per-turn trace tree.
    pub const AGENT_TURN: &str = "agent.turn";
    /// Context assembly (system prompt + memory + skills).
    pub const AGENT_CONTEXT_BUILD: &str = "agent.context.build";
    /// Cost-router classification (Amendment 1, §24).
    pub const ROUTING_CLASSIFY: &str = "routing.classify";
    /// Provider call.
    pub const AGENT_PROVIDER_REQUEST: &str = "agent.provider.request";
    /// Streaming response.
    pub const AGENT_PROVIDER_STREAM: &str = "agent.provider.stream";
    /// Tool planning (which tools to dispatch).
    pub const AGENT_TOOLS_PLAN: &str = "agent.tools.plan";
    /// Tool execution.
    pub const AGENT_TOOLS_EXECUTE: &str = "agent.tools.execute";
    /// Judge adjudication (Amendment 2, §25).
    pub const JUDGE_ADJUDICATE: &str = "judge.adjudicate";
    /// Final-state persistence.
    pub const AGENT_PERSIST: &str = "agent.persist";
    /// Argo Forge optimization run (separate tree).
    pub const FORGE_RUN: &str = "forge.run";
    /// Time-travel session replay (separate tree).
    pub const SESSION_REPLAY: &str = "session.replay";
    /// Workspace bundle push.
    pub const MARKETPLACE_PUSH: &str = "marketplace.push";
    /// Workspace bundle pull.
    pub const MARKETPLACE_PULL: &str = "marketplace.pull";

    /// All span names declared in §18.1, sorted.
    #[must_use]
    pub fn all() -> Vec<&'static str> {
        let mut v = vec![
            AGENT_TURN,
            AGENT_CONTEXT_BUILD,
            ROUTING_CLASSIFY,
            AGENT_PROVIDER_REQUEST,
            AGENT_PROVIDER_STREAM,
            AGENT_TOOLS_PLAN,
            AGENT_TOOLS_EXECUTE,
            JUDGE_ADJUDICATE,
            AGENT_PERSIST,
            FORGE_RUN,
            SESSION_REPLAY,
            MARKETPLACE_PUSH,
            MARKETPLACE_PULL,
        ];
        v.sort_unstable();
        v
    }
}

/// Metric names from §18.2.
pub mod metrics {
    /// Counter (by tier) of cost-router decisions.
    pub const ROUTING_DECISIONS_TOTAL: &str = "routing_decisions_total";
    /// Histogram of classifier latency.
    pub const ROUTING_CLASSIFY_DURATION_MS: &str = "routing_classify_duration_ms";
    /// Counter (by verdict) of judge verdicts.
    pub const JUDGE_VERDICTS_TOTAL: &str = "judge_verdicts_total";
    /// Histogram of judge latency.
    pub const JUDGE_DURATION_MS: &str = "judge_duration_ms";
    /// Counter (by status) of Forge optimization runs.
    pub const FORGE_RUNS_TOTAL: &str = "forge_runs_total";
    /// Counter of Forge spend, in USD-micros.
    pub const FORGE_SPEND_USD_MICROS_TOTAL: &str = "forge_spend_usd_micros_total";
    /// Counter (by `cap_kind`) of hard cost-cap blocks.
    pub const SPEND_CAP_BLOCKS_TOTAL: &str = "spend_cap_blocks_total";
    /// Counter of skill / memory lock violations.
    pub const LOCK_VIOLATIONS_TOTAL: &str = "lock_violations_total";

    /// All metric names declared in §18.2, sorted.
    #[must_use]
    pub fn all() -> Vec<&'static str> {
        let mut v = vec![
            ROUTING_DECISIONS_TOTAL,
            ROUTING_CLASSIFY_DURATION_MS,
            JUDGE_VERDICTS_TOTAL,
            JUDGE_DURATION_MS,
            FORGE_RUNS_TOTAL,
            FORGE_SPEND_USD_MICROS_TOTAL,
            SPEND_CAP_BLOCKS_TOTAL,
            LOCK_VIOLATIONS_TOTAL,
        ];
        v.sort_unstable();
        v
    }
}
