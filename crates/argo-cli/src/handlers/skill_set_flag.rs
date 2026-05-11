//! `argo skill set-flag <name> --no-forge-feedback|--clear-no-forge-feedback`
//! (Stream `mission-forge-operator-feedback-eval-signal`,
//! MISSION_BACKLOG Section 3 Claim 4 #85).
//!
//! Thin wrapper over `argo_store::skill_flag::set_flag`. Today the
//! only consumer is the Forge ingestion path
//! (`argo-forge::feedback_eval::ingest_operator_feedback`); future
//! per-skill toggles will land additional flag args here.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::skill_flag::{set_flag, FLAG_NO_FORGE_FEEDBACK};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

/// Dispatch the `argo skill set-flag` CLI args.
pub async fn run(
    name: &str,
    no_forge_feedback: bool,
    clear_no_forge_feedback: bool,
    json: bool,
) -> ArgoResult<()> {
    if !no_forge_feedback && !clear_no_forge_feedback {
        return Err(ArgoError::Provider(
            "argo skill set-flag: pass --no-forge-feedback or \
             --clear-no-forge-feedback (no flags would be a no-op)"
                .to_string(),
        ));
    }
    // Clap's `conflicts_with` on the subcommand makes
    // (no_forge_feedback && clear_no_forge_feedback) unreachable;
    // we trust the argument parser and treat the clear-bit as
    // dominant just in case a future refactor relaxes the
    // conflicts_with attribute — set_flag is upsert-only, so
    // either ordering converges to value=0 in that hypothetical.
    let value: i64 = if clear_no_forge_feedback { 0 } else { 1 };

    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    set_flag(store.pool(), name, FLAG_NO_FORGE_FEEDBACK, value).await?;
    if json {
        let payload = serde_json::json!({
            "skill": name,
            "flag": FLAG_NO_FORGE_FEEDBACK,
            "value": value,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        let label = if value == 1 { "enabled" } else { "cleared" };
        println!("skill_flag {label}: {name} {FLAG_NO_FORGE_FEEDBACK} = {value}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_no_flags_passed() {
        // Without --no-forge-feedback or --clear-no-forge-feedback,
        // the call is a no-op and surfaces a clear error rather
        // than silently doing nothing.
        let err = run("alpha", false, false, false).await.unwrap_err();
        assert!(err.to_string().contains("set-flag"));
    }
}
