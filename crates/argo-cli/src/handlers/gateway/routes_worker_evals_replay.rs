//! `GET /api/v1/workers/<id>/evals/replay` — store-backed teammate
//! historical-replay scorecard.
//!
//! Closes the DEC-017 phase-2 "live-outcome producer" seam: until now
//! [`argo_eval::replay`] defined a [`ProductionOutcomeSource`] trait +
//! [`replay_report`] but shipped only the in-memory `VecOutcomeSource`,
//! so the teammate-eval scorer had **zero live consumers**. This handler
//! is the live consumer — a store-backed [`StoreOutcomeSource`] reads a
//! worker's real past interactions off the tamper-evident audit chain and
//! feeds them to the SAME `replay_report` + `build_scorecard` the unit
//! tests exercise, so the teammate is graded on what it actually did.
//!
//! Two inputs are merged into one scorecard:
//! - **synthetic** rubrics + golden outcomes loaded from disk
//!   (`eval/scenarios/<teammate>/*.jsonl`, `$ARGO_EVAL_SCENARIOS_DIR`
//!   overridable) — the hand-authored expectation set.
//! - **production** interactions from `audit_record` — rows whose
//!   `payload_json` carries a `replay_outcome` object shaped like a
//!   [`ReplayedInteraction`]. A future producer (the runtime / an
//!   `argo eval … --record` path) writes those rows; this brief only
//!   wires the **consumer**, so a worker with no such rows replays as
//!   production-empty (NO THEATER — an empty production set is honest,
//!   never a fabricated score).
//!
//! Tenant scoping mirrors the sibling worker GETs: the tenant is
//! resolved via [`helpers::resolve_worker_ctx`] and a worker id that
//! belongs to another tenant is indistinguishable from one that does not
//! exist (`404`), so tenant A cannot read tenant B's replay scorecard.
//! The production query is additionally pinned to
//! `tenant_id = <resolved>`.

use super::routes_worker_skills_helpers as helpers;
use argo_audit::AuditKind;
use argo_eval::replay::{
    pair_replayed, replay_report, ProductionOutcomeSource, ReplayedInteraction,
};
use argo_eval::teammate::TeammateScenario;
use argo_eval::teammate_runner::GoldenCase;
use argo_eval::teammate_scorecard::{
    build_scorecard, load_golden_cases, Scorecard, DEFAULT_THRESHOLD,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Default number of recent production interactions pulled per replay.
const DEFAULT_LIMIT: usize = 50;
/// `$ARGO_EVAL_SCENARIOS_DIR` wins; default is `./eval/scenarios`.
const ENV_SCENARIOS_DIR: &str = "ARGO_EVAL_SCENARIOS_DIR";

/// Store-backed [`ProductionOutcomeSource`] — reads a worker's real past
/// interactions off the `audit_record` chain. The live wire for the seam
/// `argo-eval` keeps trait-only (so the pure scorer stays CI-runnable).
struct StoreOutcomeSource {
    store: Arc<SessionStore>,
    tenant: TenantId,
}

#[async_trait]
impl ProductionOutcomeSource for StoreOutcomeSource {
    async fn recent_outcomes(
        &self,
        teammate: &str,
        limit: usize,
    ) -> ArgoResult<Vec<ReplayedInteraction>> {
        // Pin tenant (the cross-tenant guard), match the worker by its id
        // appearing in the payload (same pattern as
        // `routes_worker_skills_helpers::fetch_history`, robust to how the
        // `worker_id` filter column is populated), and require a
        // `replay_outcome` object so only genuinely replay-shaped rows are
        // returned.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT payload_json FROM audit_record \
             WHERE tenant_id = ? AND payload_json LIKE ? \
               AND payload_json LIKE '%replay_outcome%' \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(self.tenant.to_string())
        .bind(format!("%{teammate}%"))
        .bind(limit as i64)
        .fetch_all(self.store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("replay outcomes select: {e}")))?;
        Ok(rows
            .into_iter()
            .filter_map(|(payload,)| parse_interaction(&payload))
            .collect())
    }
}

/// Extract a [`ReplayedInteraction`] from an audit `payload_json`. The
/// production row carries it under a `replay_outcome` key; rows that do
/// not deserialize are skipped (never fabricated).
fn parse_interaction(payload_json: &str) -> Option<ReplayedInteraction> {
    let v: Value = serde_json::from_str(payload_json).ok()?;
    let inner = v.get("replay_outcome")?;
    serde_json::from_value(inner.clone()).ok()
}

