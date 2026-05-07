//! Accessibility helpers — V1.5 PoC uses AppleScript via System
//! Events to bring apps to front, query open windows, and read
//! basic AX info. The AXUIElement framework integration (`pub
//! mod axui` below — currently a stub) lands as a follow-up brief
//! once the screenshot + events path is proven.

use crate::config::MacosCuConfig;
use crate::error::{truncate_stderr, MacosCuError};
use crate::MacosCuResult;

/// Activate `bundle_id`, bringing it to front. Validates against
/// the config's app allowlist before issuing any subprocess call.
pub async fn activate_app(
    bundle_id: &str,
    config: &MacosCuConfig,
) -> MacosCuResult<()> {
    if !config.allows_app(bundle_id) {
        return Err(MacosCuError::AppNotAllowed {
            bundle_id: bundle_id.to_string(),
        });
    }
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    activate_app_impl(bundle_id).await
}

#[cfg(target_os = "macos")]
async fn activate_app_impl(bundle_id: &str) -> MacosCuResult<()> {
    let script = format!("tell application id \"{bundle_id}\" to activate");
    let output = tokio::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .await?;
    if !output.status.success() {
        return Err(MacosCuError::Subprocess {
            command: format!("osascript[activate {bundle_id}]"),
            stderr: truncate_stderr(&output.stderr),
        });
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
async fn activate_app_impl(_bundle_id: &str) -> MacosCuResult<()> {
    Err(MacosCuError::NotMacos)
}

/// Confirm the front app matches `expected`. Surfaces
/// [`MacosCuError::FrontAppMismatch`] if Spotlight, Notification
/// Center, or another window grabbed focus mid-loop.
pub async fn assert_front_app_is(expected: &str) -> MacosCuResult<()> {
    if !cfg!(target_os = "macos") {
        return Err(MacosCuError::NotMacos);
    }
    let actual = crate::events::active_app().await?;
    if actual != expected {
        return Err(MacosCuError::FrontAppMismatch {
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

/// AXUIElement-backed discovery layer. Currently a stub —
/// designed to land as a separate brief once
/// `accessibility-sys` is on the workspace dep list. Outline:
///
/// - `find_element_by_role(role: AXRole, title: Option<&str>)`
/// - `perform_action(elem: AXUIElement, action: AXAction)`
/// - `value_of(elem: AXUIElement) -> String`
///
/// See [`docs/skycot/COMPUTER_USE_NATIVE_MACOS.md`] §3 for the
/// "hybrid CGEvent + a11y action" rationale.
pub mod axui {
    use crate::error::MacosCuError;
    use crate::MacosCuResult;

    /// Placeholder for the AXRole-keyed query.
    pub fn find_button_by_title(_title: &str) -> MacosCuResult<()> {
        Err(MacosCuError::Protocol(
            "axui::find_button_by_title is a V1.5 stub — see follow-up brief".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MacosCuConfig;

    #[tokio::test]
    async fn activate_rejects_app_outside_allowlist() {
        let cfg = MacosCuConfig::for_notes_app();
        let err = activate_app("com.apple.Terminal", &cfg).await.unwrap_err();
        assert!(matches!(err, MacosCuError::AppNotAllowed { .. }));
    }

    #[tokio::test]
    async fn axui_stub_surfaces_clear_error() {
        let err = axui::find_button_by_title("New Note").unwrap_err();
        match err {
            MacosCuError::Protocol(msg) => {
                assert!(msg.contains("V1.5 stub"));
            }
            other => panic!("expected Protocol error, got {other:?}"),
        }
    }
}
