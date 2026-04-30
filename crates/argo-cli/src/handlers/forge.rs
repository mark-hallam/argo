//! `argo forge` — runs list (read-only). Optimize / promote /
//! bench live with the daemon binding (separate PR — they need a
//! live provider + judge).
use crate::cli::{ForgeCmd, ForgeRunsCmd};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(cmd: ForgeCmd) -> ArgoResult<()> {
    match cmd {
        ForgeCmd::Runs { cmd } => {
            match cmd {
                ForgeRunsCmd::List => list_runs().await,
                ForgeRunsCmd::Show { id } => {
                    println!("argo forge runs show {id}: requires the daemon's forge binding (separate PR).");
                    Ok(())
                }
                ForgeRunsCmd::Abort { id } => {
                    println!("argo forge runs abort {id}: requires the daemon's forge binding.");
                    Ok(())
                }
            }
        }
        other => {
            println!("argo forge: '{other:?}' requires the daemon's forge worker (separate PR).");
            Ok(())
        }
    }
}

async fn list_runs() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let rows = store.list_forge_runs(50).await?;
    if rows.is_empty() {
        println!("(no forge runs recorded yet)");
        return Ok(());
    }
    let header = "skill                    base       candidate  iters  spend USD  score   status";
    println!("{header}");
    for r in rows {
        let cand = r.candidate_version.unwrap_or_else(|| "-".into());
        let score = r
            .final_score
            .map(|s| format!("{s:.3}"))
            .unwrap_or_else(|| "-".into());
        let spend = format!("${:.2}", r.spend_usd_micros as f64 / 1_000_000.0);
        println!(
            "{name:<24} {base:<10} {cand:<10} {iters:<6} {spend:<10} {score:<7} {status}",
            name = r.skill_name,
            base = r.base_version,
            cand = cand,
            iters = r.iterations,
            spend = spend,
            score = score,
            status = r.status
        );
    }
    Ok(())
}
