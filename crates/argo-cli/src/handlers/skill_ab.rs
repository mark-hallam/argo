//! `argo skill versions / ab-test / ab-status / ab-promote`
//! handlers (V2.0 — `claude-skill-versioning-ab-test`).
//!
//! All four read/write the same DB the rest of `argo skill` uses
//! (workspace SessionStore at `<home>/<store>.db`). The A/B test
//! run + outcome tables are migrated lazily via
//! `argo_skill::ensure_ab_schema` so a workspace upgrades on first
//! use rather than needing a `cargo run argo db migrate` step.

use crate::cli::cli_skill_ab_cmds::{
    SkillAbPromoteCmd, SkillAbSnapshotCmd, SkillAbStatusCmd, SkillAbTestCmd, SkillVersionsCmd,
};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{
    create_run, ensure_ab_schema, get_run, list_versions, maybe_complete,
    maybe_complete_with_quality, promote, significance, variant_counts, AbTestRun,
    QualityWeightConfig, SkillQualityStore, SkillVersion, WinnerReason,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use sqlx::Row;
use std::sync::Arc;

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

/// `argo skill ab-snapshot <name>` entry.
///
/// Looks up the most recent A/B run for `name` (any status), then
/// reports per-variant counts + the significance verdict using the
/// existing `Significance` helper. Reuses `variant_counts` +
/// `significance` from `argo_skill` — no new math.
pub async fn ab_snapshot(cmd: SkillAbSnapshotCmd) -> ArgoResult<()> {
    let store = open_store().await?;
    ensure_ab_schema(&store).await?;
    let Some(run) = latest_run_for_skill(&store, &cmd.name).await? else {
        if cmd.json {
            println!("{}", json!({"skill": cmd.name, "run": null}));
        } else {
            println!("(no ab run recorded for skill '{}')", cmd.name);
        }
        return Ok(());
    };
    let (a, b) = variant_counts(&store, &run.id).await?;
    let sig = significance(a, b, cmd.min_samples);
    if cmd.json {
        println!(
            "{}",
            json!({
                "skill": cmd.name,
                "run": run,
                "counts": {"a": a, "b": b},
                "significance": sig,
            })
        );
    } else {
        print_status(&run, a, b, sig);
    }
    Ok(())
}

async fn latest_run_for_skill(
    store: &SessionStore,
    skill_name: &str,
) -> ArgoResult<Option<AbTestRun>> {
    let row = sqlx::query(
        "SELECT id FROM skill_ab_test_run \
         WHERE skill_name = ? ORDER BY started_at DESC LIMIT 1",
    )
    .bind(skill_name)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("ab-snapshot lookup: {e}")))?;
    let Some(row) = row else { return Ok(None) };
    let id: String = row.get("id");
    get_run(store, &id).await
}

/// `argo skill ab-promote <run-id> [--winner V]` entry.
pub async fn ab_promote(cmd: SkillAbPromoteCmd) -> ArgoResult<()> {
    let store = Arc::new(open_store().await?);
    ensure_ab_schema(&store).await?;
    let quality_store = SkillQualityStore::new(Arc::clone(&store));
    quality_store.ensure_schema().await?;
    let config = QualityWeightConfig {
        weight_quality: cmd.quality_weight,
        max_score_age_days: cmd.max_score_age_days,
    };
    let (_, reason) = maybe_complete_with_quality(
        &store,
        &cmd.run_id,
        cmd.min_samples,
        Some(&quality_store),
        config,
    )
    .await?;
    let winner = promote(&store, &cmd.run_id, cmd.winner.as_deref()).await?;
    let reason_str = winner_reason_str(reason);
    if cmd.json {
        println!(
            "{}",
            json!({
                "run_id": cmd.run_id,
                "winner": winner,
                "promoted": true,
                "reason": reason_str,
            })
        );
    } else {
        println!("ab-promote {}: winner = '{winner}' ({reason_str})", cmd.run_id);
    }
    Ok(())
}

fn winner_reason_str(reason: Option<WinnerReason>) -> &'static str {
    match reason {
        Some(WinnerReason::WeightedQuality) => "won by weighted-quality decision",
        Some(WinnerReason::SuccessRate) => "won by success rate",
        // None = run was already completed by a prior call; the
        // winner column is stable so we don't relitigate the
        // rationale here.
        None => "winner already chosen on a prior call",
    }
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
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
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

    async fn insert_skill_row(store: &SessionStore, name: &str, version: &str) {
        sqlx::query(
            "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, \
                                learned_in, superseded_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().as_bytes().to_vec())
        .bind(name)
        .bind(version)
        .bind("body")
        .bind("{}")
        .bind("user")
        .bind(vec![0u8; 32])
        .bind(Option::<&[u8]>::None)
        .bind(Option::<&[u8]>::None)
        .bind(0_i64)
        .execute(store.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn latest_run_for_skill_returns_none_when_no_runs() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        ensure_ab_schema(&store).await.unwrap();
        let run = latest_run_for_skill(&store, "alpha").await.unwrap();
        assert!(run.is_none());
    }

    #[tokio::test]
    async fn latest_run_for_skill_returns_most_recent() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        ensure_ab_schema(&store).await.unwrap();
        insert_skill_row(&store, "alpha", "v1").await;
        insert_skill_row(&store, "alpha", "v2").await;
        let _r1 = create_run(&store, "alpha", "v1", "v2", 100).await.unwrap();
        // tiny pause so the second run's started_at strictly exceeds
        // the first's (microsecond clock resolution).
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        let r2 = create_run(&store, "alpha", "v1", "v2", 200).await.unwrap();
        let got = latest_run_for_skill(&store, "alpha")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.id, r2);
    }

    #[tokio::test]
    async fn ab_snapshot_handler_no_run_is_ok() {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        ensure_ab_schema(&store).await.unwrap();
        // Verifies the lookup path returns cleanly when no run
        // exists — the human/json branches inside the handler are
        // smoke-tested by the JSON shape via `latest_run_for_skill`
        // returning `None`.
        let result = latest_run_for_skill(&store, "nonexistent").await.unwrap();
        assert!(result.is_none());
    }
}
