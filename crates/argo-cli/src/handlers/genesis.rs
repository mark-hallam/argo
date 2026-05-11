//! `argo genesis create` handler. Wraps the `argo-genesis`
//! crate's pipeline with the cli-specific concerns: arg
//! parsing, JSON-vs-text output, and ArgoError mapping.
//!
//! See `docs/build-plan/streams/G-genesis.md` for the full
//! brief. The pipeline itself + the safety judge live in
//! `argo-genesis`.

use crate::cli::GenesisCmd;
use argo_genesis::archetype::{list_archetypes, show_archetype};
use argo_genesis::{
    genesis_from_job_description, plan, refine, refine_plan, write_vertical, GenesisDraft,
    GenesisPlan, JobDescInput, RefineConfig,
};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::path::{Path, PathBuf};

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
            allow_broad_role,
        } => create(&charter, &persona, &output, json, refine, allow_broad_role),
        GenesisCmd::Refine {
            vertical_id,
            judge,
            no_judge,
            max_iterations,
            dry_run,
            json,
            show_vocab_evolution,
        } => super::genesis_refine::run(
            &vertical_id,
            judge,
            no_judge,
            max_iterations,
            dry_run,
            json,
            show_vocab_evolution,
        ),
        GenesisCmd::Revise {
            vertical,
            feedback,
            target,
            dry_run,
            json,
        } => super::genesis_revise::handle_revise(super::genesis_revise::GenesisReviseArgs {
            vertical,
            feedback,
            target,
            dry_run,
            json,
            verticals_dir: None,
        }),
        GenesisCmd::Interactive {
            output,
            non_interactive,
            json,
            refine,
        } => super::genesis_interactive::run(non_interactive, &output, json, refine),
        GenesisCmd::FromJobDescription { input, json } => from_job_description(&input, json),
        GenesisCmd::FromVertical {
            charter_file,
            seed,
            persona,
            output,
            json,
        } => super::genesis_from_vertical::handle_from_vertical(
            &charter_file,
            &seed,
            &persona,
            &output,
            json,
        ),
        GenesisCmd::Export {
            vertical,
            output,
            include_skills,
            manifest_only,
            force,
        } => super::genesis_export::handle_export(super::genesis_export::GenesisExportArgs {
            vertical,
            output,
            include_skills,
            manifest_only,
            force,
        }),
        GenesisCmd::Import {
            source,
            name,
            format,
            output,
            json,
        } => {
            let format = format.parse::<super::genesis_import::ImportFormat>()?;
            super::genesis_import::handle_import(super::genesis_import::GenesisImportArgs {
                source,
                name,
                format,
                output,
                json,
            })
        }
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
    println!(
        "recommended channels: {}",
        meta.recommended_channels.join(", ")
    );
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
    allow_broad_role: bool,
) -> ArgoResult<()> {
    // Early validation: when --refine is on, the API key MUST be
    // present before we do any work.
    let refine_config = if refine_flag {
        Some(RefineConfig::from_env().map_err(ArgoError::from)?)
    } else {
        None
    };
    // `--allow-broad-role` is plumbed through but currently only the
    // deterministic keyword classifier is wired into `argo genesis
    // create`. The keyword classifier returns
    // `confidence=1.0` on any matched charter, so this CLI surface
    // is functionally a no-op until a `BroadRoleClient` +
    // `GenesisJudgeClient` are wired (mission follow-up). The flag
    // is exposed today so operator tooling can opt in once the
    // clients land, without a second CLI release.
    if allow_broad_role {
        tracing::info!(
            target: "argo_cli::genesis",
            "--allow-broad-role accepted; broad-role generation requires LLM clients \
             that argo-genesis::plan_or_broad will use once wired into the CLI handler",
        );
    }
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
fn run_refinement(plan: GenesisPlan, config: RefineConfig) -> ArgoResult<GenesisPlan> {
    let caller = refine::http::ReqwestCaller::new(config).map_err(ArgoError::from)?;
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|e| ArgoError::Provider(format!("genesis --refine needs a tokio runtime: {e}")))?;
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

/// Run `argo genesis from-job-description <input> [--json]`.
/// Mission `mission-genesis-job-description-ingest` (#64). See
/// the cli-level docs on [`crate::cli::GenesisCmd::FromJobDescription`]
/// for input dispatch precedence.
pub(crate) fn from_job_description(input: &str, json: bool) -> ArgoResult<()> {
    let parsed = parse_job_description_input(input);
    let handle = tokio::runtime::Handle::try_current().map_err(|e| {
        ArgoError::Provider(format!("genesis from-job-description needs a tokio runtime: {e}"))
    })?;
    let draft =
        tokio::task::block_in_place(|| handle.block_on(genesis_from_job_description(parsed)))?;
    if json {
        print_draft_json(&draft);
    } else {
        print_draft_text(&draft);
    }
    Ok(())
}

fn parse_job_description_input(input: &str) -> JobDescInput {
    if input.starts_with("http://") || input.starts_with("https://") {
        return JobDescInput::Url(input.to_string());
    }
    let path = PathBuf::from(input);
    if path.is_file() {
        if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
        {
            return JobDescInput::Pdf(path);
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            return JobDescInput::Text(text);
        }
    }
    JobDescInput::Text(input.to_string())
}

fn print_draft_json(draft: &GenesisDraft) {
    let payload = serde_json::json!({
        "synthesized_charter": draft.synthesized_charter,
        "archetype": draft.archetype,
        "vertical_id": draft.manifest.vertical.id,
        "action_count": draft
            .manifest
            .actions
            .as_ref()
            .map(|a| a.actions.len())
            .unwrap_or(0),
        "eval_seed": draft.eval_seed,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
    );
}

fn print_draft_text(draft: &GenesisDraft) {
    println!("Genesis draft (from job description):");
    println!("  archetype:   {:?}", draft.archetype);
    println!("  vertical id: {}", draft.manifest.vertical.id);
    println!(
        "  actions:     {}",
        draft
            .manifest
            .actions
            .as_ref()
            .map(|a| a.actions.len())
            .unwrap_or(0)
    );
    println!("  eval seeds:  {}", draft.eval_seed.len());
    println!("  charter ({} chars):", draft.synthesized_charter.len());
    for line in draft.synthesized_charter.lines().take(20) {
        println!("    {line}");
    }
    if draft.synthesized_charter.lines().count() > 20 {
        println!("    …");
    }
    println!(
        "Next: review the charter above. To persist, copy into a file then run \
         `argo genesis create --charter <file> --persona <name> --output <dir>`."
    );
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
        let err = create("   ", "maya", &out, false, false, false).expect_err("empty charter");
        assert!(err.to_string().contains("charter is empty"), "got: {err}");
    }

    #[test]
    fn create_errors_on_invalid_persona() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("doesnt-matter");
        let err = create("be my marketing intern", "maya@", &out, false, false, false)
            .expect_err("bad persona");
        assert!(err.to_string().contains("invalid persona"), "got: {err}");
    }
}