/// `GET /api/v1/workers/<id>/evals/replay` entry point.
pub(super) async fn replay_response(
    worker_id_raw: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let worker_id = match helpers::parse_worker_id(worker_id_raw) {
        Ok(id) => id,
        Err(r) => return r,
    };
    // Tenant-scoped: cross-tenant / unknown both surface as 404.
    let ctx = match helpers::resolve_worker_ctx(store, req, worker_id).await {
        Ok(c) => c,
        Err(r) => return r,
    };
    let teammate = worker_id.to_string();
    let limit = parse_limit(req);
    let threshold = DEFAULT_THRESHOLD;

    let synthetic = load_synthetic_cases(&teammate);
    let scenarios: Vec<TeammateScenario> =
        synthetic.iter().map(|c| c.scenario.clone()).collect();

    let source = StoreOutcomeSource {
        store: Arc::clone(store),
        tenant: ctx.tenant,
    };
    // The store-backed source drives `replay_report` (proves it consumed
    // live rows, not an empty vec). A second read supplies the paired
    // production cases for the merged scorecard.
    let report = match replay_report(&source, &teammate, &scenarios, limit, threshold).await {
        Ok(r) => r,
        Err(e) => return helpers::server_error(&format!("replay_report: {e}")),
    };
    let interactions = match source.recent_outcomes(&teammate, limit).await {
        Ok(v) => v,
        Err(e) => return helpers::server_error(&format!("recent_outcomes: {e}")),
    };
    let pairing = pair_replayed(&scenarios, &interactions);

    let synthetic_card = build_scorecard(&teammate, &synthetic, threshold);
    let mut merged_cases = synthetic.clone();
    merged_cases.extend(pairing.cases.iter().cloned());
    let merged_card = build_scorecard(&teammate, &merged_cases, threshold);

    helpers::emit_audit(
        store,
        req,
        &ctx,
        AuditKind::HttpRequest,
        "worker.evals.replay.viewed",
        json!({
            "synthetic_count": synthetic.len(),
            "production_scored": report.scored,
            "limit": limit,
        }),
    )
    .await;

    let body = json!({
        "workerId": teammate,
        "teammate": teammate,
        "threshold": threshold,
        "limit": limit,
        "synthetic": scorecard_json(&synthetic_card),
        "production": {
            "scored": report.scored,
            "unmatched": report.unmatched,
            "uncovered": report.uncovered,
            "scorecard": scorecard_json(&report.scorecard),
        },
        "merged": scorecard_json(&merged_card),
    });
    ("200 OK", body.to_string())
}

/// Resolve the synthetic-scenarios root (`$ARGO_EVAL_SCENARIOS_DIR` or
/// `./eval/scenarios`).
fn scenarios_root() -> PathBuf {
    std::env::var(ENV_SCENARIOS_DIR)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("eval/scenarios"))
}

/// Load every `*.jsonl` golden file under `<root>/<teammate>/`, in name
/// order. A missing dir / unreadable file yields an empty set — the
/// replay then reports production-only (no fabrication).
fn load_synthetic_cases(teammate: &str) -> Vec<GoldenCase> {
    let dir = scenarios_root().join(teammate);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
        .collect();
    paths.sort();
    let mut cases = Vec::new();
    for p in paths {
        if let Ok(mut c) = load_golden_cases(&p) {
            cases.append(&mut c);
        }
    }
    cases
}

/// Parse an optional `?limit=<n>` (1..=500); falls back to
/// [`DEFAULT_LIMIT`].
fn parse_limit(req: &str) -> usize {
    req.split_once("limit=")
        .and_then(|(_, rest)| {
            let end = rest
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(rest.len());
            rest[..end].parse::<usize>().ok()
        })
        .filter(|n| *n > 0 && *n <= 500)
        .unwrap_or(DEFAULT_LIMIT)
}

/// Render a [`Scorecard`] into the camelCase JSON the SPA reads, with the
/// headline `passRate` + `weakestAxis` diagnostic surfaced.
fn scorecard_json(c: &Scorecard) -> Value {
    json!({
        "teammate": c.teammate,
        "passed": c.passed,
        "total": c.total,
        "passRate": c.pass_rate(),
        "axisMeans": c.axis_means,
        "threshold": c.threshold,
        "weakestAxis": c.weakest_axis().map(|(axis, mean)| json!({ "axis": axis, "mean": mean })),
    })
}

#[cfg(test)]
#[path = "routes_worker_evals_replay_tests.rs"]
mod tests;
