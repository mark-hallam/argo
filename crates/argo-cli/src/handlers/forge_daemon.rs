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
        "argo forge daemon: tick every {interval_secs}s, sweep stalled `running` rows older than {older_than_minutes}m"
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
        ticks += 1;
        if max_ticks.is_some_and(|cap| ticks >= cap) {
            println!("daemon: reached --max-ticks {ticks}; exiting");
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(interval_secs)).await;
    }
}
