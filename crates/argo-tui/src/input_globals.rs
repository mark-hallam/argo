//! Global key dispatch for top-level TUI shortcuts.
//!
//! This module owns keybindings that are available from the normal
//! session viewer, after modal text inputs and overlays have had a
//! chance to consume the key. New global overlays should register
//! here rather than adding another top-level arm to `input.rs`.
use crate::app::ViewerState;
use argo_store::SessionStore;
use argo_types::ArgoResult;
use crossterm::event::KeyCode;
use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Action invoked by a registered global key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlobalKeyAction {
    /// Open the help overlay.
    HelpOverlay,
    /// Delegate to the store-backed overlay dispatcher.
    StoreBackedOverlay,
    /// Delegate to the H-stack overlay dispatcher.
    HStackOverlay,
    /// Open the replay overlay.
    ReplayOverlay,
    /// Open the dashboard overlay.
    DashboardOverlay,
}

/// One global keybinding entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlobalKeyBinding {
    /// Key that triggers the action.
    pub key: KeyCode,
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
    if registry.iter().any(|binding| binding.key == key) {
        return Err(DuplicateGlobalKey { key });
    }
    registry.push(GlobalKeyBinding { key, action });
    Ok(())
}

/// Return the built-in global key registry used by the TUI.
pub fn default_global_keys() -> Vec<GlobalKeyBinding> {
    let mut registry = Vec::new();
    for (key, action) in [
        (KeyCode::Char('?'), GlobalKeyAction::HelpOverlay),
        (KeyCode::Char('F'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('S'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('$'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('K'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('P'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('T'), GlobalKeyAction::StoreBackedOverlay),
        (KeyCode::Char('A'), GlobalKeyAction::StoreBackedOverlay),
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
    registry
}

/// Look up a key in a registry.
pub fn lookup_global_key(registry: &[GlobalKeyBinding], key: KeyCode) -> Option<GlobalKeyAction> {
    registry
        .iter()
        .find(|binding| binding.key == key)
        .map(|binding| binding.action)
}

/// Dispatch a top-level global key. Returns `true` when consumed.
pub async fn dispatch_global_key(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    cwd: &Path,
    key: KeyCode,
) -> ArgoResult<bool> {
    let Some(action) = lookup_global_key(&default_global_keys(), key) else {
        return Ok(false);
    };
    match action {
        GlobalKeyAction::HelpOverlay => state.help_overlay = true,
        GlobalKeyAction::StoreBackedOverlay => {
            return crate::input_overlay_dispatch::try_open_overlay(state, store, cwd, key).await;
        }
        GlobalKeyAction::HStackOverlay => {
            return Ok(crate::input_h_stack::try_open_h_stack_overlay(state, key).await);
        }
        GlobalKeyAction::ReplayOverlay => crate::input_replay::open_replay_overlay(state),
        GlobalKeyAction::DashboardOverlay => crate::input_dashboard::open_dashboard_overlay(state),
    }
    Ok(true)
}

#[cfg(test)]
#[path = "input_globals_tests.rs"]
mod input_globals_tests;
