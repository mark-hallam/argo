//! `argo chat <id> [--user <text>]` — CLI entry that drives the
//! shared multi-turn agent loop in [`argo_agent_runner`].
//!
//! The agent loop itself (provider call, tool dispatch, audit
//! emission, spend recording) lives in `argo-agent-runner`; this
//! module stays thin: resolve the session id, optionally launch
//! interactive REPL, and pretty-print the [`TurnResult`].
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use std::path::Path;
use std::sync::Arc;

// Re-export the runner's public surface so callers that previously
// reached for `crate::handlers::chat::*` keep working without
// touching the new crate path. Gateway streaming + replay/stream
// import these directly.
pub use argo_agent_runner::{run_turn, run_turn_with_events, TurnResult, MAX_TOOL_ROUNDS};

pub async fn run(
    raw: &str,
    user: Option<&str>,
    stream: bool,
    interactive: bool,
    max_tool_rounds: Option<usize>,
) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    let id = super::session_resolve::resolve(&store, raw).await?.0;

    let rounds = max_tool_rounds.unwrap_or(MAX_TOOL_ROUNDS);
    if interactive {
        return run_interactive(&store, &cwd, SessionId(id), user, stream, rounds).await;
    }

    eprintln!("argo chat: → session {id} (stream={stream})");
    let result = run_turn(&store, &cwd, SessionId(id), user, stream, rounds).await?;
    println!("\nargo chat: ← assistant message {}", result.assistant_id);
    if !stream && !result.text.is_empty() {
        println!("\n{}\n", result.text);
    }
    if let (Some(ti), Some(to)) = (result.tokens_in, result.tokens_out) {
        println!("[{ti} prompt + {to} completion tokens]");
    }
    Ok(())
}

async fn run_interactive(
    store: &Arc<SessionStore>,
    cwd: &Path,
    session_id: SessionId,
    seed: Option<&str>,
    stream: bool,
    max_rounds: usize,
) -> ArgoResult<()> {
    use std::io::BufRead;
    eprintln!(
        "argo chat: interactive mode against session {} (stream={stream})",
        session_id.0
    );
    eprintln!("type a line to send, Ctrl-D / EOF to exit\n");

    // First turn uses the optional --user seed (if any) so callers
    // can prime the REPL from the command line.
    if let Some(text) = seed {
        eprintln!("> {text}");
        let result = run_turn(store, cwd, session_id, Some(text), stream, max_rounds).await?;
        print_result(&result, stream);
    }

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    loop {
        eprint!("> ");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let line = match lines.next() {
            Some(Ok(l)) => l,
            Some(Err(e)) => return Err(ArgoError::Provider(format!("stdin: {e}"))),
            None => {
                eprintln!("\nargo chat: EOF — bye");
                return Ok(());
            }
        };
        if line.is_empty() {
            continue;
        }
        let result = run_turn(store, cwd, session_id, Some(&line), stream, max_rounds).await?;
        print_result(&result, stream);
    }
}

fn print_result(result: &TurnResult, stream: bool) {
    println!("\nargo chat: ← assistant message {}", result.assistant_id);
    if !stream && !result.text.is_empty() {
        println!("\n{}\n", result.text);
    }
    if let (Some(ti), Some(to)) = (result.tokens_in, result.tokens_out) {
        println!("[{ti} prompt + {to} completion tokens]");
    }
}
