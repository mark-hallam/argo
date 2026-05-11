//! Mouse and keyboard injection via PowerShell + user32.dll P/Invoke.

use crate::error::truncate_stderr;
use crate::probe::ensure_supported;
use crate::screenshot::ps_quote;
use crate::{WindowsCuError, WindowsCuResult};
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
pub fn click(x: i32, y: i32) -> WindowsCuResult<InputResult> {
    ensure_supported()?;
    run_powershell(&click_args(x, y), "powershell.exe click")?;
    Ok(InputResult {
        action: "click".into(),
        detail: format!("{x},{y}"),
    })
}

/// Press one named key, e.g. `Enter`, `Escape`, `F1`, `Ctrl+L`.
pub fn key(name: &str) -> WindowsCuResult<InputResult> {
    ensure_supported()?;
    let vk = lookup_vk(name)?;
    run_powershell(&key_args(vk), "powershell.exe key")?;
    Ok(InputResult {
        action: "key".into(),
        detail: name.to_string(),
    })
}

/// Type text into the focused target via SendKeys, escaping the
/// reserved meta characters per MSDN rules.
pub fn type_text(text: &str) -> WindowsCuResult<InputResult> {
    ensure_supported()?;
    run_powershell(&type_args(text), "powershell.exe type")?;
    Ok(InputResult {
        action: "type".into(),
        detail: format!("{} chars", text.chars().count()),
    })
}

fn run_powershell(args: &[String], label: &str) -> WindowsCuResult<()> {
    let output = Command::new("powershell.exe").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(WindowsCuError::Subprocess {
            command: label.into(),
            stderr: truncate_stderr(&output.stderr),
        })
    }
}

pub(crate) fn click_args(x: i32, y: i32) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        click_script(x, y),
    ]
}

pub(crate) fn key_args(vk: u16) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        key_script(vk),
    ]
}

pub(crate) fn type_args(text: &str) -> Vec<String> {
    vec![
        "-NoProfile".into(),
        "-NonInteractive".into(),
        "-Command".into(),
        type_script(text),
    ]
}

pub(crate) fn click_script(x: i32, y: i32) -> String {
    // user32 P/Invoke: SetCursorPos + mouse_event left-down/up.
    // Constants: MOUSEEVENTF_LEFTDOWN=0x02, MOUSEEVENTF_LEFTUP=0x04.
    format!(
        "Add-Type -MemberDefinition '\
[DllImport(\"user32.dll\")] public static extern bool SetCursorPos(int X, int Y); \
[DllImport(\"user32.dll\")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, System.UIntPtr dwExtraInfo);' \
-Name U32 -Namespace ArgoCu -PassThru | Out-Null; \
[ArgoCu.U32]::SetCursorPos({x}, {y}) | Out-Null; \
[ArgoCu.U32]::mouse_event(0x02, 0, 0, 0, [System.UIntPtr]::Zero); \
[ArgoCu.U32]::mouse_event(0x04, 0, 0, 0, [System.UIntPtr]::Zero);"
    )
}

pub(crate) fn key_script(vk: u16) -> String {
    // user32 P/Invoke: keybd_event down + up (KEYEVENTF_KEYUP=0x02).
    format!(
        "Add-Type -MemberDefinition '\
[DllImport(\"user32.dll\")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, System.UIntPtr dwExtraInfo);' \
-Name U32K -Namespace ArgoCu -PassThru | Out-Null; \
[ArgoCu.U32K]::keybd_event([byte]{vk}, 0, 0, [System.UIntPtr]::Zero); \
[ArgoCu.U32K]::keybd_event([byte]{vk}, 0, 0x02, [System.UIntPtr]::Zero);"
    )
}

pub(crate) fn type_script(text: &str) -> String {
    let escaped = ps_quote(&escape_sendkeys(text));
    format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
[System.Windows.Forms.SendKeys]::SendWait({escaped});"
    )
}

/// Escape the seven reserved SendKeys meta characters per MSDN.
///
/// `+ ^ % ~ ( ) { }` plus `[` and `]` are wrapped in `{}` so the
/// receiver sees them literally rather than as modifiers/grouping.
pub(crate) fn escape_sendkeys(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '+' | '^' | '%' | '~' | '(' | ')' | '{' | '}' | '[' | ']' => {
                out.push('{');
                out.push(c);
                out.push('}');
            }
            other => out.push(other),
        }
    }
    out
}

