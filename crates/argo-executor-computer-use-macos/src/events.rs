//! Input injection via `osascript` → System Events.
//!
//! Why subprocess vs `core-graphics::CGEvent`: the V1.5 PoC keeps
//! the dep list minimal. AppleScript's System Events handles the
//! same primitives we need (click, type, key, scroll) and respects
//! the same Accessibility permission gate. The follow-up brief
//! adds CGEvent for finer coordinate control + scroll precision.
//!
//! Each event posted is followed by a configurable dwell so an
//! operator can `Ctrl+C` mid-loop.

use crate::config::MacosCuConfig;
use crate::error::{truncate_stderr, MacosCuError};
use crate::MacosCuResult;
use serde::{Deserialize, Serialize};

/// Re-export of the existing CU primitive enum from the E2B
/// crate. Both executors speak the same action shape, so the
/// dispatcher can hand off between them without translating.
pub use argo_executor_computer_use::e2b::CuAction;

/// Outcome of one event post — mirrors
/// [`argo_executor_computer_use::e2b::ActionResult`] but adds a
/// `front_app_after` field so the audit log can flag focus
/// drifts mid-loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MacosActionResult {
    /// Whether the OS accepted the event.
    pub ok: bool,
    /// Optional short text describing what happened (e.g., echo of
    /// typed text, key combo).
    pub text: Option<String>,
    /// Bundle id of the app holding focus after the event posted.
    /// `None` if the active-app probe failed (treated as
    /// `Unknown`).
    pub front_app_after: Option<String>,
}

/// Validate `action` against the config and post it via System
/// Events. `dwell` is honored AFTER the post (so the next step
/// always sees the latest state).
pub async fn post(
    action: &CuAction,
    config: &MacosCuConfig,
) -> MacosCuResult<MacosActionResult> {
    let kind = action_kind(action);
    if !config.allows_action(kind) {
        return Err(MacosCuError::ActionNotAllowed { kind });
    }
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    let result = post_impl(action, config).await?;
    let dwell = std::time::Duration::from_millis(u64::from(config.dwell_ms));
    tokio::time::sleep(dwell).await;
    Ok(result)
}

/// Static action-kind label used for the allowlist + audit log.
/// Matches the names in `MacosCuConfig::for_notes_app()`.
#[must_use]
pub fn action_kind(action: &CuAction) -> &'static str {
    match action {
        CuAction::Click { .. } => "click",
        CuAction::DoubleClick { .. } => "double_click",
        CuAction::Type { .. } => "type",
        CuAction::Key { .. } => "key",
        CuAction::Scroll { .. } => "scroll",
        CuAction::MouseMove { .. } => "mouse_move",
        // `Navigate` is the E2B-browser-only primitive — for
        // native macOS we reject it (the model shouldn't be
        // navigating URLs from the desktop).
        CuAction::Navigate { .. } => "navigate",
    }
}

