//! `argo forge enqueue <skill> --examples <path> [--iterations N --budget X]`
//! — enqueue a forge optimize run for the v1.1 daemon to
//! pick up later. Inserts a `forge_run` row with
//! status='queued' plus a parallel `forge_queue` payload row;
//! the daemon claims + runs the work on its next tick.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(
    skill: &str,
    examples: &str,
    iterations: u32,
    budget_usd: f64,
    json: bool,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    // Resolve current active skill version for `base_version`.
    // If the skill doesn't exist yet, fall back to "0.0.0" so
    // the row is still queryable; the daemon will fail-fast
    // when it tries to claim and finds no skill.
    let skills = argo_skill::SkillStore::new(std::sync::Arc::new(
        SessionStore::open(&db_path(&home)).await?,
    ));
    let base_version = match skills.find_active_by_name(skill).await {
        Ok(Some(s)) => s.frontmatter.version,
        _ => String::from("0.0.0"),
    };
    // Examples count: parse the file lazily — non-fatal if it
    // doesn't exist yet. The daemon will surface a clearer
    // error when it tries to actually run.
    let examples_count = std::fs::read_to_string(examples)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count() as u32)
        .unwrap_or(0);
    let payload = serde_json::json!({
        "skill": skill,
        "examples": examples,
        "iterations": iterations,
        "budget_usd": budget_usd,
    });
    let payload_str = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Provider(format!("serialize payload: {e}")))?;
    let id = store
        .enqueue_forge_run(skill, &base_version, examples_count, &payload_str)
        .await?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "enqueued": true,
                "run_id": id.to_string(),
                "skill": skill,
                "examples": examples,
                "iterations": iterations,
                "budget_usd": budget_usd,
            }))
            .unwrap_or_default()
        );
    } else {
        println!(
            "enqueued forge run {id} (skill={skill}, iterations={iterations}, budget_usd={budget_usd:.4}); daemon will pick it up on next tick"
        );
    }
    Ok(())
}
