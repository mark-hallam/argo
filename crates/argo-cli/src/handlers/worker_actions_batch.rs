//! Shared helpers for `argo worker actions approve-batch` —
//! decision selection, dry-run listing, the operator
//! confirmation prompt, and stable summary printing
//! (Stream `claude-approval-batch-decisions-cli`).
//!
//! Lives in its own module so `worker_actions_dispatch.rs`
//! stays under the per-file 500-LOC ceiling.

use super::worker_actions::ActionRecord;
use super::worker_actions_transitions;
use argo_memory::MemoryStore;
use argo_types::{ArgoError, ArgoResult, WorkspaceId};
use std::io::{BufRead, IsTerminal, Write};

/// Operator-supplied decision applied uniformly across the id
/// list. `Approve` lifts each pending action to `approved`;
/// `Deny` cancels it (`pending → cancelled`) — both reuse the
/// existing single-id transition functions in
/// `worker_actions_transitions`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchDecision {
    Approve,
    Deny,
}

impl BatchDecision {
    pub(crate) fn from_flag(deny: bool) -> Self {
        if deny { Self::Deny } else { Self::Approve }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Deny => "deny",
        }
    }
}

/// Print the dry-run preview: every id paired with the decision
/// that *would* be applied, then exit cleanly. Writes nothing to
/// the ledger.
pub(crate) fn print_dry_run(
    action_ids: &[String],
    decision: BatchDecision,
    json: bool,
) -> ArgoResult<()> {
    if json {
        let payload = serde_json::json!({
            "dry_run": true,
            "decision": decision.label(),
            "action_ids": action_ids,
            "count": action_ids.len(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!(
            "[dry-run] would {} {} action(s):",
            decision.label(),
            action_ids.len()
        );
        for id in action_ids {
            println!("  {id}");
        }
    }
    Ok(())
}

/// Ask the operator once before writing. Returns `Ok(true)` to
/// proceed, `Ok(false)` to abort. Refuses to silently proceed
/// when stdin is not a terminal — that path requires `--confirm`
/// so a non-interactive launcher can't accidentally apply a
/// batch decision the operator didn't intend.
pub(crate) fn prompt_confirms(count: usize, decision: BatchDecision) -> ArgoResult<bool> {
    if !std::io::stdin().is_terminal() {
        return Err(ArgoError::Provider(
            "approve-batch refuses to prompt on a non-interactive stdin; \
             pass --confirm (or -y) to apply non-interactively, or \
             --dry-run to preview"
                .into(),
        ));
    }
    print!(
        "Apply '{}' to {} pending approval(s)? [y/N] ",
        decision.label(),
        count
    );
    std::io::stdout()
        .flush()
        .map_err(|e| ArgoError::Provider(format!("flush stdout: {e}")))?;
    let mut buf = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut buf)
        .map_err(|e| ArgoError::Io(format!("read stdin: {e}")))?;
    Ok(matches!(buf.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

/// Stable summary table written after `approve_batch` runs.
/// Column order is fixed so a downstream `argo approval export`
/// can reuse the same row shape (id, decision, status, note).
/// JSON envelope groups successes and failures separately.
pub(crate) fn print_summary(
    ok: &[ActionRecord],
    failed: &[(String, String)],
    decision: BatchDecision,
    json: bool,
) {
    if json {
        let payload = serde_json::json!({
            "decision": decision.label(),
            "approved": ok,
            "failed": failed
                .iter()
                .map(|(id, e)| serde_json::json!({"action_id": id, "error": e}))
                .collect::<Vec<_>>(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    println!("id\tdecision\tstatus\tnote");
    for r in ok {
        println!(
            "{}\t{}\t{}\t",
            r.action_id,
            decision.label(),
            r.state,
        );
    }
    for (id, err) in failed {
        println!("{}\t{}\tfailed\t{}", id, decision.label(), err);
    }
}

/// Walk every supplied id, dispatching to the existing single-id
/// transition for the chosen decision. Each id is independent —
/// one failure does not abort the rest. Returns the successful
/// records and the (id, error-message) pairs separately so the
/// caller can render them however it likes.
pub(crate) async fn apply_decisions(
    memory: &MemoryStore,
    workspace: WorkspaceId,
    action_ids: &[String],
    actor: &str,
    note: Option<&str>,
    decision: BatchDecision,
) -> (Vec<ActionRecord>, Vec<(String, String)>) {
    let mut ok = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    for id in action_ids {
        let outcome = match decision {
            BatchDecision::Approve => {
                worker_actions_transitions::approve(memory, workspace, id, actor, note).await
            }
            BatchDecision::Deny => {
                worker_actions_transitions::cancel(memory, workspace, id, actor, note).await
            }
        };
        match outcome {
            Ok(rec) => ok.push(rec),
            Err(e) => failed.push((id.clone(), e.to_string())),
        }
    }
    (ok, failed)
}

#[cfg(test)]
#[path = "worker_actions_batch_tests.rs"]
mod tests;
