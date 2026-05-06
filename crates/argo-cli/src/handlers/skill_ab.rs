//! `argo skill versions / ab-test / ab-status / ab-promote`
//! handlers (V2.0 — `claude-skill-versioning-ab-test`).
//!
//! All four read/write the same DB the rest of `argo skill` uses
//! (workspace SessionStore at `<home>/<store>.db`). The A/B test
//! run + outcome tables are migrated lazily via
//! `argo_skill::ensure_ab_schema` so a workspace upgrades on first
//! use rather than needing a `cargo run argo db migrate` step.

use crate::cli::cli_skill_ab_cmds::{
    SkillAbPromoteCmd, SkillAbStatusCmd, SkillAbTestCmd, SkillVersionsCmd,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{
    create_run, ensure_ab_schema, get_run, list_versions, maybe_complete, promote, significance,
    variant_counts, AbTestRun, SkillVersion,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;

/// `argo skill versions <name>` entry.
pub async fn versions(cmd: SkillVersionsCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    let rows = list_versions(&store, &cmd.name).await?;
    if cmd.json {
        println!("{}", json_array(&rows));
    } else if rows.is_empty() {
        println!("(no versions installed for skill '{}')", cmd.name);
    } else {
        println!(
            "{:<14} {:<32} {:<24} {:<14} {:<10}",
            "version", "id", "trust_grade", "lifecycle", "superseded"
        );
        for row in &rows {
            println!(
                "{:<14} {:<32} {:<24} {:<14} {:<10}",
                row.version,
                short(&row.id),
                row.trust_grade,
                row.lifecycle_state,
                row.superseded
            );
        }
    }
    Ok(())
}

/// `argo skill ab-test <name> --variants vN,vM --runs N` entry.
pub async fn ab_test(cmd: SkillAbTestCmd) -> ArgoResult<()> {
    let (a, b) = parse_variants(&cmd.variants)?;
    let store = open_store().await?;
    ensure_ab_schema(&store).await?;
    let run_id = create_run(&store, &cmd.name, &a, &b, cmd.runs).await?;
    let run = get_run(&store, &run_id)
        .await?
        .ok_or_else(|| ArgoError::Provider("ab-test: run inserted but not found".into()))?;
    if cmd.json {
        println!("{}", serde_json::to_string(&run).unwrap_or_default());
    } else {
        println!(
            "ab-test '{}' v{} vs v{} → run id {} (target_runs={})",
            cmd.name, run.variant_a, run.variant_b, run.id, run.target_runs
        );
        println!("  record outcomes via argo_skill::record_outcome(...)");
        println!("  status: argo skill ab-status {}", run.id);
    }
    Ok(())
}

/// `argo skill ab-status <run-id>` entry.
pub async fn ab_status(cmd: SkillAbStatusCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_ab_schema(&store).await?;
    let run = get_run(&store, &cmd.run_id).await?.ok_or_else(|| {
        ArgoError::Provider(format!("ab-status: no run with id '{}'", cmd.run_id))
    })?;
    let (a, b) = variant_counts(&store, &cmd.run_id).await?;
    let sig = significance(a, b, cmd.min_samples);
    // Auto-complete if eligible (idempotent).
    let _ = maybe_complete(&store, &cmd.run_id, cmd.min_samples).await?;
    let run_after = get_run(&store, &cmd.run_id).await?.unwrap_or(run.clone());
    if cmd.json {
        println!(
            "{}",
            json!({
                "run": run_after,
                "counts": {"a": a, "b": b},
                "significance": sig,
            })
        );
    } else {
        print_status(&run_after, a, b, sig);
    }
    Ok(())
}

/// `argo skill ab-promote <run-id> [--winner V]` entry.
pub async fn ab_promote(cmd: SkillAbPromoteCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_ab_schema(&store).await?;
    let winner = promote(&store, &cmd.run_id, cmd.winner.as_deref()).await?;
    if cmd.json {
        println!(
            "{}",
            json!({"run_id": cmd.run_id, "winner": winner, "promoted": true})
        );
    } else {
        println!("ab-promote {}: winner = '{winner}'", cmd.run_id);
    }
    Ok(())
}

fn parse_variants(s: &str) -> ArgoResult<(String, String)> {
    let parts: Vec<&str> = s.split(',').map(str::trim).collect();
    if parts.len() != 2 {
        return Err(ArgoError::Provider(
            "--variants must be '<vA>,<vB>' — exactly two comma-separated versions".into(),
        ));
    }
    if parts[0].is_empty() || parts[1].is_empty() {
        return Err(ArgoError::Provider("--variants: each variant must be non-empty".into()));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn print_status(
    run: &AbTestRun,
    a: argo_skill::VariantCounts,
    b: argo_skill::VariantCounts,
    sig: argo_skill::Significance,
) {
    println!("ab-run {}", run.id);
    println!("  skill:        {}", run.skill_name);
    println!("  status:       {}", run.status);
    println!(
        "  variants:     A={} (success={}/trials={}), B={} (success={}/trials={})",
        run.variant_a, a.successes, a.trials, run.variant_b, b.successes, b.trials
    );
    println!(
        "  chi-square:   {:.3}{}",
        sig.chi_square,
        if sig.significant {
            " (significant; α=0.05)"
        } else {
            " (not yet significant)"
        }
    );
    if let Some(w) = run.winner.as_deref() {
        println!("  winner:       {w}");
    }
}

fn short(id: &str) -> String {
    if id.len() > 30 {
        id[..28].to_string() + ".."
    } else {
        id.to_string()
    }
}

fn json_array(rows: &[SkillVersion]) -> String {
    serde_json::to_string(rows).unwrap_or_else(|_| "[]".into())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variants_accepts_clean_pair() {
        let (a, b) = parse_variants("v1,v2").unwrap();
        assert_eq!(a, "v1");
        assert_eq!(b, "v2");
    }

    #[test]
    fn parse_variants_trims_whitespace() {
        let (a, b) = parse_variants(" v1 , v2 ").unwrap();
        assert_eq!(a, "v1");
        assert_eq!(b, "v2");
    }

    #[test]
    fn parse_variants_rejects_single() {
        let err = parse_variants("v1").unwrap_err();
        assert!(err.to_string().contains("exactly two"));
    }

    #[test]
    fn parse_variants_rejects_empty_part() {
        let err = parse_variants("v1,").unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }
}
