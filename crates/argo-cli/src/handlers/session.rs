//! `argo session` — diff / bisect / replay (offline ops only;
//! the live `replay` against a different model lives with the
//! daemon binding).
use crate::cli::SessionCmd;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(cmd: SessionCmd) -> ArgoResult<()> {
    match cmd {
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
