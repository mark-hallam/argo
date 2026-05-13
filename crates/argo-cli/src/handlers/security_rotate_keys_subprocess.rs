//! Subprocess plumbing for `argo security rotate-keys commit`
//! (Mission `mission-rotation-cli-subsystem-wiring`).
//!
//! Split out from the parent handler so the 500-LOC ceiling applies
//! per file. The coordinator stays free of `std::process::Command`
//! state machinery; this module owns command construction, capture,
//! defensive redaction, and the dry-run / commit display helpers.
//!
//! No key material is logged. The raw `--new-key-source` string the
//! operator supplied is redacted out of captured stdout/stderr
//! before persisting, in case the subsystem CLI echoes it verbatim.

use crate::handlers::security_rotate_keys_registry::{ExecutionKind, KeyEntry};
use argo_types::{ArgoError, ArgoResult};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use uuid::Uuid;

pub const ROTATION_SUCCEEDED: &str = "succeeded";
pub const ROTATION_FAILED: &str = "failed";
pub const REDACTED_PLACEHOLDER: &str = "<redacted-key-source>";

/// Latest persisted attempt for one rotation_id; drives retry.
#[derive(Debug)]
pub struct PriorRotation {
    pub audit_id: Uuid,
    pub outcome: &'static str,
}

/// Captured output of one subsystem subprocess.
#[derive(Debug)]
pub struct SubprocessResult {
    pub command_display: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub succeeded: bool,
}

/// Build the `Command` for an entry's subprocess execution, with the
/// `argo` binary resolved from `current_exe()` (no `PATH` lookup, no
/// shell). `raw_new_key_source` is the operator-supplied
/// `--new-key-source` value forwarded to the child.
pub fn build_command(
    args: &'static [&'static str],
    new_key_flag: Option<&'static str>,
    raw_new_key_source: &str,
) -> ArgoResult<(Command, String)> {
    let bin: PathBuf = std::env::current_exe()
        .map_err(|e| ArgoError::Provider(format!("rotate-keys subprocess: current_exe: {e}")))?;
    let mut cmd = Command::new(&bin);
    cmd.args(args);
    let mut display: Vec<String> = vec![bin.display().to_string()];
    for a in args {
        display.push((*a).to_string());
    }
    if let Some(flag) = new_key_flag {
        cmd.arg(flag).arg(raw_new_key_source);
        display.push(flag.to_string());
        display.push(raw_new_key_source.to_string());
    }
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    Ok((cmd, display.join(" ")))
}

/// Spawn one command synchronously and capture its output. Split out
/// so unit tests can drive it with a synthetic `Command` (e.g.
/// `/bin/sh -c "..."`) without needing a real `argo` binary on disk.
pub fn spawn_and_capture(
    mut cmd: Command,
    command_display: String,
) -> ArgoResult<SubprocessResult> {
    let output = cmd
        .output()
        .map_err(|e| ArgoError::Provider(format!("rotate-keys subprocess spawn: {e}")))?;
    Ok(SubprocessResult {
        command_display,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        succeeded: output.status.success(),
    })
}

/// Run the rotation for one entry. `OperatorPrompt` returns `None`;
/// `Subprocess` spawns and returns the captured result with defensive
/// redaction of the raw `--new-key-source` applied to every stream.
pub fn run_execution(
    entry: &KeyEntry,
    raw_new_key_source: &str,
) -> ArgoResult<Option<SubprocessResult>> {
    match entry.execution {
        ExecutionKind::OperatorPrompt => Ok(None),
        ExecutionKind::Subprocess { args, new_key_flag } => {
            let (cmd, display) = build_command(args, new_key_flag, raw_new_key_source)?;
            let mut result = spawn_and_capture(cmd, display)?;
            redact_in_place(&mut result.stdout, raw_new_key_source);
            redact_in_place(&mut result.stderr, raw_new_key_source);
            redact_in_place(&mut result.command_display, raw_new_key_source);
            Ok(Some(result))
        }
    }
}

/// Scrub the raw `--new-key-source` argument out of a captured
/// stream. Skipped for `random` / empty inputs (nothing to redact).
pub fn redact_in_place(buf: &mut String, raw_new_key_source: &str) {
    let trimmed = raw_new_key_source.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("random") {
        return;
    }
    if buf.contains(trimmed) {
        *buf = buf.replace(trimmed, REDACTED_PLACEHOLDER);
    }
}

/// JSON envelope for the dry-run preview. Pure — no side effects,
/// no subprocess started. Subprocess entries get a fully resolved
/// (and redacted) command-line string under `command`;
/// operator-prompt entries get `command: null`.
pub fn dry_run_envelope(entry: &KeyEntry, raw_new_key_source: &str, redacted: &str) -> Value {
    let command_line = match entry.execution {
        ExecutionKind::Subprocess { args, new_key_flag } => {
            build_command(args, new_key_flag, raw_new_key_source)
                .ok()
                .map(|(_, mut display)| {
                    redact_in_place(&mut display, raw_new_key_source);
                    display
                })
        }
        ExecutionKind::OperatorPrompt => None,
    };
    json!({
        "key": entry.name,
        "status": "dry_run",
        "execution": entry.execution,
        "command": command_line,
        "new_key_source": redacted,
        "operator_followups": entry.operator_followups,
    })
}

