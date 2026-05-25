//! Front-window PNG capture via the macOS `screencapture` CLI.
//!
//! Why subprocess vs `core-graphics`: the V1.5 PoC keeps the
//! crate dep list minimal (no macOS-only Rust crates) so the
//! workspace `cargo check` stays green on every host. The
//! subprocess path uses Apple's own `screencapture` binary
//! (since 10.6) which honours the same Screen Recording
//! permission as `CGWindowListCreateImage` — equivalent
//! security, simpler glue.

use crate::config::ScreenshotCompression;
use crate::error::{truncate_stderr, MacosCuError};
use crate::MacosCuResult;
use std::ffi::OsString;
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

/// Build the `sips` argv that applies `compression` to the file at
/// `src` in place. Returns `None` for the pass-through default
/// ([`ScreenshotCompression::Png`] with `max_edge: None`), signaling
/// "no subprocess to run."
///
/// Pure function — does not touch the filesystem or spawn a process,
/// so it can be unit-tested off-macOS. `apply_compression` is the
/// thin runner that actually invokes `sips`.
///
/// `sips` flags used:
/// - `-s format jpeg` — re-encode to JPEG.
/// - `-s formatOptions <q>` — JPEG quality (0..=100; we restrict to
///   1..=100 in [`ScreenshotCompression::validate`]).
/// - `-Z <edge>` — resample so the longest edge becomes `edge` px,
///   preserving aspect ratio.
///
/// The output file is the same path as `src`; `sips` rewrites in
/// place. The extension stays whatever the caller picked
/// (typically `.png`) even when the bytes become JPEG — see
/// [`ScreenshotCompression::extension`] for the truthful name when
/// callers want it.
pub(crate) fn build_sips_argv(
    compression: ScreenshotCompression,
    src: &Path,
) -> Option<Vec<OsString>> {
    let mut args: Vec<OsString> = Vec::new();
    match compression {
        ScreenshotCompression::Png { max_edge: None } => return None,
        ScreenshotCompression::Png {
            max_edge: Some(edge),
        } => {
            args.push("-Z".into());
            args.push(edge.to_string().into());
        }
        ScreenshotCompression::Jpeg { max_edge, quality } => {
            args.push("-s".into());
            args.push("format".into());
            args.push("jpeg".into());
            args.push("-s".into());
            args.push("formatOptions".into());
            args.push(quality.to_string().into());
            if let Some(edge) = max_edge {
                args.push("-Z".into());
                args.push(edge.to_string().into());
            }
        }
    }
    args.push(src.as_os_str().to_os_string());
    Some(args)
}

/// Apply `compression` to the file at `src` in place, via `sips`.
/// No-op (returns `Ok(())`) for the pass-through default.
///
/// macOS-only — returns [`MacosCuError::NotMacos`] off-platform if
/// the caller asked for a real transform. The pass-through default
/// short-circuits before the platform check so off-macOS unit tests
/// can call this freely.
pub fn apply_compression(src: &Path, compression: ScreenshotCompression) -> MacosCuResult<()> {
    let Some(args) = build_sips_argv(compression, src) else {
        return Ok(());
    };
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    apply_compression_impl(&args)
}

#[cfg(target_os = "macos")]
fn apply_compression_impl(args: &[OsString]) -> MacosCuResult<()> {
    use std::process::Command;
    let output = Command::new("sips").args(args).output()?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "sips".to_string(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply_compression_impl(_args: &[OsString]) -> MacosCuResult<()> {
    Err(MacosCuError::NotMacos)
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

    fn argv_strings(argv: &Option<Vec<OsString>>) -> Vec<String> {
        argv.as_ref()
            .map(|v| v.iter().map(|s| s.to_string_lossy().into_owned()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn pass_through_default_emits_no_argv() {
        let argv = build_sips_argv(
            ScreenshotCompression::Png { max_edge: None },
            Path::new("/tmp/cap.png"),
        );
        assert!(argv.is_none());
    }

    #[test]
    fn png_downscale_emits_resample_argv_only() {
        let argv = build_sips_argv(
            ScreenshotCompression::Png {
                max_edge: Some(1280),
            },
            Path::new("/tmp/cap.png"),
        );
        let argv = argv.expect("downscale must emit argv");
        let strs: Vec<String> = argv.iter().map(|s| s.to_string_lossy().into_owned()).collect();
        assert_eq!(
            strs,
            vec!["-Z", "1280", "/tmp/cap.png"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn jpeg_without_max_edge_emits_format_argv_only() {
        let argv = build_sips_argv(
            ScreenshotCompression::Jpeg {
                max_edge: None,
                quality: 80,
            },
            Path::new("/tmp/cap.png"),
        );
        let strs = argv_strings(&argv);
        assert_eq!(strs[0], "-s");
        assert_eq!(strs[1], "format");
        assert_eq!(strs[2], "jpeg");
        assert_eq!(strs[3], "-s");
        assert_eq!(strs[4], "formatOptions");
        assert_eq!(strs[5], "80");
        assert_eq!(strs[6], "/tmp/cap.png");
        assert_eq!(strs.len(), 7);
    }

    #[test]
    fn jpeg_with_max_edge_emits_both_in_canonical_order() {
        let argv = build_sips_argv(
            ScreenshotCompression::Jpeg {
                max_edge: Some(1600),
                quality: 70,
            },
            Path::new("/tmp/cap.png"),
        );
        let strs = argv_strings(&argv);
        assert_eq!(strs[0], "-s");
        assert_eq!(strs[1], "format");
        assert_eq!(strs[2], "jpeg");
        assert_eq!(strs[3], "-s");
        assert_eq!(strs[4], "formatOptions");
        assert_eq!(strs[5], "70");
        assert_eq!(strs[6], "-Z");
        assert_eq!(strs[7], "1600");
        assert_eq!(strs[8], "/tmp/cap.png");
        assert_eq!(strs.len(), 9);
    }

    #[test]
    fn argv_preserves_path_with_spaces() {
        let path = Path::new("/tmp/argo cu/front window.png");
        let argv = build_sips_argv(
            ScreenshotCompression::Png {
                max_edge: Some(640),
            },
            path,
        );
        let argv = argv.expect("downscale must emit argv");
        let last = argv.last().unwrap();
        // The argv vector retains the raw OsString, so a quoted
        // path round-trips intact — the subprocess call below does
        // not go through a shell.
        assert_eq!(last.as_os_str(), path.as_os_str());
    }

    #[test]
    fn apply_compression_is_noop_for_default() {
        // Pass-through default short-circuits before the platform
        // check, so this must succeed even off-macOS and without a
        // real file on disk.
        let result = apply_compression(
            Path::new("/dev/null/does-not-exist.png"),
            ScreenshotCompression::default(),
        );
        assert!(result.is_ok());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn apply_compression_off_macos_returns_not_macos_when_transform_requested() {
        let err = apply_compression(
            Path::new("/tmp/cap.png"),
            ScreenshotCompression::Jpeg {
                max_edge: None,
                quality: 80,
            },
        )
        .unwrap_err();
        assert!(matches!(err, MacosCuError::NotMacos));
    }
}
