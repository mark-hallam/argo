//! Helpers extracted from `forge_optimize.rs` to keep it under
//! the 500-LOC ceiling (HARD RULE 6). No public API changes.
use super::forge_optimize::run;
use super::forge_optimize_judge::{
    self, evaluate_promote_gates, write_promote_blocked_audit, PromoteDecision,
};
use argo_judge::Judge;
use argo_skill::{Skill, SkillStore};
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// On-disk body of the winning variant, so the judge adjudicates
/// against the prompt the agent would inherit; baseline on miss.
pub(super) async fn resolve_candidate_body(
    skills: &SkillStore,
    skill_name: &str,
    best_version: &str,
    baseline: &Skill,
) -> Option<String> {
    if best_version == baseline.frontmatter.version {
        return Some(baseline.body.clone());
    }
    let versions = skills.versions_for_name(skill_name).await.ok()?;
    versions
        .into_iter()
        .find(|s| s.frontmatter.version == best_version)
        .map(|s| s.body)
}

/// Async kick-off for `POST /forge/optimize`; caller has 202'd.
pub(crate) fn spawn_optimize(skill: String, examples: String, iterations: u32, budget_usd: f64) {
    tokio::spawn(async move {
        if let Err(e) = run(&skill, &examples, iterations, budget_usd, false).await {
            eprintln!("argo gateway: forge optimize ({skill}) failed: {e}");
        }
    });
}

/// Emit one NDJSON event line on stdout. Used by `--stream` mode.
pub(super) fn emit_event(value: &serde_json::Value) {
    use std::io::Write;
    if let Ok(line) = serde_json::to_string(value) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

/// Fallback mutator when the LLM mutator can't or shouldn't run:
/// append a fixed `## Refinement attempt N` block.
pub(super) fn mechanical_refinement(baseline_body: &str, iter: u32) -> String {
    format!(
        "{}\n\n## Refinement attempt {iter}\n\
         Be precise, structured, and avoid filler. Cite tool \
         outputs verbatim when relevant.\n",
        baseline_body.trim_end()
    )
}

/// Bump the trailing numeric segment of a semver-ish version
/// by `delta` (e.g. `bump_version("0.1", 3) == "0.4"`).
pub(super) fn bump_version(v: &str, delta: u32) -> String {
    let mut parts: Vec<String> = v.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u64>() {
            *last = (n + u64::from(delta)).to_string();
            return parts.join(".");
        }
    }
    format!("{v}-cand{delta}")
}

/// Render a skill-doc string back into the on-disk SKILL.md
/// shape `argo_skill::parse_skill_md` accepts.
pub(super) fn build_skill_md(name: &str, version: &str, description: &str, body: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let hash = hex::encode(h.finalize());
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: {description}\n\
         lock: none\nprovenance:\n  source: forge-optimize\n  hash: sha256-{hash}\n---\n{body}"
    )
}

/// Post-loop promotion gates (threshold + judge, fail-CLOSED on
/// judge availability). Returns the decision and updated status.
#[allow(clippy::too_many_arguments)]
pub(super) async fn evaluate_promote_decision(
    skills: &SkillStore,
    store: &Arc<SessionStore>,
    baseline: &Skill,
    skill_name: &str,
    best_version: &str,
    best_score: f64,
    iterations: u32,
    run_id: uuid::Uuid,
    judge: Option<&dyn Judge>,
    last_status: &'static str,
) -> (PromoteDecision, &'static str) {
    if best_version == baseline.frontmatter.version && iterations <= 1 {
        return (PromoteDecision::Allow, last_status);
    }
    let candidate_body = resolve_candidate_body(skills, skill_name, best_version, baseline)
        .await
        .unwrap_or_else(|| baseline.body.clone());
    let decision = evaluate_promote_gates(
        best_score,
        forge_optimize_judge::promote_threshold(),
        judge,
        SessionId::new(),
        skill_name,
        &baseline.frontmatter.version,
        &candidate_body,
    )
    .await;
    let mut new_status = last_status;
    if matches!(decision, PromoteDecision::Allow) {
        let _ = skills.promote(skill_name, best_version).await;
    } else {
        write_promote_blocked_audit(store, run_id, skill_name, best_version, &decision).await;
        if last_status == "succeeded" {
            new_status = decision.run_status().unwrap_or(last_status);
        }
    }
    (decision, new_status)
}

/// Per-iteration budget check; `Some("partial")` when over cap so
/// the caller breaks the loop. `budget_micros <= 0` disables.
#[allow(clippy::too_many_arguments)]
pub(super) async fn check_iter_budget(
    store: &Arc<SessionStore>,
    run_id: uuid::Uuid,
    iter: u32,
    budget_micros: i64,
    budget_usd: f64,
    interactive_text: bool,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
) -> Option<&'static str> {
    if budget_micros <= 0 {
        return None;
    }
    let row = store.get_forge_run(run_id).await.ok().flatten()?;
    if row.spend_usd_micros < budget_micros {
        return None;
    }
    if interactive_text {
        println!(
            "  (over budget after iter {iter}: spend=${:.2} > cap=${budget_usd:.2})",
            row.spend_usd_micros as f64 / 1_000_000.0
        );
    } else {
        dispatch(
            &serde_json::json!({
                "event": "budget_exceeded",
                "run_id": run_id.to_string(),
                "iter": iter,
                "spend_usd_micros": row.spend_usd_micros,
                "cap_usd_micros": budget_micros,
            }),
            events_tx,
            stream,
        );
    }
    Some("partial")
}

/// End-of-run summary: stdout banner for interactive runs, or a
/// `done` NDJSON event for stream/channel consumers.
#[allow(clippy::too_many_arguments)]
pub(super) fn report_summary(
    interactive_text: bool,
    skill_name: &str,
    last_status: &str,
    best_score: f64,
    best_version: &str,
    completed: i64,
    iterations: u32,
    final_score: Option<f64>,
    promoted: bool,
    run_id: uuid::Uuid,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
) {
    if interactive_text {
        println!(
            "\nsummary: status={last_status} best_score={best_score:.3} \
             best_version={best_version} iters_run={completed}/{iterations} (run_id={run_id})"
        );
        if promoted {
            println!("  → promoted {skill_name} v{best_version} to active");
        } else {
            println!("  → promotion refused ({last_status}); active version unchanged");
        }
    } else {
        dispatch(
            &serde_json::json!({
                "event": "done",
                "run_id": run_id.to_string(),
                "status": last_status,
                "best_score": final_score,
                "best_version": best_version,
                "iterations_run": completed,
                "iterations_planned": iterations,
                "promoted": promoted,
            }),
            events_tx,
            stream,
        );
    }
}

fn dispatch(
    payload: &serde_json::Value,
    events_tx: Option<&tokio::sync::mpsc::UnboundedSender<serde_json::Value>>,
    stream: bool,
) {
    if let Some(tx) = events_tx {
        let _ = tx.send(payload.clone());
    } else if stream {
        emit_event(payload);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_version_increments_trailing_number() {
        assert_eq!(bump_version("0.1", 1), "0.2");
        assert_eq!(bump_version("0.1", 3), "0.4");
        assert_eq!(bump_version("1.5.3", 2), "1.5.5");
    }

    #[test]
    fn bump_version_falls_back_for_non_numeric_tail() {
        assert_eq!(bump_version("alpha", 1), "alpha-cand1");
    }

    #[test]
    fn mechanical_refinement_appends_a_block() {
        let refined = mechanical_refinement("base body", 3);
        assert!(refined.contains("base body"));
        assert!(refined.contains("## Refinement attempt 3"));
    }
}
