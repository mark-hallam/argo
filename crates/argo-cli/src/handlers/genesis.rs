//! `argo genesis create` handler. Wraps the `argo-genesis`
//! crate's pipeline with the cli-specific concerns: arg
//! parsing, JSON-vs-text output, and ArgoError mapping.
//!
//! See `docs/build-plan/streams/G-genesis.md` for the full
//! brief. The pipeline itself + the safety judge live in
//! `argo-genesis`.

use argo_genesis::{plan, write_vertical, GenesisPlan};
use argo_types::{ArgoError, ArgoResult};
use std::path::Path;

/// Run `argo genesis create --charter ... --persona ... --output ...`.
/// Returns the resolved plan + the absolute output dir; the
/// caller prints whichever shape the operator asked for.
pub(crate) fn create(
    charter: &str,
    persona: &str,
    output: &Path,
    json: bool,
) -> ArgoResult<()> {
    let plan = plan(charter, persona).map_err(ArgoError::from)?;
    let abs_output = output.to_path_buf();
    write_vertical(&plan, &abs_output).map_err(ArgoError::from)?;
    print_plan(&plan, &abs_output, json);
    Ok(())
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
        let err = create("   ", "maya", &out, false).expect_err("empty charter");
        assert!(err.to_string().contains("charter is empty"), "got: {err}");
    }

    #[test]
    fn create_errors_on_invalid_persona() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("doesnt-matter");
        let err = create("be my marketing intern", "maya@", &out, false)
            .expect_err("bad persona");
        assert!(err.to_string().contains("invalid persona"), "got: {err}");
    }
}
