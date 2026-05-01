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
use argo_agent_runner::{build_transport, load_manifest};
use argo_audit::{AuditKind, AuditSink};
use argo_eval::{load_jsonl, run_eval, EvalRecord, EvalTask};
use argo_provider::{ProviderRequest, Sampling, ToolSchema};
use argo_skill::{PrincipalOverride, SkillStore};
use argo_types::{ArgoError, ArgoResult};
use std::path::PathBuf;
use std::sync::Arc;

pub(super) async fn run(
    skill_name: &str,
    examples_path: &str,
    iterations: u32,
    budget_usd: f64,
    stream: bool,
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
    if !stream {
        println!(
            "argo forge optimize: {skill_name} v{} → {} iters × {} task(s); budget ${budget_usd:.2}",
            baseline.frontmatter.version,
            iterations,
            tasks.len()
        );
    }

    let ws_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let ws_root = cwd.to_string_lossy();
    let (tenant, workspace) = store.ensure_workspace(ws_name, &ws_root).await?;
    let run_id = store
        .start_forge_run(skill_name, &baseline.frontmatter.version, tasks.len() as u32)
        .await?;
    if stream {
        emit_event(&serde_json::json!({
            "event": "start",
            "run_id": run_id.to_string(),
            "skill_name": skill_name,
            "base_version": baseline.frontmatter.version,
            "iterations": iterations,
            "examples": examples_path,
            "n_tasks": tasks.len(),
        }));
    }
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
    // Records from the previous iter — used to drive the next
    // mutation by feeding the meta-prompt the failed examples.
    let mut prev_records: Vec<EvalRecord> = Vec::new();

    if !stream {
        println!("\n  iter  version    mean_score  pass_rate  errors  best_so_far");
    }
    for iter in 0..iterations {
        let version_for_iter = if iter == 0 {
            baseline.frontmatter.version.clone()
        } else {
            let candidate_version = bump_version(&baseline.frontmatter.version, iter);
            // Use the LLM mutator if there were failures last
            // iter; otherwise fall back to the mechanical
            // refinement template (so we still try *something*
            // when everything passed but score was sub-1.0).
            let candidate_body = match mutate_via_llm(
                &cwd,
                &subject.provider,
                &subject.model,
                &baseline.body,
                &tasks,
                &prev_records,
            )
            .await
            {
                Ok(Some(body)) => body,
                Ok(None) | Err(_) => mechanical_refinement(&baseline.body, iter),
            };
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

        let (records, summary) = run_eval(&subject, &tasks).await;
        prev_records = records;
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
        if stream {
            emit_event(&serde_json::json!({
                "event": "iter",
                "run_id": run_id.to_string(),
                "iter": iter,
                "version": version_for_iter,
                "mean_score": summary.mean_score,
                "pass_rate": summary.pass_rate,
                "errors": summary.errors,
                "best_so_far": best_score,
            }));
        } else {
            println!(
                "  {iter:>3}   {ver:<9}  {mean:.3}      {pr:.3}     {errs:<6}  {best:.3}",
                iter = iter,
                ver = version_for_iter,
                mean = summary.mean_score,
                pr = summary.pass_rate,
                errs = summary.errors,
                best = best_score,
            );
        }

        if let Ok(Some(row)) = store.get_forge_run(run_id).await {
            if budget_micros > 0 && row.spend_usd_micros >= budget_micros {
                last_status = "partial";
                if stream {
                    emit_event(&serde_json::json!({
                        "event": "budget_exceeded",
                        "run_id": run_id.to_string(),
                        "iter": iter,
                        "spend_usd_micros": row.spend_usd_micros,
                        "cap_usd_micros": budget_micros,
                    }));
                } else {
                    println!(
                        "  (over budget after iter {iter}: spend=${:.2} > cap=${budget_usd:.2})",
                        row.spend_usd_micros as f64 / 1_000_000.0
                    );
                }
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
    if stream {
        emit_event(&serde_json::json!({
            "event": "done",
            "run_id": run_id.to_string(),
            "status": last_status,
            "best_score": final_score,
            "best_version": best_version,
            "iterations_run": completed,
            "iterations_planned": iterations,
            "promoted": true,
        }));
    } else {
        println!(
            "\nsummary: status={last_status} best_score={best_score:.3} \
             best_version={best_version} iters_run={completed}/{iterations} (run_id={run_id})"
        );
        println!("  → promoted {skill_name} v{best_version} to active");
    }
    Ok(())
}

/// Emit one NDJSON event line on stdout. Used by the `--stream`
/// mode so the TUI / watch scripts / gateway can consume forge
/// progress without parsing the table.
fn emit_event(value: &serde_json::Value) {
    use std::io::Write;
    if let Ok(line) = serde_json::to_string(value) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

/// Fallback mutator: append a fixed `## Refinement attempt N`
/// block. Used when the LLM mutator can't or shouldn't run
/// (transport build failure, no failed examples to learn from,
/// etc).
fn mechanical_refinement(baseline_body: &str, iter: u32) -> String {
    format!(
        "{}\n\n## Refinement attempt {iter}\n\
         Be precise, structured, and avoid filler. Cite tool \
         outputs verbatim when relevant.\n",
        baseline_body.trim_end()
    )
}

/// LLM-driven mutator: ask the configured provider/model to
/// propose a refined skill body, given the baseline body + the
/// records from the previous bench iteration (so the prompt
/// shows which examples failed and what the wrong output was).
///
/// Returns `Ok(None)` when there's nothing to learn from
/// (no failed records yet, e.g. the very first iteration). The
/// caller falls back to the mechanical template in that case.
async fn mutate_via_llm(
    cwd: &std::path::Path,
    provider_id: &str,
    model: &str,
    baseline_body: &str,
    tasks: &[EvalTask],
    prev_records: &[EvalRecord],
) -> ArgoResult<Option<String>> {
    if prev_records.is_empty() {
        return Ok(None);
    }
    let failures: Vec<&EvalRecord> = prev_records.iter().filter(|r| !r.passed).collect();
    if failures.is_empty() {
        return Ok(None);
    }
    let mut failure_block = String::new();
    for r in failures.iter().take(8) {
        let task = tasks.iter().find(|t| t.name == r.task);
        let input = task.map(|t| t.input.as_str()).unwrap_or("?");
        let expected = task.map(|t| t.expected.as_str()).unwrap_or("?");
        let actual: String = r.output.chars().take(280).collect();
        failure_block.push_str(&format!(
            "- task: {name}\n  input: {input}\n  expected: {expected}\n  actual: {actual}\n  score: {score:.3}\n\n",
            name = r.task,
            score = r.score,
        ));
    }
    let user_prompt = format!(
        "You are tuning a workspace skill. The current skill body is:\n\n\
         ```markdown\n{baseline}\n```\n\n\
         The skill was just benched against an eval-task suite. \
         Below are the tasks that *failed*:\n\n{failures}\n\
         Propose a refined skill body that addresses these failures \
         while preserving correct behaviour on passing tasks. Reply \
         with ONLY the refined Markdown body — no preamble, no \
         frontmatter, no code fences.",
        baseline = baseline_body.trim(),
        failures = failure_block,
    );

    let manifest = load_manifest(cwd, provider_id)?;
    let transport = build_transport(manifest).await?;
    let req = ProviderRequest {
        model: model.to_string(),
        messages: vec![argo_types::Message {
            id: argo_types::MessageId(uuid::Uuid::now_v7()),
            session_id: argo_types::SessionId(uuid::Uuid::nil()),
            role: argo_types::MessageRole::User,
            content: vec![argo_types::ContentBlock::Text { text: user_prompt }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            reasoning: None,
            created_at: chrono::Utc::now(),
            provider_meta: serde_json::Value::Null,
        }],
        tools: Vec::<ToolSchema>::new(),
        system: Some(
            "You refine workspace skills based on bench-result feedback. \
             Output only the refined skill body."
                .to_string(),
        ),
        sampling: Sampling::default(),
    };
    let resp = transport.complete(&req).await?;
    let text = resp.text.unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(strip_code_fence(&text)))
}

/// If the LLM wrapped the body in a Markdown code fence
/// (`\`\`\`markdown … \`\`\``), strip it. Idempotent on
/// fence-free inputs.
fn strip_code_fence(s: &str) -> String {
    let t = s.trim();
    let stripped = t.strip_prefix("```markdown").or_else(|| t.strip_prefix("```"));
    if let Some(after_open) = stripped {
        let body = after_open.trim_start_matches('\n');
        if let Some(idx) = body.rfind("```") {
            return body[..idx].trim_end().to_string();
        }
    }
    t.to_string()
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

    #[test]
    fn strip_code_fence_drops_markdown_wrapper() {
        let wrapped = "```markdown\nbody line\nmore\n```\n";
        assert_eq!(strip_code_fence(wrapped), "body line\nmore");
    }

    #[test]
    fn strip_code_fence_drops_plain_fence() {
        let wrapped = "```\nbody\n```";
        assert_eq!(strip_code_fence(wrapped), "body");
    }

    #[test]
    fn strip_code_fence_idempotent_on_unfenced_text() {
        assert_eq!(strip_code_fence("just text"), "just text");
    }

    #[test]
    fn mechanical_refinement_appends_a_block() {
        let refined = mechanical_refinement("base body", 3);
        assert!(refined.contains("base body"));
        assert!(refined.contains("## Refinement attempt 3"));
    }
}
