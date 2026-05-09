//! Mouse and keyboard injection via `xdotool`.

use crate::error::truncate_stderr;
use crate::probe::ensure_supported;
use crate::{LinuxCuError, LinuxCuResult};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Result of one input action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputResult {
    /// Action label.
    pub action: String,
    /// Human-readable detail for audit logs.
    pub detail: String,
}

/// Move the pointer to `(x, y)` and left-click once.
pub fn click(x: i32, y: i32) -> LinuxCuResult<InputResult> {
    ensure_supported()?;
    run_xdotool(&click_args(x, y))?;
    Ok(InputResult {
        action: "click".into(),
        detail: format!("{x},{y}"),
    })
}

/// Press one key symbol, e.g. `Return`, `Escape`, or `ctrl+l`.
pub fn key(keysym: &str) -> LinuxCuResult<InputResult> {
    ensure_supported()?;
    let key = normalize_key(keysym)?;
    run_xdotool(&key_args(&key))?;
    Ok(InputResult {
        action: "key".into(),
        detail: key,
    })
}

/// Type text into the focused target.
pub fn type_text(text: &str) -> LinuxCuResult<InputResult> {
    ensure_supported()?;
    run_xdotool(&type_args(text))?;
    Ok(InputResult {
        action: "type".into(),
        detail: format!("{} chars", text.chars().count()),
    })
}

fn run_xdotool(args: &[String]) -> LinuxCuResult<()> {
    let output = Command::new("xdotool").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(LinuxCuError::Subprocess {
            command: format!("xdotool {}", args.join(" ")),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

pub(crate) fn click_args(x: i32, y: i32) -> Vec<String> {
    vec![
        "mousemove".into(),
        x.to_string(),
        y.to_string(),
        "click".into(),
        "1".into(),
    ]
}

pub(crate) fn key_args(keysym: &str) -> Vec<String> {
    vec!["key".into(), keysym.into()]
}

pub(crate) fn type_args(text: &str) -> Vec<String> {
    vec![
        "type".into(),
        "--delay".into(),
        "0".into(),
        "--".into(),
        text.into(),
    ]
}

pub(crate) fn normalize_key(raw: &str) -> LinuxCuResult<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_args_match_xdotool_shape() {
        assert_eq!(
            click_args(10, 20),
            vec!["mousemove", "10", "20", "click", "1"]
        );
    }

    #[test]
    fn type_args_insert_double_dash_before_text() {
        assert_eq!(
            type_args("hello"),
            vec!["type", "--delay", "0", "--", "hello"]
        );
    }

    #[test]
    fn common_key_aliases_normalize_to_xdotool_symbols() {
        assert_eq!(normalize_key("enter").unwrap(), "Return");
        assert_eq!(normalize_key("esc").unwrap(), "Escape");
        assert_eq!(normalize_key("ctrl+l").unwrap(), "ctrl+l");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wrappers_use_xdotool_from_path() {
        let _guard = crate::test_support::ENV_LOCK.lock().unwrap();
        let dir = crate::test_support::temp_dir("xdotool-input");
        let log = dir.join("xdotool.log");
        crate::test_support::write_executable(
            &dir,
            "xdotool",
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$ARGO_XDOTOOL_LOG\"\nexit 0\n",
        );
        crate::test_support::write_executable(&dir, "scrot", "#!/bin/sh\nexit 0\n");
        let old_path = std::env::var_os("PATH");
        let old_display = std::env::var_os("DISPLAY");
        let old_log = std::env::var_os("ARGO_XDOTOOL_LOG");
        std::env::set_var("PATH", &dir);
        std::env::set_var("DISPLAY", ":99");
        std::env::set_var("ARGO_XDOTOOL_LOG", &log);
        click(10, 20).unwrap();
        key("enter").unwrap();
        type_text("hello").unwrap();
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        }
        if let Some(display) = old_display {
            std::env::set_var("DISPLAY", display);
        } else {
            std::env::remove_var("DISPLAY");
        }
        if let Some(value) = old_log {
            std::env::set_var("ARGO_XDOTOOL_LOG", value);
        } else {
            std::env::remove_var("ARGO_XDOTOOL_LOG");
        }
        let body = std::fs::read_to_string(log).unwrap();
        assert!(body.contains("mousemove"));
        assert!(body.contains("Return"));
        assert!(body.contains("hello"));
    }
}
