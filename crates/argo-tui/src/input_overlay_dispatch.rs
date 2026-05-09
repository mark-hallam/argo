//! Overlay open + close dispatchers extracted from `input.rs` to
//! keep that file under the 500-LOC ceiling and free headroom for
//! new key bindings. Each function returns `true` when the key
//! was consumed so the caller can `continue` the event loop.
//!
//! No behaviour change — the bodies match the original arms 1:1,
//! just relocated. Mirrors the convention already established by
//! `try_close_aux_overlay` in `lib.rs`.
use crate::app::ViewerState;
use crate::input_helpers::{handle_scroll_overlay, scan_dir_basenames};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use crossterm::event::KeyCode;
use std::path::Path;
use std::sync::Arc;

/// Try to close any open list/scroll overlay for `key_code`.
/// Returns `true` if one of the overlays was open (caller should
/// `continue` regardless of whether the key was a close key —
/// matches the pre-split behaviour where each branch ended in
/// `continue;`).
pub(crate) fn try_close_overlay(state: &mut ViewerState, key_code: KeyCode) -> bool {
    if let Some(rows) = state.forge_runs.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['F'], &mut state.forge_runs_scroll, n) {
            state.forge_runs = None;
            state.forge_runs_scroll = 0;
        }
        return true;
    }
    if let Some(rows) = state.skills_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['S'], &mut state.skills_overlay_scroll, n) {
            state.skills_overlay = None;
            state.skills_overlay_scroll = 0;
        }
        return true;
    }
    if let Some(rows) = state.memory_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['M'], &mut state.memory_overlay_scroll, n) {
            state.memory_overlay = None;
            state.memory_overlay_scroll = 0;
        }
        return true;
    }
    if state.cost_overlay.is_some() {
        if matches!(
            key_code,
            KeyCode::Esc | KeyCode::Char('$') | KeyCode::Char('q')
        ) {
            state.cost_overlay = None;
        }
        return true;
    }
    if let Some(rows) = state.audit_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['A'], &mut state.audit_overlay_scroll, n) {
            state.audit_overlay = None;
            state.audit_overlay_scroll = 0;
        }
        return true;
    }
    if let Some(rows) = state.cron_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['K'], &mut state.cron_overlay_scroll, n) {
            state.cron_overlay = None;
            state.cron_overlay_scroll = 0;
        }
        return true;
    }
    if let Some(rows) = state.providers_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['P'], &mut state.providers_overlay_scroll, n) {
            state.providers_overlay = None;
            state.providers_overlay_scroll = 0;
        }
        return true;
    }
    if let Some(rows) = state.tools_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['T'], &mut state.tools_overlay_scroll, n) {
            state.tools_overlay = None;
            state.tools_overlay_scroll = 0;
        }
        return true;
    }
    if let Some(rows) = state.tool_errors_overlay.as_ref() {
        let n = rows.len();
        if handle_scroll_overlay(key_code, &['E'], &mut state.tool_errors_overlay_scroll, n) {
            state.tool_errors_overlay = None;
            state.tool_errors_overlay_scroll = 0;
        }
        return true;
    }
    false
}

/// Try to open a store-backed overlay for `key_code`. Returns
/// `Ok(true)` when handled. Modifier-key, navigation, mode-switch,
/// and aux-overlay (h_stack/replay/dashboard) arms stay in the
/// caller's match — those don't fit the open-an-overlay shape.
pub(crate) async fn try_open_overlay(
    state: &mut ViewerState,
    store: &Arc<SessionStore>,
    cwd: &Path,
    key_code: KeyCode,
) -> ArgoResult<bool> {
    match key_code {
        KeyCode::Char('F') => {
            // Errors → empty list (UI shows
            // "(no forge runs recorded yet)").
            let rows = store.list_forge_runs(50).await.unwrap_or_default();
            state.forge_runs = Some(rows);
        }
        KeyCode::Char('S') => {
            let skills = argo_skill::SkillStore::new(Arc::clone(store));
            let rows = skills.list_active().await.unwrap_or_default();
            state.skills_overlay = Some(rows);
        }
        KeyCode::Char('$') => {
            // Snapshot taken once at open — re-press `$` to refresh.
            let snap = store.load_global_spend().await.unwrap_or_default();
            state.cost_overlay = Some(snap);
        }
        KeyCode::Char('K') => {
            state.cron_overlay = Some(scan_dir_basenames(
                &cwd.join("cron").join("jobs"),
                "json",
            ));
        }
        KeyCode::Char('P') => {
            state.providers_overlay = Some(scan_dir_basenames(
                &cwd.join("manifests").join("providers"),
                "toml",
            ));
        }
        KeyCode::Char('T') => {
            // Snapshot the in-core tool registry. Names only — full
            // schemas are available via the gateway's GET /tools.
            let mut reg = argo_tool::ToolRegistry::new();
            for t in argo_tool::builtin_tools(cwd.to_path_buf()) {
                reg.register(t);
            }
            let mut names: Vec<String> = reg.schemas().into_iter().map(|s| s.name).collect();
            names.sort();
            state.tools_overlay = Some(names);
        }
        KeyCode::Char('A') => {
            // No selection → empty overlay so the user can tell
            // why nothing rendered.
            let rows = if let Some(s) = state.selected_session() {
                store
                    .list_audit_for_session(s.id)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            state.audit_overlay = Some(rows);
        }
        KeyCode::Char('E') => {
            let rows = argo_store::tool_errors_histogram(store.pool())
                .await
                .unwrap_or_default();
            state.tool_errors_overlay = Some(rows);
        }
        KeyCode::Char('M') => {
            // Active-memory rows across all 4 canonical files. No
            // workspace → empty list.
            let mut all = Vec::new();
            if let Ok(Some(ws)) = store.first_workspace().await {
                let memory = argo_memory::MemoryStore::new(Arc::clone(store));
                for f in [
                    argo_memory::MemoryFile::Memory,
                    argo_memory::MemoryFile::User,
                    argo_memory::MemoryFile::Soul,
                    argo_memory::MemoryFile::Agents,
                ] {
                    if let Ok(rows) = memory.list_active(ws, f).await {
                        all.extend(rows);
                    }
                }
            }
            state.memory_overlay = Some(all);
        }
        _ => return Ok(false),
    }
    Ok(true)
}
