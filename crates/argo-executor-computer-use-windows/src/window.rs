//! Window enumeration and focus via PowerShell + `user32!SetForegroundWindow`.

use crate::error::truncate_stderr;
use crate::probe::ensure_supported;
use crate::screenshot::ps_quote;
use crate::{WindowsCuError, WindowsCuResult};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Minimal window row returned by `Get-Process` filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowInfo {
    /// Native window handle (HWND, packed as i64 for serde stability).
    pub handle: i64,
    /// Owning process id.
    pub pid: u32,
    /// MainWindowTitle reported by the OS.
    pub title: String,
}

/// Search visible windows by title substring (case-sensitive PowerShell `-like`).
pub fn search_by_name(pattern: &str) -> WindowsCuResult<Vec<WindowInfo>> {
    ensure_supported()?;
    let output = Command::new("powershell.exe")
        .args(search_args(pattern))
        .output()?;
    if !output.status.success() {
        return Err(WindowsCuError::Subprocess {
            command: "powershell.exe Get-Process".into(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    parse_window_rows(&String::from_utf8_lossy(&output.stdout))
}

/// Bring a window handle to the foreground.
pub fn focus(handle: i64) -> WindowsCuResult<()> {
    ensure_supported()?;
    let output = Command::new("powershell.exe")
        .args(focus_args(handle))
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(WindowsCuError::Subprocess {
            command: "powershell.exe SetForegroundWindow".into(),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

pub(crate) fn search_args(pattern: &str) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        search_script(pattern),
    ]
}

pub(crate) fn focus_args(handle: i64) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        focus_script(handle),
    ]
}

pub(crate) fn search_script(pattern: &str) -> String {
    let escaped = ps_quote(&format!("*{pattern}*"));
    // Tab-separated rows: handle\tpid\ttitle. Filter out empty titles
    // because background processes always carry a 0 MainWindowHandle.
    format!(
        "Get-Process | Where-Object {{ $_.MainWindowTitle -like {escaped} -and $_.MainWindowHandle -ne 0 }} \
| ForEach-Object {{ \"$($_.MainWindowHandle.ToInt64())`t$($_.Id)`t$($_.MainWindowTitle)\" }}"
    )
}

pub(crate) fn focus_script(handle: i64) -> String {
    format!(
        "Add-Type -MemberDefinition '\
[DllImport(\"user32.dll\")] public static extern bool SetForegroundWindow(System.IntPtr hWnd);' \
-Name U32W -Namespace ArgoCu -PassThru | Out-Null; \
[ArgoCu.U32W]::SetForegroundWindow([System.IntPtr]{handle}) | Out-Null;"
    )
}

pub(crate) fn parse_window_rows(raw: &str) -> WindowsCuResult<Vec<WindowInfo>> {
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_one_row)
        .collect()
}

fn parse_one_row(line: &str) -> WindowsCuResult<WindowInfo> {
    let mut parts = line.splitn(3, '\t');
    let handle_raw = parts
        .next()
        .ok_or_else(|| WindowsCuError::Protocol(format!("missing handle in `{line}`")))?
        .trim();
    let pid_raw = parts
        .next()
        .ok_or_else(|| WindowsCuError::Protocol(format!("missing pid in `{line}`")))?
        .trim();
    let title = parts
        .next()
        .ok_or_else(|| WindowsCuError::Protocol(format!("missing title in `{line}`")))?
        .trim()
        .to_string();
    let handle = handle_raw
        .parse::<i64>()
        .map_err(|e| WindowsCuError::Protocol(format!("bad handle `{handle_raw}`: {e}")))?;
    let pid = pid_raw
        .parse::<u32>()
        .map_err(|e| WindowsCuError::Protocol(format!("bad pid `{pid_raw}`: {e}")))?;
    Ok(WindowInfo { handle, pid, title })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tab_separated_rows() {
        let rows = parse_window_rows("12345\t4321\tNotepad - Untitled\n").unwrap();
        assert_eq!(
            rows,
            vec![WindowInfo {
                handle: 12345,
                pid: 4321,
                title: "Notepad - Untitled".into(),
            }]
        );
    }

    #[test]
    fn rejects_non_numeric_handle() {
        let err = parse_window_rows("abc\t1\tx").unwrap_err();
        assert!(matches!(err, WindowsCuError::Protocol(_)));
    }

    #[test]
    fn search_script_uses_no_execution_policy_bypass() {
        let s = search_script("Notepad");
        assert!(!s.to_lowercase().contains("set-executionpolicy"));
        assert!(!s.to_lowercase().contains("bypass"));
        assert!(s.contains("'*Notepad*'"));
    }

    #[test]
    fn focus_script_routes_through_setforegroundwindow() {
        let s = focus_script(987);
        assert!(s.contains("SetForegroundWindow"));
        assert!(s.contains("987"));
    }

    #[test]
    fn search_args_carry_no_profile_and_command() {
        let args = search_args("Notepad");
        assert_eq!(args[0], "-NoProfile");
        assert_eq!(args[1], "-NonInteractive");
        assert_eq!(args[2], "-Command");
    }

    #[test]
    fn search_is_unsupported_off_windows() {
        if !cfg!(target_os = "windows") {
            let err = search_by_name("Notepad").unwrap_err();
            assert!(matches!(err, WindowsCuError::Unsupported { .. }));
        }
    }
}
