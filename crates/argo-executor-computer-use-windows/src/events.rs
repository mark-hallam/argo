//! Input-event emission via `SendInput`.
//!
//! Every primitive uses the modern `SendInput` API; the
//! deprecated `mouse_event` / `keybd_event` calls are avoided.
//!
//! - **Mouse**: absolute coordinates normalised to `[0, 65535]`
//!   across the virtual screen via `MOUSEEVENTF_ABSOLUTE`.
//! - **Keyboard**: `VK_*` codes for combos, `KEYEVENTF_UNICODE`
//!   for plain text (so single-UTF-16 codepoints type without
//!   an IME round-trip).

use crate::config::WindowsCuConfig;
use crate::error::WindowsCuError;
use crate::WindowsCuResult;
use serde::{Deserialize, Serialize};

pub use argo_executor_computer_use::e2b::CuAction;

/// Outcome of one event post.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WindowsActionResult {
    /// Whether the OS accepted the event.
    pub ok: bool,
    /// Optional short text describing what happened.
    pub text: Option<String>,
    /// Window title holding focus after the event posted.
    pub front_app_after: Option<String>,
}

/// Validate `action` against the config and post it via
/// `SendInput`. Settle delay is honoured AFTER the post.
pub async fn post(
    action: &CuAction,
    config: &WindowsCuConfig,
) -> WindowsCuResult<WindowsActionResult> {
    let kind = action_kind(action);
    if !config.allows_action(kind) {
        return Err(WindowsCuError::ActionNotAllowed { kind });
    }
    if matches!(action, CuAction::Key { .. } | CuAction::Type { .. }) && !config.allow_send_keys {
        return Err(WindowsCuError::ActionNotAllowed { kind: "key" });
    }
    if !cfg!(target_os = "windows") {
        return Err(WindowsCuError::NotWindows);
    }
    let result = post_impl(action, config).await?;
    let settle = settle_for(action, config);
    if settle > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(settle)).await;
    }
    Ok(result)
}

/// Static action-kind label used for the allowlist + audit log.
#[must_use]
pub fn action_kind(action: &CuAction) -> &'static str {
    match action {
        CuAction::Click { .. } => "click",
        CuAction::DoubleClick { .. } => "double_click",
        CuAction::Type { .. } => "type",
        CuAction::Key { .. } => "key",
        CuAction::Scroll { .. } => "scroll",
        CuAction::MouseMove { .. } => "mouse_move",
        CuAction::Navigate { .. } => "navigate",
    }
}

fn settle_for(action: &CuAction, config: &WindowsCuConfig) -> u64 {
    match action {
        CuAction::Type { .. } | CuAction::Key { .. } => config.key_delay_ms,
        _ => config.click_settle_ms,
    }
}

/// Map a virtual-screen pixel to the normalised `[0, 65535]`
/// range that `MOUSEEVENTF_ABSOLUTE` expects.
#[must_use]
pub fn normalise_mouse_coords(
    pixel_x: i32,
    pixel_y: i32,
    virtual_origin: (i32, i32),
    virtual_size: (u32, u32),
) -> (u32, u32) {
    let (vx, vy) = virtual_origin;
    let (vw, vh) = virtual_size;
    let rel_x = pixel_x.saturating_sub(vx).max(0) as u64;
    let rel_y = pixel_y.saturating_sub(vy).max(0) as u64;
    let nx = if vw <= 1 { 0 } else { ((rel_x * 65_535) / (vw as u64 - 1)).min(65_535) as u32 };
    let ny = if vh <= 1 { 0 } else { ((rel_y * 65_535) / (vh as u64 - 1)).min(65_535) as u32 };
    (nx, ny)
}

/// Translate a friendly key name to a Windows VK_ code.
#[must_use]
pub fn vk_for_key(name: &str) -> Option<u16> {
    match name.to_lowercase().as_str() {
        "ctrl" | "control" => Some(0x11),
        "alt" | "menu" => Some(0x12),
        "shift" => Some(0x10),
        "win" | "lwin" => Some(0x5B),
        "rwin" => Some(0x5C),
        "return" | "enter" => Some(0x0D),
        "tab" => Some(0x09),
        "esc" | "escape" => Some(0x1B),
        "space" => Some(0x20),
        "backspace" => Some(0x08),
        "delete" | "del" => Some(0x2E),
        "left" => Some(0x25),
        "right" => Some(0x27),
        "up" => Some(0x26),
        "down" => Some(0x28),
        "home" => Some(0x24),
        "end" => Some(0x23),
        "pageup" | "pgup" => Some(0x21),
        "pagedown" | "pgdn" => Some(0x22),
        f if f.len() <= 3 && f.starts_with('f') => f[1..].parse::<u16>().ok().and_then(|n| {
            if (1..=12).contains(&n) {
                Some(0x6F + n)
            } else {
                None
            }
        }),
        single if single.len() == 1 => single.chars().next().and_then(|c| {
            let upper = c.to_ascii_uppercase();
            if upper.is_ascii_alphanumeric() { Some(upper as u16) } else { None }
        }),
        _ => None,
    }
}

