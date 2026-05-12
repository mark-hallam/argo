//! Wayland input path: `wtype` for typing/keys; portal RemoteDesktop
//! for mouse (currently fail-closed in v1 — see [`crate::wayland::portal`]).

use crate::error::truncate_stderr;
use crate::probe::{probe, DisplayMode, WaylandCapabilities};
use crate::wayland::portal;
use crate::x11::input::InputResult;
use crate::{LinuxCuError, LinuxCuResult};
use std::process::Command;

/// Move pointer to `(x, y)` and left-click via portal RemoteDesktop.
/// v1 fail-closed when the compositor lacks portal support: returns
/// a clear `Unsupported` error pointing operators at the supported
/// compositor matrix.
pub fn click(x: i32, y: i32, caps: &WaylandCapabilities) -> LinuxCuResult<InputResult> {
    if !caps.portal_reachable {
        return Err(LinuxCuError::Unsupported {
            reason: crate::wayland::probe::mouse_unsupported_hint(&caps.compositor),
        });
    }
    portal::click_via_portal(x, y, &caps.compositor)?;
    Ok(InputResult {
        action: "click".into(),
        detail: format!("{x},{y}"),
    })
}

/// Press one key via `wtype -k <keysym>`.
pub fn key(keysym: &str) -> LinuxCuResult<InputResult> {
    require_wayland()?;
    let normalized = normalize_key(keysym)?;
    run_wtype(&key_args(&normalized))?;
    Ok(InputResult {
        action: "key".into(),
        detail: normalized,
    })
}

/// Type literal text via `wtype <text>`.
pub fn type_text(text: &str) -> LinuxCuResult<InputResult> {
    require_wayland()?;
    run_wtype(&type_args(text))?;
    Ok(InputResult {
        action: "type".into(),
        detail: format!("{} chars", text.chars().count()),
    })
}

fn require_wayland() -> LinuxCuResult<()> {
    match probe().display_mode {
        DisplayMode::Wayland(_) => Ok(()),
        _ => Err(LinuxCuError::Unsupported {
            reason: "Wayland backend not active for input".into(),
        }),
    }
}

fn run_wtype(args: &[String]) -> LinuxCuResult<()> {
    let output = Command::new("wtype").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(LinuxCuError::Subprocess {
            command: format!("wtype {}", args.join(" ")),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

/// Argument vector for `wtype -k <keysym>`.
#[must_use]
pub fn key_args(keysym: &str) -> Vec<String> {
    vec!["-k".into(), keysym.into()]
}

/// Argument vector for `wtype -- <text>`. `--` terminates option
/// parsing so a leading `-` in user text is treated as literal.
#[must_use]
pub fn type_args(text: &str) -> Vec<String> {
    vec!["--".into(), text.into()]
}

/// Map common aliases to `wtype`'s keysym names. wtype follows the
/// same X11-style keysym names that xdotool uses, so the mapping
/// table mirrors [`crate::x11::input::normalize_key`].
pub fn normalize_key(raw: &str) -> LinuxCuResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(LinuxCuError::Protocol("empty keysym".into()));
    }
    let lower = trimmed.to_ascii_lowercase();
    let mapped = match lower.as_str() {
        "enter" | "return" => "Return",
        "esc" | "escape" => "Escape",
        "backspace" | "delete" => "BackSpace",
        "tab" => "Tab",
        "space" => "space",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        _ => trimmed,
    };
    Ok(mapped.to_string())
}
