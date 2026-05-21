//! Stdin/stdout helpers for `argo workflow def ...` (Stream
//! `mission-pm-workflow-rollback-cli`).
//!
//! Kept in a sibling module so `workflow_def.rs` stays under the
//! per-file LOC ceiling. The `confirm_or_abort` prompt is gated by
//! `is_terminal` so unattended scripts that pipe stdin do not block
//! indefinitely waiting for `y/N`; in that case the prompt prints
//! and returns `false` (the default-deny answer to "yes proceed?").

use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::io::{IsTerminal, Write};

/// Print `value` as pretty-formatted JSON followed by a newline.
pub fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body = serde_json::to_string_pretty(value)
        .map_err(|e| ArgoError::Provider(format!("serialize workflow def output: {e}")))?;
    println!("{body}");
    Ok(())
}

/// Print a plain text message followed by a newline.
pub fn print_text(msg: &str) -> ArgoResult<()> {
    println!("{msg}");
    Ok(())
}

/// Interactive y/N prompt. Reads one line from stdin and returns
/// `true` only if the trimmed answer is `y` or `yes`
/// (case-insensitive). When stdin is not a TTY the prompt prints and
/// returns `false` immediately — non-interactive callers must pass
/// `--yes` to skip the prompt explicitly.
pub fn confirm_or_abort(prompt: &str) -> ArgoResult<bool> {
    print!("{prompt}");
    std::io::stdout()
        .flush()
        .map_err(|e| ArgoError::Io(format!("flush prompt: {e}")))?;
    if !std::io::stdin().is_terminal() {
        println!();
        return Ok(false);
    }
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|e| ArgoError::Io(format!("read prompt: {e}")))?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn print_text_returns_ok() {
        assert!(print_text("hello").is_ok());
    }

    #[test]
    fn print_json_returns_ok() {
        let body = json!({"ok": true});
        assert!(print_json(&body).is_ok());
    }
}
