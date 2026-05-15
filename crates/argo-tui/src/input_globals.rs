//! Global key dispatch for top-level TUI shortcuts.
//!
//! This module owns keybindings that are available from the normal
//! session viewer, after modal text inputs and overlays have had a
//! chance to consume the key. New global overlays should register
//! here rather than adding another top-level arm to `input.rs`.
use crate::app::ViewerState;
use argo_store::SessionStore;
use argo_types::ArgoResult;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fmt;
use std::io::Stdout;
use std::path::Path;
use std::sync::Arc;

/// Action invoked by a registered global key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalKeyAction {
    /// Open the help overlay.
    HelpOverlay,
    /// Open or close the context-aware keybindings help overlay.
    KeybindingsHelpOverlay,
    /// Delegate to the store-backed overlay dispatcher.
    StoreBackedOverlay,
    /// Delegate to the H-stack overlay dispatcher.
    HStackOverlay,
    /// Open the replay overlay.
    ReplayOverlay,
    /// Open the dashboard overlay.
    DashboardOverlay,
    /// Open the universal Ctrl-K search overlay.
    SearchEverywhereOverlay,
    /// Open the fleet spend monitor overlay.
    SpendMonitorOverlay,
    /// Open the pending-approvals tray.
    ApprovalTrayOverlay,
}

/// One global keybinding entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlobalKeyBinding {
    /// Key that triggers the action.
    pub key: KeyCode,
    /// Required modifiers for the key.
    pub modifiers: KeyModifiers,
    /// Action invoked for the key.
    pub action: GlobalKeyAction,
}

/// Error returned when a registry-builder tries to bind a key twice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DuplicateGlobalKey {
    /// Key that already exists in the registry.
    pub key: KeyCode,
}

impl fmt::Display for DuplicateGlobalKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "duplicate global key binding: {:?}", self.key)
    }
}

impl std::error::Error for DuplicateGlobalKey {}

/// Register one key/action pair in a caller-owned registry.
pub fn register_global_key(
    registry: &mut Vec<GlobalKeyBinding>,
    key: KeyCode,
    action: GlobalKeyAction,
) -> Result<(), DuplicateGlobalKey> {
    register_global_key_with_modifiers(registry, key, KeyModifiers::NONE, action)
}

/// Register one modified key/action pair in a caller-owned registry.
pub fn register_global_key_with_modifiers(
    registry: &mut Vec<GlobalKeyBinding>,
    key: KeyCode,
    modifiers: KeyModifiers,
    action: GlobalKeyAction,
) -> Result<(), DuplicateGlobalKey> {
    if registry
        .iter()
        .any(|binding| binding.key == key && binding.modifiers == modifiers)
    {
        return Err(DuplicateGlobalKey { key });
    }
    registry.push(GlobalKeyBinding {
        key,
        modifiers,
        action,
    });
    Ok(())
}

/// Return the built-in global key registry used by the TUI.
pub fn default_global_keys() -> Vec<GlobalKeyBinding> {
    let mut registry = Vec::new();
    for (key, action) in [
        (KeyCode::Char('?'), GlobalKeyAction::KeybindingsHelpOverlay),
        (KeyCode::Char('F'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('S'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('$'), GlobalKeyAction::SpendMonitorOverlay),
        (KeyCode::Char('K'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('P'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('T'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('A'), GlobalKeyAction::ApprovalTrayOverlay),
        (KeyCode::Char('E'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('M'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('O'), GlobalKeyAction::HStackOverlay),
        (KeyCode::Char('H'), GlobalKeyAction::HStackOverlay),
        (KeyCode::Char('B'), GlobalKeyAction::HStackOverlay),
        (KeyCode::Char('V'), GlobalKeyAction::ReplayOverlay),
        (KeyCode::Char('D'), GlobalKeyAction::DashboardOverlay),
    ] {
        let _ = register_global_key(&mut registry, key, action);
    }
    let _ = register_global_key_with_modifiers(
        &mut registry,
        KeyCode::Char('k'),
        KeyModifiers::CONTROL,
        GlobalKeyAction::SearchEverywhereOverlay,
    );
    registry
}

/// Look up a key in a registry.
pub fn lookup_global_key(registry: &[GlobalKeyBinding], key: KeyCode) -> Option<GlobalKeyAction> {
    lookup_global_key_with_modifiers(registry, key, KeyModifiers::NONE)
}

/// Look up a modified key in a registry.
pub fn lookup_global_key_with_modifiers(
    registry: &[GlobalKeyBinding],
    key: KeyCode,
    modifiers: KeyModifiers,
) -> Option<GlobalKeyAction> {
    registry
        .iter()
        .find(|binding| binding.key == key && binding.modifiers == modifiers)
        .map(|binding| binding.action)
}

/// Dispatch a top-level global key. Returns `true` when consumed.
pub async fn dispatch_global_key(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    cwd: &Path,
    key: KeyEvent,
) -> ArgoResult<bool> {
    let Some(action) =
        lookup_global_key_with_modifiers(&default_global_keys(), key.code, key.modifiers)
    else {
        return Ok(false);
    };
    match action {
        GlobalKeyAction::HelpOverlay => state.help_overlay = true,
        GlobalKeyAction::KeybindingsHelpOverlay => {
            state.keybindings_help_overlay = !state.keybindings_help_overlay;
        }
        GlobalKeyAction::StoreBackedOverlay => {
            return crate::input_overlay_dispatch::try_open_overlay(state, store, cwd, key.code)
                .await;
        }
        GlobalKeyAction::HStackOverlay => {
            return Ok(crate::input_h_stack::try_open_h_stack_overlay(state, key.code).await);
        }
        GlobalKeyAction::ReplayOverlay => crate::input_replay::open_replay_overlay(state),
        GlobalKeyAction::DashboardOverlay => crate::input_dashboard::open_dashboard_overlay(state),
        GlobalKeyAction::SearchEverywhereOverlay => {
            crate::search_everywhere::run_search_overlay(terminal, state, store).await?;
        }
        GlobalKeyAction::SpendMonitorOverlay => {
            crate::spend_monitor::open_spend_monitor(state, store).await?;
        }
        GlobalKeyAction::ApprovalTrayOverlay => {
            crate::approval_tray::open_approval_tray(state).await;
        }
    }
    Ok(true)
}

#[cfg(test)]
#[path = "input_globals_tests.rs"]
mod input_globals_tests;
