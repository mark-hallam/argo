//! `argo forge daemon` — standalone worker that periodically
//! sweeps stalled `running` forge_run rows to `failed`. First
//! step in the v1.1 forge-daemon split: replaces the manual
//! `argo forge runs sweep` cadence with a long-running worker
//! that survives gateway restarts.
//!
//! Architecture (current PR — sweep-only):
//! ```text
//!   ┌─────────────┐                ┌─────────────┐
//!   │  gateway    │ ─enqueue─────▶ │  forge_run  │
//!   │  (in-proc)  │ ◀──poll/sweep─ │  table      │
//!   └─────────────┘                └─────────────┘
//!                                         ▲
//!                                  poll/sweep
//!                                         │
//!                                  ┌──────────────┐
//!                                  │ forge daemon │
//!                                  │  (separate)  │
//!                                  └──────────────┘
//! ```
//! Future iterations will:
//! 1. Add a `queued` status to the forge_run lifecycle
//! 2. Make the gateway's POST /forge/optimize enqueue-only
//! 3. Have the daemon claim queued rows + run them
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::time::Duration;

pub async fn run(
    interval_secs: u64,
    older_than_minutes: i64,
    max_ticks: Option<u64>,
) -> ArgoResult<()> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    println!(
        "argo forge daemon: tick every {interval_secs}s, sweep stalled `running` rows older than {older_than_minutes}m, claim+run queued rows"
    );
    let mut ticks = 0_u64;
    loop {
        let cutoff_micros =
            (chrono::Utc::now() - chrono::Duration::minutes(older_than_minutes))
                .timestamp_micros();
        match store.sweep_stalled_forge_runs(cutoff_micros).await {
            Ok(0) => {}
            Ok(n) => {
                println!(
                    "tick {ticks}: swept {n} stalled run(s) older than {older_than_minutes}m"
                );
            }
            Err(e) => {
                eprintln!("tick {ticks}: sweep error: {e}");
            }
        }
        // Claim + run the next queued row (synchronous within
        // the tick — first-cut keeps races simple, future PRs
        // can spawn). One row per tick; the next tick picks up
        // the next queued row.
        match store.claim_queued_forge_run().await {
            Ok(Some((run_id, payload))) => {
                run_claimed_row(&store, run_id, &payload, ticks).await;
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("tick {ticks}: claim error: {e}");
            }
        }
        ticks += 1;
        if max_ticks.is_some_and(|cap| ticks >= cap) {
            println!("daemon: reached --max-ticks {ticks}; exiting");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}

/// Drive a claimed forge_run row through `forge_optimize::run_claimed`.
/// On error, mark the row failed so it doesn't sit in 'running'
/// indefinitely. The payload was serialized at enqueue time and
/// has the shape:
/// `{"skill": …, "examples": …, "iterations": N, "budget_usd": X}`.
async fn run_claimed_row(
    store: &SessionStore,
    run_id: uuid::Uuid,
    payload: &str,
    tick: u64,
) {
    let parsed: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "tick {tick}: claim {run_id}: payload parse failed: {e}; failing run"
            );
            let _ = store.finish_forge_run(run_id, "failed", None, 0).await;
            return;
        }
    };
    let skill = parsed
        .get("skill")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let examples = parsed
        .get("examples")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let iterations =
        parsed.get("iterations").and_then(|v| v.as_u64()).unwrap_or(12) as u32;
    let budget_usd = parsed.get("budget_usd").and_then(|v| v.as_f64()).unwrap_or(5.0);
    if skill.is_empty() || examples.is_empty() {
        eprintln!(
            "tick {tick}: claim {run_id}: payload missing skill or examples; failing run"
        );
        let _ = store.finish_forge_run(run_id, "failed", None, 0).await;
        return;
    }
    println!(
        "tick {tick}: claimed run {run_id} (skill={skill}, iterations={iterations}, budget_usd={budget_usd:.4}); running…"
    );
    match super::forge_optimize::run_claimed(&skill, &examples, iterations, budget_usd, run_id)
        .await
    {
        Ok(()) => {
            println!("tick {tick}: run {run_id} finished");
        }
        Err(e) => {
            eprintln!("tick {tick}: run {run_id} failed: {e}");
            let _ = store.finish_forge_run(run_id, "failed", None, 0).await;
        }
    }
}
