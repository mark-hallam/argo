//! `argo genesis create` handler. Wraps the `argo-genesis`
//! crate's pipeline with the cli-specific concerns: arg
//! parsing, JSON-vs-text output, and ArgoError mapping.
//!
//! See `docs/build-plan/streams/G-genesis.md` for the full
//! brief. The pipeline itself + the safety judge live in
//! `argo-genesis`.

use crate::cli::GenesisCmd;
use argo_genesis::archetype::{list_archetypes, show_archetype};
use argo_genesis::{plan, refine, refine_plan, write_vertical, GenesisPlan, RefineConfig};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
struct Suggestion {
    name: String,
    slug: String,
    confidence: f64,
    matched_keywords: Vec<String>,
}

pub(crate) fn run(cmd: GenesisCmd) -> ArgoResult<()> {
    match cmd {
        GenesisCmd::ListArchetypes { json } => list_archetypes_cmd(json),
        GenesisCmd::ShowArchetype { name, json } => show_archetype_cmd(&name, json),
        GenesisCmd::Suggest { charter, json } => suggest_cmd(&charter, json),
        GenesisCmd::Create {
            charter,
            persona,
            output,
            json,
            refine,
        } => create(&charter, &persona, &output, json, refine),
    }
}

fn list_archetypes_cmd(json: bool) -> ArgoResult<()> {
    let rows = list_archetypes();
    if json {
        print_json(&serde_json::json!({"archetypes": rows}));
        return Ok(());
    }
    println!("name                      actions sensitive description");
    for a in rows {
        println!(
            "{:<25} {:>7} {:>9} {}",
            a.name, a.base_action_count, a.sensitive_verb_count, a.description
        );
    }
    Ok(())
}

fn show_archetype_cmd(name: &str, json: bool) -> ArgoResult<()> {
    let meta = show_archetype(name).ok_or_else(|| {
        ArgoError::Provider(format!(
            "genesis archetype '{name}' not found; run `argo genesis list-archetypes`"
        ))
    })?;
    if json {
        print_json(&meta);
        return Ok(());
    }
    println!("{} ({})", meta.name, meta.slug);
    println!("{}", meta.description);
    println!("persona traits: {}", meta.default_persona_traits.join(", "));
    println!("recommended channels: {}", meta.recommended_channels.join(", "));
    println!("actions:");
    for action in meta.seed_actions {
        println!(
            "  {:<24} risk={:<6} approval={:<5} {}",
            action.name, action.risk, action.requires_approval, action.description
        );
    }
    Ok(())
}

fn suggest_cmd(charter: &str, json: bool) -> ArgoResult<()> {
    if charter.trim().is_empty() {
        return Err(ArgoError::Provider("charter is empty".into()));
    }
    let matches = suggest_archetypes(charter);
    if json {
        print_json(&serde_json::json!({"charter": charter, "matches": matches}));
        return Ok(());
    }
    println!("name                      confidence matched_keywords");
    for m in matches {
        println!(
            "{:<25} {:>10.3} {}",
            m.name,
            m.confidence,
            m.matched_keywords.join(", ")
        );
    }
    Ok(())
}

fn suggest_archetypes(charter: &str) -> Vec<Suggestion> {
    let lowered = charter.to_lowercase();
    let mut scored: Vec<_> = list_archetypes()
        .into_iter()
        .map(|a| {
            let matched: Vec<String> = a
                .keywords
                .iter()
                .filter(|kw| lowered.matches(&kw.to_lowercase()).count() > 0)
                .cloned()
                .collect();
            let score = matched.len();
            (a, matched, score)
        })
        .collect();
    let total = scored.iter().map(|(_, _, score)| *score).sum::<usize>();
    scored.sort_by(|(a, _, sa), (b, _, sb)| sb.cmp(sa).then_with(|| a.name.cmp(&b.name)));
    scored
        .into_iter()
        .take(3)
        .map(|(a, matched, score)| Suggestion {
            name: a.name,
            slug: a.slug,
            confidence: if total == 0 {
                0.0
            } else {
                score as f64 / total as f64
            },
            matched_keywords: matched,
        })
        .collect()
}

fn print_json<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
    );
}