/// Split a combo into modifier VKs + tail VK.
pub fn classify_combo(keys: &[String]) -> WindowsCuResult<(Vec<u16>, u16)> {
    if keys.is_empty() {
        return Err(WindowsCuError::Protocol("key action with empty key list".into()));
    }
    let mut modifiers = Vec::new();
    let mut tail: Option<u16> = None;
    for raw in keys {
        let vk = vk_for_key(raw)
            .ok_or_else(|| WindowsCuError::Protocol(format!("unsupported key `{raw}`")))?;
        if matches!(vk, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C) {
            modifiers.push(vk);
        } else {
            tail = Some(vk);
        }
    }
    let tail = tail.ok_or_else(|| {
        WindowsCuError::Protocol("key action without a non-modifier key".into())
    })?;
    Ok((modifiers, tail))
}

#[cfg(target_os = "windows")]
async fn post_impl(
    action: &CuAction,
    _config: &WindowsCuConfig,
) -> WindowsCuResult<WindowsActionResult> {
    match action {
        CuAction::Click { x, y } => win::send_mouse_click(*x as i32, *y as i32, false)?,
        CuAction::DoubleClick { x, y } => win::send_mouse_click(*x as i32, *y as i32, true)?,
        CuAction::MouseMove { x, y } => win::send_mouse_move(*x as i32, *y as i32)?,
        CuAction::Scroll { x, y, dy } => win::send_scroll(*x as i32, *y as i32, *dy)?,
        CuAction::Type { text } => win::send_unicode_string(text)?,
        CuAction::Key { keys } => {
            let (mods, tail) = classify_combo(keys)?;
            win::send_key_combo(&mods, tail)?;
        }
        CuAction::Navigate { url } => {
            return Err(WindowsCuError::Protocol(format!(
                "navigate(`{url}`) is not supported by the native Windows executor"
            )));
        }
    }
    Ok(WindowsActionResult {
        ok: true,
        text: Some(action.summary()),
        front_app_after: active_window_title().ok(),
    })
}

