//! Front-window PNG capture via the macOS `screencapture` CLI.
//!
//! Why subprocess vs `core-graphics`: the V1.5 PoC keeps the
//! crate dep list minimal (no macOS-only Rust crates) so the
//! workspace `cargo check` stays green on every host. The
//! subprocess path uses Apple's own `screencapture` binary
//! (since 10.6) which honours the same Screen Recording
//! permission as `CGWindowListCreateImage` — equivalent
//! security, simpler glue.

use crate::error::{truncate_stderr, MacosCuError};
use crate::MacosCuResult;
use std::path::Path;

/// Capture the foreground window of the current display to a PNG
/// file at `dest`. The host must have Screen Recording permission
/// granted.
///
/// Subprocess invocation:
/// `screencapture -t png -x -o -W <dest>`
///
/// - `-t png` — PNG format (default; explicit for clarity)
/// - `-x` — no shutter sound
/// - `-o` — no window shadow
/// - `-W` — interactive window selection — replaced by `-l` +
///   front-window-id when wired through the executor; for the
///   PoC we let the operator handle target selection by ensuring
///   the target app is already activated by `crate::a11y::activate_app`
pub fn capture_front_window(dest: &Path) -> MacosCuResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    capture_front_window_impl(dest)
}

#[cfg(target_os = "macos")]
fn capture_front_window_impl(dest: &Path) -> MacosCuResult<()> {
    use std::process::Command;
    let output = Command::new("screencapture")
        .args(["-t", "png", "-x", "-o", "-l"])
        .arg(front_window_id_arg()?)
        .arg(dest)
        .output()?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "screencapture".to_string(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn capture_front_window_impl(_dest: &Path) -> MacosCuResult<()> {
    Err(MacosCuError::NotMacos)
}

/// Discover the front window's window-id via Quartz, formatted
/// as the literal `<n>` `screencapture -l` expects.
#[cfg(target_os = "macos")]
fn front_window_id_arg() -> MacosCuResult<String> {
    use std::process::Command;
    // osascript trick: System Events exposes the front window's
    // AXIdentifier-equivalent via process. The `id` of the
    // first window of the frontmost process is a stable
    // identifier we can pass to screencapture.
    let out = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to tell (first process whose frontmost is true) to get id of front window",
        ])
        .output()?;
    if !out.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "osascript[front_window_id]".to_string(),
            stderr: truncate_stderr(&out.stderr),
        });
    }
    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if raw.is_empty() {
        return Err(MacosCuError::Protocol(
            "front window id was empty (no app focused?)".to_string(),
        ));
    }
    Ok(raw)
}

/// Read the captured PNG back as bytes (for the Anthropic CU
/// payload). Caller is responsible for path management.
pub fn read_png(path: &Path) -> MacosCuResult<Vec<u8>> {
    Ok(std::fs::read(path)?)
}

/// Capture the screen size in pixels — needed by Anthropic's CU
/// tool spec so click coordinates map onto the right space.
/// Returns `(width, height)`.
pub fn screen_size() -> MacosCuResult<(u32, u32)> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    screen_size_impl()
}

#[cfg(target_os = "macos")]
fn screen_size_impl() -> MacosCuResult<(u32, u32)> {
    use std::process::Command;
    // `system_profiler SPDisplaysDataType` is verbose; a tighter
    // path is `osascript` against System Events.
    let out = Command::new("osascript")
        .args([
            "-e",
            "tell application \"Finder\" to get bounds of window of desktop",
        ])
        .output()?;
    if !out.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "osascript[screen_size]".to_string(),
            stderr: truncate_stderr(&out.stderr),
        });
    }
    parse_bounds(&String::from_utf8_lossy(&out.stdout))
}

#[cfg(not(target_os = "macos"))]
fn screen_size_impl() -> MacosCuResult<(u32, u32)> {
    Err(MacosCuError::NotMacos)
}

/// Parse the `0, 0, 1920, 1200` shape Finder returns into
/// `(width, height)`. The first two values are the origin (always
/// 0, 0 for the desktop window); the last two are width, height.
pub(crate) fn parse_bounds(raw: &str) -> MacosCuResult<(u32, u32)> {
    let parts: Vec<&str> = raw.trim().split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(MacosCuError::Protocol(format!(
            "expected 4-tuple bounds from Finder, got `{raw}`"
        )));
    }
    let w: u32 = parts[2]
        .parse()
        .map_err(|e| MacosCuError::Protocol(format!("bad width `{}`: {e}", parts[2])))?;
    let h: u32 = parts[3]
        .parse()
        .map_err(|e| MacosCuError::Protocol(format!("bad height `{}`: {e}", parts[3])))?;
    Ok((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bounds_extracts_width_and_height() {
        let (w, h) = parse_bounds("0, 0, 1920, 1200").unwrap();
        assert_eq!(w, 1920);
        assert_eq!(h, 1200);
    }

    #[test]
    fn parse_bounds_handles_no_spaces() {
        let (w, h) = parse_bounds("0,0,2560,1440").unwrap();
        assert_eq!(w, 2560);
        assert_eq!(h, 1440);
    }

    #[test]
    fn parse_bounds_rejects_short_tuple() {
        let err = parse_bounds("0, 0, 1920").unwrap_err();
        assert!(matches!(err, MacosCuError::Protocol(_)));
    }

    #[test]
    fn parse_bounds_rejects_non_numeric() {
        let err = parse_bounds("0, 0, wide, tall").unwrap_err();
        assert!(matches!(err, MacosCuError::Protocol(_)));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn capture_returns_not_macos_off_platform() {
        let path = std::path::Path::new("/tmp/argo-cu-macos-shouldnt-exist.png");
        let err = capture_front_window(path).unwrap_err();
        assert!(matches!(err, MacosCuError::NotMacos));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn screen_size_returns_not_macos_off_platform() {
        let err = screen_size().unwrap_err();
        assert!(matches!(err, MacosCuError::NotMacos));
    }
}
