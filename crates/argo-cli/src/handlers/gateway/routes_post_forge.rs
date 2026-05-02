//! `POST /forge/optimize` — gateway entry point for forge
//! optimize runs. Lifted out of `routes_post_writes.rs` once
//! v1.1 (#581) flipped the default from "spawn in-gateway
//! Tokio task" to "enqueue against forge_queue and let the
//! daemon claim it" — the new path needs a SessionStore handle
//! plus the skill-store lookup, which made the handler
//! ~110 LOC and pushed `routes_post_writes.rs` over the
//! 500-LOC budget.
use super::routes_post_writes::parse_forge_body;
use super::store_unavailable;
use argo_store::SessionStore;
use std::sync::Arc;

/// `POST /forge/optimize` — kick off `argo forge optimize`.
///
/// Body shape:
/// `{"skill": "...", "examples": "<path>", "iterations": 12,
///   "budget_usd": 5.0}` (iterations + budget_usd optional).
///
/// **Default behaviour (since v1.1 #581)**: enqueues the run
/// against the workspace `forge_queue` table; an `argo forge
/// daemon` worker claims and runs it on its next tick. The
/// gateway no longer hosts the forge loop in-process.
///
/// **Backward-compat escape hatch**: pass `?in_process=true` on
/// the URL to keep the old in-gateway Tokio-task behaviour
/// (the gateway runs the loop itself; no daemon needed).
/// Useful for CI / single-process setups, but doesn't survive
/// gateway restarts.
///
/// Response shape always carries both `enqueued` and `started`
/// so existing callers that check `started` keep working.
pub(super) async fn forge_optimize_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let parsed = match parse_forge_body(req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let skill = parsed.skill;
    let examples = parsed.examples;
    let iterations = parsed
        .raw
        .get("iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(12) as u32;
    let budget_usd = parsed
        .raw
        .get("budget_usd")
        .and_then(|v| v.as_f64())
        .unwrap_or(5.0);
    let in_process = req
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| path.split('?').nth(1))
        .map(|q| {
            q.split('&')
                .any(|kv| matches!(kv, "in_process=true" | "in_process=1"))
        })
        .unwrap_or(false);
    if in_process {
        crate::handlers::forge_optimize::spawn_optimize(
            skill.clone(),
            examples.clone(),
            iterations,
            budget_usd,
        );
        let resp = serde_json::json!({
            "started": true,
            "enqueued": false,
            "kind": "optimize",
            "mode": "in_process",
            "skill": skill,
            "examples": examples,
            "iterations": iterations,
            "budget_usd": budget_usd,
        });
        return (
            "202 Accepted",
            serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
        );
    }
    let Some(store) = store else {
        return store_unavailable();
    };
    // Resolve baseline version + examples count for the
    // forge_run row. Both are best-effort — the daemon
    // surfaces a clearer error if the skill / examples are
    // missing when it actually runs.
    let skills = argo_skill::SkillStore::new(Arc::clone(store));
    let base_version = match skills.find_active_by_name(&skill).await {
        Ok(Some(s)) => s.frontmatter.version,
        _ => String::from("0.0.0"),
    };
    let examples_count = std::fs::read_to_string(&examples)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count() as u32)
        .unwrap_or(0);
    let payload = serde_json::json!({
        "skill": skill,
        "examples": examples,
        "iterations": iterations,
        "budget_usd": budget_usd,
    });
    let payload_str = match serde_json::to_string(&payload) {
        Ok(s) => s,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"serialize payload: {e}\"}}"),
            );
        }
    };
    match store
        .enqueue_forge_run(&skill, &base_version, examples_count, &payload_str)
        .await
    {
        Ok(id) => {
            let resp = serde_json::json!({
                "enqueued": true,
                "started": false,
                "kind": "optimize",
                "mode": "queued",
                "run_id": id.to_string(),
                "skill": skill,
                "examples": examples,
                "iterations": iterations,
                "budget_usd": budget_usd,
            });
            (
                "202 Accepted",
                serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"enqueue_forge_run: {e}\"}}"),
        ),
    }
}