#[cfg(not(target_os = "windows"))]
async fn post_impl(
    _action: &CuAction,
    _config: &WindowsCuConfig,
) -> WindowsCuResult<WindowsActionResult> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(target_os = "windows")]
mod win {
    use super::{normalise_mouse_coords, WindowsCuError, WindowsCuResult};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
        MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_WHEEL,
        MOUSEINPUT, VIRTUAL_KEY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
        SM_YVIRTUALSCREEN,
    };

    fn send(input: INPUT) -> WindowsCuResult<()> {
        unsafe {
            let n = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if n != 1 { Err(WindowsCuError::InputBlocked) } else { Ok(()) }
        }
    }

    pub(super) fn send_mouse_move(x: i32, y: i32) -> WindowsCuResult<()> {
        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN).max(1),
                GetSystemMetrics(SM_CYVIRTUALSCREEN).max(1),
            )
        };
        let (nx, ny) = normalise_mouse_coords(x, y, (vx, vy), (vw as u32, vh as u32));
        send(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: nx as i32,
                    dy: ny as i32,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
    }

    fn mouse_button(down: bool) -> WindowsCuResult<()> {
        let flags = if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP };
        send(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
    }

    pub(super) fn send_mouse_click(x: i32, y: i32, double: bool) -> WindowsCuResult<()> {
        send_mouse_move(x, y)?;
        mouse_button(true)?;
        mouse_button(false)?;
        if double {
            mouse_button(true)?;
            mouse_button(false)?;
        }
        Ok(())
    }

    pub(super) fn send_scroll(x: i32, y: i32, dy: i32) -> WindowsCuResult<()> {
        send_mouse_move(x, y)?;
        send(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: dy.saturating_mul(120) as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
    }

    fn key_event(vk: u16, scan: u16, flag: KEYBD_EVENT_FLAGS) -> WindowsCuResult<()> {
        send(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(vk),
                    wScan: scan,
                    dwFlags: flag,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        })
    }

    pub(super) fn send_unicode_string(text: &str) -> WindowsCuResult<()> {
        for code_unit in text.encode_utf16() {
            key_event(0, code_unit, KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0))?;
            key_event(
                0,
                code_unit,
                KEYBD_EVENT_FLAGS(KEYEVENTF_UNICODE.0 | KEYEVENTF_KEYUP.0),
            )?;
        }
        Ok(())
    }

    pub(super) fn send_key_combo(modifiers: &[u16], tail: u16) -> WindowsCuResult<()> {
        for vk in modifiers {
            key_event(*vk, 0, KEYBD_EVENT_FLAGS(0))?;
        }
        key_event(tail, 0, KEYBD_EVENT_FLAGS(0))?;
        key_event(tail, 0, KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0))?;
        for vk in modifiers.iter().rev() {
            key_event(*vk, 0, KEYBD_EVENT_FLAGS(KEYEVENTF_KEYUP.0))?;
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn active_window_title() -> WindowsCuResult<String> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == 0 {
            return Err(WindowsCuError::Protocol("no foreground window".into()));
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut buf);
        if n <= 0 {
            return Err(WindowsCuError::Protocol("GetWindowTextW returned 0".into()));
        }
        Ok(String::from_utf16_lossy(&buf[..n as usize]))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn active_window_title() -> WindowsCuResult<String> {
    Err(WindowsCuError::NotWindows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(allow: &[&str]) -> WindowsCuConfig {
        let mut c = WindowsCuConfig::for_notepad_app();
        c.allowed_actions.clear();
        for a in allow {
            c.allowed_actions.insert((*a).to_string());
        }
        c
    }

    #[test]
    fn action_kind_matches_enum_variants() {
        assert_eq!(action_kind(&CuAction::Click { x: 1, y: 2 }), "click");
        assert_eq!(action_kind(&CuAction::DoubleClick { x: 1, y: 2 }), "double_click");
        assert_eq!(action_kind(&CuAction::Type { text: "x".into() }), "type");
        assert_eq!(action_kind(&CuAction::MouseMove { x: 1, y: 2 }), "mouse_move");
        assert_eq!(action_kind(&CuAction::Scroll { x: 0, y: 0, dy: 1 }), "scroll");
        assert_eq!(action_kind(&CuAction::Navigate { url: "x".into() }), "navigate");
    }

    #[test]
    fn vk_for_key_resolves_common_inputs() {
        assert_eq!(vk_for_key("ctrl"), Some(0x11));
        assert_eq!(vk_for_key("Shift"), Some(0x10));
        assert_eq!(vk_for_key("a"), Some(b'A' as u16));
        assert_eq!(vk_for_key("return"), Some(0x0D));
        assert_eq!(vk_for_key("f5"), Some(0x74));
        assert_eq!(vk_for_key("not-a-key"), None);
    }

    #[test]
    fn classify_combo_splits_modifiers_and_tail() {
        let (m, t) = classify_combo(&["ctrl".into(), "n".into()]).unwrap();
        assert_eq!(m, vec![0x11]);
        assert_eq!(t, b'N' as u16);
    }

    #[test]
    fn classify_combo_rejects_modifier_only() {
        let err = classify_combo(&["ctrl".into()]).unwrap_err();
        assert!(matches!(err, WindowsCuError::Protocol(_)));
    }

    #[test]
    fn normalise_mouse_coords_maps_origin_and_corner() {
        assert_eq!(normalise_mouse_coords(0, 0, (0, 0), (1920, 1080)), (0, 0));
        assert_eq!(
            normalise_mouse_coords(1919, 1079, (0, 0), (1920, 1080)),
            (65_535, 65_535)
        );
    }

    #[tokio::test]
    async fn post_rejects_action_not_in_allowlist() {
        let c = cfg(&["click"]);
        let err = post(&CuAction::Type { text: "x".into() }, &c).await.unwrap_err();
        assert!(matches!(err, WindowsCuError::ActionNotAllowed { kind: "type" }));
    }

    #[tokio::test]
    async fn post_rejects_key_when_send_keys_disabled() {
        let mut c = cfg(&["key"]);
        c.allow_send_keys = false;
        let err = post(&CuAction::Key { keys: vec!["return".into()] }, &c).await.unwrap_err();
        assert!(matches!(err, WindowsCuError::ActionNotAllowed { kind: "key" }));
    }
}