#[cfg(target_os = "macos")]
async fn post_impl(
    action: &CuAction,
    config: &MacosCuConfig,
) -> MacosCuResult<MacosActionResult> {
    let script = build_script(action, config)?;
    let output = tokio::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .await?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "osascript[event]".to_string(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    let echo = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let front = active_app().await.ok();
    Ok(MacosActionResult {
        ok: true,
        text: if echo.is_empty() { None } else { Some(echo) },
        front_app_after: front,
    })
}

#[cfg(not(target_os = "macos"))]
async fn post_impl(
    _action: &CuAction,
    _config: &MacosCuConfig,
) -> MacosCuResult<MacosActionResult> {
    Err(MacosCuError::NotMacos)
}

/// Build the AppleScript snippet for one CU action. Pure
/// (testable on every platform) — the subprocess invocation is
/// in `post_impl`.
pub(crate) fn build_script(
    action: &CuAction,
    config: &MacosCuConfig,
) -> MacosCuResult<String> {
    let app_target = format!("application id \"{}\"", config.target_app);
    let script = match action {
        CuAction::Click { x, y } => format!(
            "tell {app_target} to activate\ntell application \"System Events\" to click at {{{x}, {y}}}"
        ),
        CuAction::DoubleClick { x, y } => format!(
            "tell {app_target} to activate\ntell application \"System Events\" to perform action \"AXPress\" of (UI element 1 at {{{x}, {y}}})"
        ),
        CuAction::Type { text } => format!(
            "tell {app_target} to activate\ntell application \"System Events\" to keystroke \"{}\"",
            escape_applescript(text)
        ),
        CuAction::Key { keys } => keys_script(&app_target, keys)?,
        CuAction::Scroll { x: _, y: _, dy } => format!(
            "tell {app_target} to activate\ntell application \"System Events\" to key code {} using {{}}",
            scroll_keycode(*dy)
        ),
        CuAction::MouseMove { x, y } => format!(
            "tell {app_target} to activate\ntell application \"System Events\" to set the position of the mouse to {{{x}, {y}}}"
        ),
        CuAction::Navigate { url } => {
            return Err(MacosCuError::Protocol(format!(
                "navigate(`{url}`) is not supported by the native macOS executor"
            )));
        }
    };
    Ok(script)
}

/// Best-effort lookup of the bundle id of the front app. Returns
/// the bundle id on success or a structured error if the probe
/// fails. Used inside `MacosActionResult::front_app_after`.
#[cfg(target_os = "macos")]
pub async fn active_app() -> MacosCuResult<String> {
    let output = tokio::process::Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get bundle identifier of (first process whose frontmost is true)",
        ])
        .output()
        .await?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: "osascript[active_app]".to_string(),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(not(target_os = "macos"))]
pub async fn active_app() -> MacosCuResult<String> {
    Err(MacosCuError::NotMacos)
}

/// Translate a multi-key combo (e.g., `["cmd", "n"]`) into the
/// AppleScript shape `keystroke "n" using {command down}`.
fn keys_script(app_target: &str, keys: &[String]) -> MacosCuResult<String> {
    if keys.is_empty() {
        return Err(MacosCuError::Protocol(
            "key action with empty key list".to_string(),
        ));
    }
    let mut modifiers: Vec<&'static str> = Vec::new();
    let mut letter: Option<&str> = None;
    let mut keycode: Option<&'static str> = None;

    for raw in keys {
        let lower = raw.to_lowercase();
        match lower.as_str() {
            "cmd" | "command" | "meta" => modifiers.push("command down"),
            "ctrl" | "control" => modifiers.push("control down"),
            "alt" | "opt" | "option" => modifiers.push("option down"),
            "shift" => modifiers.push("shift down"),
            "return" | "enter" => keycode = Some("36"),
            "tab" => keycode = Some("48"),
            "esc" | "escape" => keycode = Some("53"),
            "left" => keycode = Some("123"),
            "right" => keycode = Some("124"),
            "down" => keycode = Some("125"),
            "up" => keycode = Some("126"),
            "delete" | "backspace" => keycode = Some("51"),
            other if other.len() == 1 => letter = Some(raw),
            other => {
                return Err(MacosCuError::Protocol(format!(
                    "unsupported key `{other}` — extend `keys_script` if needed"
                )));
            }
        }
    }

    let modifier_clause = if modifiers.is_empty() {
        "{}".to_string()
    } else {
        format!("{{{}}}", modifiers.join(", "))
    };

    if let Some(letter) = letter {
        Ok(format!(
            "tell {app_target} to activate\ntell application \"System Events\" to keystroke \"{}\" using {modifier_clause}",
            escape_applescript(letter)
        ))
    } else if let Some(code) = keycode {
        Ok(format!(
            "tell {app_target} to activate\ntell application \"System Events\" to key code {code} using {modifier_clause}"
        ))
    } else {
        Err(MacosCuError::Protocol(
            "key action with no resolvable letter or keycode".to_string(),
        ))
    }
}

/// Map a scroll dy delta to the closest arrow keycode. Positive
/// dy → down arrow; negative dy → up arrow. The PoC ships
/// arrow-key approximation; CGEvent scroll precision lands in
/// the follow-up.
fn scroll_keycode(dy: i32) -> &'static str {
    if dy >= 0 {
        "125" // down arrow
    } else {
        "126" // up arrow
    }
}

