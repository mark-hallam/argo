//! `argo session` — list / diff / bisect / replay (offline ops
//! only; the live `replay` against a different model lives with
//! the daemon binding).
use crate::cli::SessionCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(cmd: SessionCmd) -> ArgoResult<()> {
    match cmd {
        SessionCmd::List { limit } => list_sessions(limit).await,
        SessionCmd::Diff { id1, id2 } => {
            // The diff against the live SQLite store needs the
            // tool_call → TurnRecord projection, which lands with
            // the daemon's session-replay-source binding (separate
            // PR). Without it, we surface a clear "needs daemon"
            // message rather than a half-working diff.
            println!(
                "session diff {id1} {id2}: requires the daemon's session-source binding (separate PR)."
            );
            Err(ArgoError::Provider(
                "session diff requires the daemon binding".into(),
            ))
        }
        SessionCmd::Bisect { id, hypothesis } => {
            println!(
                "session bisect {id} '{hypothesis}': same as diff — needs the daemon binding."
            );
            Err(ArgoError::Provider(
                "session bisect requires the daemon binding".into(),
            ))
        }
        SessionCmd::Replay {
            id,
            model,
            tools,
            workspace,
        } => {
            println!("session replay {id}");
            if let Some(m) = model {
                println!("  --model {m}");
            }
            if let Some(t) = tools {
                println!("  --tools {t}");
            }
            if let Some(w) = workspace {
                println!("  --workspace {w}");
            }
            println!(
                "  (verbatim replay against the recorded provider response is the daemon's job;"
            );
            println!("   the argo-replay library is built and tested at the lib tier.)");
            Ok(())
        }
    }
}

async fn list_sessions(limit: u32) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let sessions = store.list_all_sessions(limit).await?;
    if sessions.is_empty() {
        println!("(no sessions in {})", db.display());
        return Ok(());
    }
    println!(
        "{sid:<38} {prov:<12} {model:<24} title",
        sid = "session_id",
        prov = "provider",
        model = "model"
    );
    for s in sessions {
        println!(
            "{:<38} {:<12} {:<24} {}",
            s.id.0,
            short(&s.provider, 12),
            short(&s.model, 24),
            s.title.as_deref().unwrap_or("")
        );
    }
    Ok(())
}

fn short(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