/// Run `argo genesis create --charter ... --persona ... --output ...`.
/// Returns the resolved plan + the absolute output dir; the
/// caller prints whichever shape the operator asked for.
///
/// When `refine` is `true`, the deterministic plan goes through
/// the LLM refinement pass (Stream `claude-genesis-llm-refinement`).
/// We surface a missing `ANTHROPIC_API_KEY` at startup BEFORE the
/// deterministic pipeline runs so the operator sees the gap
/// without writing a half-baked vertical.
pub(crate) fn create(
    charter: &str,
    persona: &str,
    output: &Path,
    json: bool,
    refine_flag: bool,
) -> ArgoResult<()> {
    // Early validation: when --refine is on, the API key MUST be
    // present before we do any work.
    let refine_config = if refine_flag {
        Some(RefineConfig::from_env().map_err(ArgoError::from)?)
    } else {
        None
    };
    let plan = plan(charter, persona).map_err(ArgoError::from)?;
    let plan = match refine_config {
        Some(cfg) => run_refinement(plan, cfg)?,
        None => plan,
    };
    let abs_output = output.to_path_buf();
    write_vertical(&plan, &abs_output).map_err(ArgoError::from)?;
    print_plan(&plan, &abs_output, json);
    Ok(())
}

/// Run the LLM refinement pass on the deterministic plan.
///
/// Builds the production [`refine::http::ReqwestCaller`] inline
/// and drives it via `tokio::task::block_in_place` — the cli is
/// always inside a tokio runtime here (same sync→async bridge
/// the cu executor uses).
fn run_refinement(
    plan: GenesisPlan,
    config: RefineConfig,
) -> ArgoResult<GenesisPlan> {
    let caller = refine::http::ReqwestCaller::new(config).map_err(ArgoError::from)?;
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!("genesis --refine needs a tokio runtime: {e}"))
    })?;
    let refined = tokio::task::block_in_place(|| handle.block_on(refine_plan(plan, &caller)));
    refined.map_err(ArgoError::from)
}

fn print_plan(plan: &GenesisPlan, output: &Path, json: bool) {
    if json {
        let payload = serde_json::json!({
            "vertical_id": plan.vertical_id,
            "persona": plan.persona,
            "archetype": plan.archetype,
            "charter": plan.charter,
            "output_dir": output.display().to_string(),
            "dry_run_hours": argo_genesis::DRY_RUN_HOURS,
            "action_count": plan
                .manifest
                .actions
                .as_ref()
                .map(|a| a.actions.len())
                .unwrap_or(0),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("Generated vertical: {}", plan.vertical_id);
        println!("  archetype: {:?}", plan.archetype);
        println!("  persona:   {}", plan.persona);
        println!("  output:    {}", output.display());
        println!(
            "  actions:   {}",
            plan.manifest
                .actions
                .as_ref()
                .map(|a| a.actions.len())
                .unwrap_or(0)
        );
        println!(
            "  dry_run:   true (auto-clears after {}h — flip the flag in worker.toml when ready)",
            argo_genesis::DRY_RUN_HOURS
        );
        println!(
            "Next: review prompts/system.md, run `argo vertical lint`, \
             then `argo worker create --vertical {}` when you're ready.",
            plan.vertical_id
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_writes_full_layout_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("marketing-intern-maya");
        create(
            "be my marketing intern, run weekly campaigns to cold leads",
            "maya",
            &out,
            true,
            false,
        )
        .expect("create succeeds");
        for relpath in [
            "vertical.toml",
            "worker.toml",
            "prompts/system.md",
            "prompts/persona.md",
            "memory/schema.toml",
            "eval/seed.jsonl",
        ] {
            assert!(out.join(relpath).exists(), "missing: {relpath}");
        }
    }

    #[test]
    fn create_errors_on_empty_charter() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("doesnt-matter");
        let err = create("   ", "maya", &out, false, false).expect_err("empty charter");
        assert!(err.to_string().contains("charter is empty"), "got: {err}");
    }

    #[test]
    fn create_errors_on_invalid_persona() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("doesnt-matter");
        let err = create("be my marketing intern", "maya@", &out, false, false)
            .expect_err("bad persona");
        assert!(err.to_string().contains("invalid persona"), "got: {err}");
    }
}
