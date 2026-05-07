//! macOS Notes-specific helpers.
//!
//! Used by the smoke test + by any caller that wants to drive
//! Notes through the V1.5 PoC. Wraps the AppleScript surface
//! Notes exposes:
//!
//! - `tell application "Notes" to make new note with properties {body: "..."}`
//! - `get body of every note` (readback for verification)
//! - `delete every note whose body contains "..."` (test cleanup)
//!
//! Anything outside Notes is out of scope here — broader
//! per-app helper modules ship in follow-up briefs as new
//! verticals adopt the native macOS executor.

use crate::error::{truncate_stderr, MacosCuError};
use crate::MacosCuResult;

/// Bundle id Notes always reports under. Stable since 10.8.
pub const NOTES_BUNDLE_ID: &str = "com.apple.Notes";

/// Open Notes (creates a fresh window if none) and bring it to
/// front. Idempotent — safe to call multiple times per session.
pub async fn ensure_notes_open() -> MacosCuResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    run_script("tell application id \"com.apple.Notes\" to activate").await
}

/// Create a new note with the given body. Returns the note's id
/// for follow-up readback.
pub async fn create_note(body: &str) -> MacosCuResult<String> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    let escaped = crate::events::escape_applescript(body);
    let script = format!(
        "tell application id \"com.apple.Notes\" to (id of (make new note with properties {{body:\"{escaped}\"}})) as string"
    );
    let out = run_script_capturing(&script).await?;
    Ok(out.trim().to_string())
}

/// Read the body of the note with the given id. Used by the
/// smoke test to verify the CU loop's typed text actually landed
/// in Notes.
pub async fn body_of(note_id: &str) -> MacosCuResult<String> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    let escaped = crate::events::escape_applescript(note_id);
    let script = format!(
        "tell application id \"com.apple.Notes\" to (body of note id \"{escaped}\") as string"
    );
    let out = run_script_capturing(&script).await?;
    Ok(out.trim().to_string())
}

/// Test cleanup: delete the note with the given id if it exists.
/// Doesn't error if the note has already been deleted (script's
/// `try ... end try` swallows the missing-note error).
pub async fn delete_note(note_id: &str) -> MacosCuResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    let escaped = crate::events::escape_applescript(note_id);
    let script = format!(
        "tell application id \"com.apple.Notes\"\n\ttry\n\t\tdelete note id \"{escaped}\"\n\tend try\nend tell"
    );
    run_script(&script).await
}

#[cfg(target_os = "macos")]
async fn run_script(script: &str) -> MacosCuResult<()> {
    let output = tokio::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .await?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "osascript[notes]".to_string(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    Ok(())
}

#[cfg(target_os = "macos")]
async fn run_script_capturing(script: &str) -> MacosCuResult<String> {
    let output = tokio::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .await?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "osascript[notes]".to_string(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(not(target_os = "macos"))]
async fn run_script(_script: &str) -> MacosCuResult<()> {
    Err(MacosCuError::NotMacos)
}

#[cfg(not(target_os = "macos"))]
async fn run_script_capturing(_script: &str) -> MacosCuResult<String> {
    Err(MacosCuError::NotMacos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_id_is_constant() {
        assert_eq!(NOTES_BUNDLE_ID, "com.apple.Notes");
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn ensure_notes_open_returns_not_macos_off_platform() {
        let err = ensure_notes_open().await.unwrap_err();
        assert!(matches!(err, MacosCuError::NotMacos));
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn create_note_returns_not_macos_off_platform() {
        let err = create_note("body").await.unwrap_err();
        assert!(matches!(err, MacosCuError::NotMacos));
    }
}