/// Print the dry-run preview for one commit invocation. Subprocess
/// entries print the resolved command line (after redaction);
/// operator-prompt entries print the follow-up checklist.
pub fn print_dry_run(entry: &KeyEntry, raw_new_key_source: &str, redacted: &str, json: bool) {
    let envelope = dry_run_envelope(entry, raw_new_key_source, redacted);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&envelope).unwrap_or_default()
        );
        return;
    }
    println!("# Dry run: {} ({})", entry.name, entry.category);
    match envelope["command"].as_str() {
        Some(cmd) => {
            println!("Would spawn: `{cmd}`");
            println!("(`--dry-run`: no subprocess started, no audit row written.)");
        }
        None => {
            println!("No subprocess to spawn — operator follow-ups only:");
            for step in entry.operator_followups {
                println!("  - {step}");
            }
        }
    }
}

/// JSON envelope returned by `commit` after the rotation (or noop /
/// failure) has been applied.
pub fn commit_envelope(
    entry: &KeyEntry,
    redacted_source: &str,
    status: &str,
    audit_uuid: Uuid,
    rotation_id: &str,
    sub: Option<&SubprocessResult>,
) -> Value {
    let subprocess_envelope = sub.map(|r| {
        json!({
            "command": r.command_display,
            "exit_code": r.exit_code,
            "stdout": r.stdout,
            "stderr": r.stderr,
            "succeeded": r.succeeded,
        })
    });
    json!({
        "key": entry.name,
        "status": status,
        "audit_id": audit_uuid.to_string(),
        "rotation_id": rotation_id,
        "new_key_source": redacted_source,
        "rotation_command": entry.rotation_command,
        "execution": entry.execution,
        "subprocess": subprocess_envelope,
        "operator_followups": entry.operator_followups,
    })
}

/// Human-readable output for one commit invocation. The `status`
/// string is `"noop" | "failed" | "rotated"`.
pub fn print_commit_human(
    entry: &KeyEntry,
    redacted_source: &str,
    status: &str,
    audit_uuid: Uuid,
    rotation_id: &str,
    sub: Option<&SubprocessResult>,
) {
    match status {
        "noop" => println!(
            "{}: already succeeded for source `{}` today (rotation_id {}); no audit row written.",
            entry.name, redacted_source, rotation_id
        ),
        "failed" => print_failed(entry, redacted_source, audit_uuid, sub),
        _ => print_rotated(entry, redacted_source, audit_uuid, sub),
    }
}

fn print_failed(
    entry: &KeyEntry,
    redacted_source: &str,
    audit_uuid: Uuid,
    sub: Option<&SubprocessResult>,
) {
    println!(
        "{}: subsystem command FAILED (audit_id {}, source `{}`).",
        entry.name, audit_uuid, redacted_source
    );
    if let Some(r) = sub {
        if let Some(code) = r.exit_code {
            println!("  exit code: {code}");
        }
        if !r.stderr.trim().is_empty() {
            println!("  stderr: {}", r.stderr.trim());
        }
        println!(
            "Re-run `argo security rotate-keys commit {}` to retry.",
            entry.name
        );
    }
}

fn print_rotated(
    entry: &KeyEntry,
    redacted_source: &str,
    audit_uuid: Uuid,
    sub: Option<&SubprocessResult>,
) {
    println!(
        "{}: rotation recorded (audit_id {}, source `{}`).",
        entry.name, audit_uuid, redacted_source
    );
    if let Some(r) = sub {
        if let Some(code) = r.exit_code {
            println!(
                "  subsystem `{}` succeeded (exit {code}).",
                r.command_display
            );
        }
    } else if !entry.rotation_command.is_empty() {
        println!(
            "Run the subsystem command to apply: `{}`",
            entry.rotation_command
        );
    }
    for step in entry.operator_followups {
        println!("  • {step}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_in_place_replaces_raw_source() {
        let mut buf = "new key_source: env:SECRET_FOO\n".to_string();
        redact_in_place(&mut buf, "env:SECRET_FOO");
        assert!(buf.contains(REDACTED_PLACEHOLDER));
        assert!(!buf.contains("SECRET_FOO"));
    }

    #[test]
    fn redact_in_place_is_noop_for_random_or_empty() {
        let original = "ok\nstate: rotated\n".to_string();
        let mut buf = original.clone();
        redact_in_place(&mut buf, "random");
        assert_eq!(buf, original);
        redact_in_place(&mut buf, "");
        assert_eq!(buf, original);
    }

    #[test]
    fn spawn_and_capture_records_stdout_stderr_and_exit() {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg("echo hello; echo bye 1>&2; exit 0");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let r = spawn_and_capture(cmd, "/bin/sh -c '…'".to_string()).expect("spawn ok");
        assert!(r.succeeded);
        assert_eq!(r.exit_code, Some(0));
        assert!(r.stdout.contains("hello"));
        assert!(r.stderr.contains("bye"));
    }

    #[test]
    fn spawn_and_capture_surfaces_nonzero_exit() {
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("-c").arg("echo oops 1>&2; exit 7");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let r = spawn_and_capture(cmd, "/bin/sh -c '…'".to_string()).expect("spawn ok");
        assert!(!r.succeeded);
        assert_eq!(r.exit_code, Some(7));
        assert!(r.stderr.contains("oops"));
    }
}
