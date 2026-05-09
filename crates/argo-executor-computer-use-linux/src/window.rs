//! Window search and focus via `xdotool`.

use crate::error::truncate_stderr;
use crate::probe::ensure_supported;
use crate::{LinuxCuError, LinuxCuResult};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Minimal window row returned by `xdotool search --name`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    /// X11 window id.
    pub id: u64,
}

/// Search windows by title/name pattern.
pub fn search_by_name(pattern: &str) -> LinuxCuResult<Vec<WindowInfo>> {
    ensure_supported()?;
    let output = Command::new("xdotool")
        .args(["search", "--name", pattern])
        .output()?;
    if !output.status.success() {
        return Err(LinuxCuError::Subprocess {
            command: "xdotool search --name".into(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    parse_window_ids(&String::from_utf8_lossy(&output.stdout))
}

/// Activate a window id.
pub fn focus(window_id: u64) -> LinuxCuResult<()> {
    ensure_supported()?;
    let id = window_id.to_string();
    let output = Command::new("xdotool")
        .args(["windowactivate", id.as_str()])
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(LinuxCuError::Subprocess {
            command: "xdotool windowactivate".into(),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

pub(crate) fn parse_window_ids(raw: &str) -> LinuxCuResult<Vec<WindowInfo>> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<u64>()
                .map(|id| WindowInfo { id })
                .map_err(|e| LinuxCuError::Protocol(format!("bad window id `{line}`: {e}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_window_ids() {
        let ids = parse_window_ids("100\n200\n").unwrap();
        assert_eq!(ids, vec![WindowInfo { id: 100 }, WindowInfo { id: 200 }]);
    }

    #[test]
    fn rejects_non_numeric_window_id() {
        let err = parse_window_ids("abc").unwrap_err();
        assert!(matches!(err, LinuxCuError::Protocol(_)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn wrappers_use_xdotool_from_path() {
        let _guard = crate::test_support::ENV_LOCK.lock().unwrap();
        let dir = crate::test_support::temp_dir("xdotool-window");
        crate::test_support::write_executable(
            &dir,
            "xdotool",
            "#!/bin/sh\nif [ \"$1\" = \"search\" ]; then echo 123; exit 0; fi\nexit 0\n",
        );
        crate::test_support::write_executable(&dir, "scrot", "#!/bin/sh\nexit 0\n");
        let old_path = std::env::var_os("PATH");
        let old_display = std::env::var_os("DISPLAY");
        std::env::set_var("PATH", &dir);
        std::env::set_var("DISPLAY", ":99");
        let rows = search_by_name("Terminal").unwrap();
        focus(123).unwrap();
        if let Some(path) = old_path {
            std::env::set_var("PATH", path);
        }
        if let Some(display) = old_display {
            std::env::set_var("DISPLAY", display);
        } else {
            std::env::remove_var("DISPLAY");
        }
        assert_eq!(rows, vec![WindowInfo { id: 123 }]);
    }
}
