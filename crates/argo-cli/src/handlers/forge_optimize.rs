//! `argo forge optimize` — score-driven candidate sweep.
//! Lives in its own module so `forge.rs` stays under the
//! 500-LOC ceiling.
//!
//! Iter 0 benches the *baseline* active skill; iters 1..N each:
//! 1. mutate the baseline body with a per-iteration hint,
//! 2. supersede the active row with the candidate,
//! 3. run the bench against the same examples,
//! 4. record the score.
//!
//! At the end, the highest-scoring version is promoted to
//! active so the next agent turn picks it up. Mutation is
//! mechanical (a templated `Refinement attempt N` block
//! appended to the body); LLM-driven mutation is a follow-up.
use super::forge_subject::{open_store, BenchSubject};
use crate::home::resolve_home;
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval};
use argo_skill::{PrincipalOverride, SkillStore};
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) async fn run(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let _home = resolve_home(&cwd)?;
    let store = Arc::new(open_store().await?);
    let skills = SkillStore::new(Arc::clone(&store));
    let baseline = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;

    let tasks = load_jsonl(&PathBuf::from(examples_path)).await?;
    if tasks.is_empty() {
        return Err(ArgoError::Provider(format!(
            "no eval tasks parsed from {examples_path}"
        )));
    }
    println!(
        "argo forge optimize: {skill_name} v{} → {} iters × {} task(s); budget ${budget_usd:.2}",
        baseline.frontmatter.version,
        iterations,
        tasks.len()
    );

    let ws_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let ws_root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(ws_name, &ws_root).await?;
    let run_id = store
        .start_forge_run(skill_name, &baseline.frontmatter.version, tasks.len() as u32)
        .await?;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunStart,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "skill_name": skill_name,
                "examples": examples_path,
                "iterations": iterations,
                "budget_usd": budget_usd,
            }),
        )
        .await;

    let subject = BenchSubject {
        store: Arc::clone(&store),
        cwd: cwd.clone(),
        tenant,
        workspace,
        provider: "anthropic".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        skill_name: skill_name.to_string(),
    };

    let mut best_score: f64 = f64::NEG_INFINITY;
    let mut best_version = baseline.frontmatter.version.clone();
    let mut completed = 0_i64;
    let mut last_status = "succeeded";
    let budget_micros = (budget_usd * 1_000_000.0).round() as i64;

    println!("\n  iter  version    mean_score  pass_rate  errors  best_so_far");
    for iter in 0..iterations {
        let version_for_iter = if iter == 0 {
            baseline.frontmatter.version.clone()
        } else {
            let candidate_version = bump_version(&baseline.frontmatter.version, iter);
            let candidate_body = format!(
                "{}\n\n## Refinement attempt {iter}\n\
                 Be precise, structured, and avoid filler. Cite tool \
                 outputs verbatim when relevant.\n",
                baseline.body.trim_end()
            );
            let raw = build_skill_md(
                skill_name,
                &candidate_version,
                &baseline.frontmatter.description,
                &candidate_body,
            );
            skills
                .supersede(&raw, None, PrincipalOverride::Force)
                .await?;
            candidate_version
        };

        let (_, summary) = run_eval(&subject, &tasks).await;
        if summary.errors > 0 {
            last_status = if summary.errors == summary.n {
                "failed"
            } else {
                "partial"
            };
        }
        if summary.mean_score > best_score {
            best_score = summary.mean_score;
            best_version = version_for_iter.clone();
        }
        completed += 1;
        println!(
            "  {iter:>3}   {ver:<9}  {mean:.3}      {pr:.3}     {errs:<6}  {best:.3}",
            iter = iter,
            ver = version_for_iter,
            mean = summary.mean_score,
            pr = summary.pass_rate,
            errs = summary.errors,
            best = best_score,
        );

        if let Ok(Some(row)) = store.get_forge_run(run_id).await {
            if budget_micros > 0 && row.spend_usd_micros >= budget_micros {
                last_status = "partial";
                println!(
                    "  (over budget after iter {iter}: spend=${:.2} > cap=${budget_usd:.2})",
                    row.spend_usd_micros as f64 / 1_000_000.0
                );
                break;
            }
        }
    }

    if best_version != baseline.frontmatter.version || iterations > 1 {
        let _ = skills.promote(skill_name, &best_version).await;
    }

    let final_score = if best_score.is_finite() {
        Some(best_score)
    } else {
        None
    };
    let _ = store
        .finish_forge_run(run_id, last_status, final_score, completed)
        .await;
    let _ = AuditSink::new(Arc::clone(&store))
        .write(
            None,
            AuditKind::ForgeRunEnd,
            serde_json::json!({
                "run_id": run_id.to_string(),
                "status": last_status,
                "best_score": final_score,
                "best_version": best_version,
                "iterations_run": completed,
            }),
        )
        .await;
    println!(
        "\nsummary: status={last_status} best_score={best_score:.3} \
         best_version={best_version} iters_run={completed}/{iterations} (run_id={run_id})"
    );
    println!("  → promoted {skill_name} v{best_version} to active");
    Ok(())
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
fn build_skill_md(name: &str, version: &str, description: &str, body: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    let hash = hex::encode(h.finalize());
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: {description}\n\
         lock: none\nprovenance:\n  source: forge-optimize\n  hash: sha256-{hash}\n---\n{body}"
    )
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
}