/// Escape `"` and `\` for embedding in an AppleScript double-
/// quoted string literal.
pub(crate) fn escape_applescript(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_actions(target: &str, actions: &[&str]) -> MacosCuConfig {
        let mut cfg = MacosCuConfig::for_notes_app();
        cfg.target_app = target.to_string();
        cfg.allowed_apps.insert(target.to_string());
        cfg.allowed_actions.clear();
        for a in actions {
            cfg.allowed_actions.insert((*a).to_string());
        }
        cfg
    }

    #[test]
    fn action_kind_matches_enum_variants() {
        assert_eq!(
            action_kind(&CuAction::Click { x: 1, y: 2 }),
            "click"
        );
        assert_eq!(action_kind(&CuAction::Type { text: "x".into() }), "type");
        assert_eq!(
            action_kind(&CuAction::Key {
                keys: vec!["cmd".into(), "n".into()]
            }),
            "key"
        );
    }

    #[test]
    fn navigate_action_kind_has_label() {
        // Even though native macOS rejects navigate, the kind
        // string must exist so the audit row prints something.
        assert_eq!(
            action_kind(&CuAction::Navigate {
                url: "https://x".into()
            }),
            "navigate"
        );
    }

    #[test]
    fn build_script_click_emits_system_events_call() {
        let cfg = cfg_with_actions("com.apple.Notes", &["click"]);
        let s = build_script(&CuAction::Click { x: 100, y: 200 }, &cfg).unwrap();
        assert!(s.contains("application id \"com.apple.Notes\""));
        assert!(s.contains("click at {100, 200}"));
    }

    #[test]
    fn build_script_type_escapes_quotes() {
        let cfg = cfg_with_actions("com.apple.Notes", &["type"]);
        let s = build_script(
            &CuAction::Type {
                text: "hello \"world\"".into(),
            },
            &cfg,
        )
        .unwrap();
        assert!(s.contains("hello \\\"world\\\""));
    }

    #[test]
    fn build_script_key_combo_emits_modifier_clause() {
        let cfg = cfg_with_actions("com.apple.Notes", &["key"]);
        let s = build_script(
            &CuAction::Key {
                keys: vec!["cmd".into(), "n".into()],
            },
            &cfg,
        )
        .unwrap();
        assert!(s.contains("keystroke \"n\""));
        assert!(s.contains("command down"));
    }

    #[test]
    fn build_script_navigate_rejected() {
        let cfg = cfg_with_actions("com.apple.Notes", &["navigate"]);
        let err = build_script(
            &CuAction::Navigate {
                url: "https://x".into(),
            },
            &cfg,
        )
        .unwrap_err();
        assert!(matches!(err, MacosCuError::Protocol(_)));
    }

    #[test]
    fn build_script_named_keys_use_keycode() {
        let cfg = cfg_with_actions("com.apple.Notes", &["key"]);
        let s = build_script(
            &CuAction::Key {
                keys: vec!["return".into()],
            },
            &cfg,
        )
        .unwrap();
        assert!(s.contains("key code 36"));
    }

    #[test]
    fn build_script_unknown_key_errors() {
        let cfg = cfg_with_actions("com.apple.Notes", &["key"]);
        let err = build_script(
            &CuAction::Key {
                keys: vec!["someweirdkey".into()],
            },
            &cfg,
        )
        .unwrap_err();
        assert!(matches!(err, MacosCuError::Protocol(_)));
    }

    #[tokio::test]
    async fn post_rejects_action_not_in_allowlist() {
        let cfg = cfg_with_actions("com.apple.Notes", &["click"]);
        let err = post(&CuAction::Type { text: "x".into() }, &cfg)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            MacosCuError::ActionNotAllowed { kind: "type" }
        ));
    }

    #[test]
    fn escape_applescript_handles_quotes_and_backslashes() {
        assert_eq!(escape_applescript("hello"), "hello");
        assert_eq!(escape_applescript("a \"b\" c"), "a \\\"b\\\" c");
        assert_eq!(escape_applescript("a\\b"), "a\\\\b");
    }

    #[test]
    fn scroll_keycode_picks_arrow_by_sign() {
        assert_eq!(scroll_keycode(5), "125"); // down arrow
        assert_eq!(scroll_keycode(0), "125"); // tie → down
        assert_eq!(scroll_keycode(-3), "126"); // up arrow
    }
}