/// Map a friendly key name to a Windows virtual-key code.
///
/// Subset sufficient for the POC mirror layer; values come from
/// MSDN "Virtual-Key Codes". Unknown names return [`Protocol`].
///
/// [`Protocol`]: WindowsCuError::Protocol
pub(crate) fn lookup_vk(name: &str) -> WindowsCuResult<u16> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(WindowsCuError::Protocol("empty key name".into()));
    }
    let lower = trimmed.to_ascii_lowercase();
    let vk = match lower.as_str() {
        "back" | "backspace" => 0x08,
        "tab" => 0x09,
        "enter" | "return" => 0x0D,
        "shift" => 0x10,
        "ctrl" | "control" => 0x11,
        "alt" | "menu" => 0x12,
        "pause" => 0x13,
        "capslock" | "caps" => 0x14,
        "esc" | "escape" => 0x1B,
        "space" => 0x20,
        "pageup" => 0x21,
        "pagedown" => 0x22,
        "end" => 0x23,
        "home" => 0x24,
        "left" => 0x25,
        "up" => 0x26,
        "right" => 0x27,
        "down" => 0x28,
        "insert" => 0x2D,
        "delete" | "del" => 0x2E,
        "f1" => 0x70,
        "f2" => 0x71,
        "f3" => 0x72,
        "f4" => 0x73,
        "f5" => 0x74,
        "f6" => 0x75,
        "f7" => 0x76,
        "f8" => 0x77,
        "f9" => 0x78,
        "f10" => 0x79,
        "f11" => 0x7A,
        "f12" => 0x7B,
        single if single.len() == 1 => {
            let ch = single.chars().next().unwrap_or(' ').to_ascii_uppercase();
            if ch.is_ascii_alphanumeric() {
                ch as u16
            } else {
                return Err(WindowsCuError::Protocol(format!(
                    "unknown key `{name}` (only A-Z, 0-9 and named keys are supported)"
                )));
            }
        }
        _ => {
            return Err(WindowsCuError::Protocol(format!(
                "unknown key `{name}`; extend lookup_vk if you need this VK"
            )))
        }
    };
    Ok(vk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_args_carry_no_profile_and_inline_command() {
        let args = click_args(10, 20);
        assert_eq!(args[0], "-NoProfile");
        assert_eq!(args[1], "-NonInteractive");
        assert_eq!(args[2], "-Command");
        assert!(args[3].contains("SetCursorPos(10, 20)"));
        assert!(args[3].contains("mouse_event"));
    }

    #[test]
    fn type_args_escape_sendkeys_meta_chars() {
        let args = type_args("hello+world");
        assert!(args[3].contains("hello{+}world"));
    }

    #[test]
    fn type_args_pass_text_via_single_quoted_argument() {
        let args = type_args("hello");
        assert!(args[3].contains("'hello'"));
    }

    #[test]
    fn lookup_vk_handles_named_and_letter_keys() {
        assert_eq!(lookup_vk("enter").unwrap(), 0x0D);
        assert_eq!(lookup_vk("ESC").unwrap(), 0x1B);
        assert_eq!(lookup_vk("a").unwrap(), 'A' as u16);
        assert_eq!(lookup_vk("F5").unwrap(), 0x74);
        assert!(lookup_vk("").is_err());
        assert!(lookup_vk("badkey").is_err());
    }

    #[test]
    fn escape_sendkeys_wraps_all_reserved_chars() {
        let raw = "+^%~(){}[]";
        let esc = escape_sendkeys(raw);
        assert_eq!(esc, "{+}{^}{%}{~}{(}{)}{{}{}}{[}{]}");
    }

    #[test]
    fn escape_sendkeys_passes_plain_ascii_unchanged() {
        assert_eq!(escape_sendkeys("hello world"), "hello world");
    }

    #[test]
    fn key_script_does_not_bypass_execution_policy() {
        let s = key_script(0x0D);
        assert!(!s.to_lowercase().contains("bypass"));
        assert!(!s.to_lowercase().contains("set-executionpolicy"));
    }

    #[test]
    fn click_is_unsupported_off_windows() {
        if !cfg!(target_os = "windows") {
            let err = click(10, 20).unwrap_err();
            assert!(matches!(err, WindowsCuError::Unsupported { .. }));
        }
    }
}
