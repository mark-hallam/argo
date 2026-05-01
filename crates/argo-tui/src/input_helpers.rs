//! Pure-state + read-only helpers for the input event loop.
//! Lifted out of `input.rs` so that file stays under the
//! 500-LOC budget. Each fn here is independently testable.
use crate::ViewerState;
use argo_store::SessionStore;
use crossterm::event::KeyCode;
use std::sync::Arc;

/// One keystroke for the message-detail overlay. Pure state.
pub(crate) fn handle_message_overlay(code: KeyCode, state: &mut ViewerState) {
    match code {
        KeyCode::Esc | KeyCode::Char('o') | KeyCode::Char('q') => {
            state.message_overlay = None;
            state.message_overlay_text_only = false;
            state.message_overlay_scroll = 0;
        }
        KeyCode::Char('t') => {
            state.message_overlay_text_only = !state.message_overlay_text_only;
            state.message_overlay_scroll = 0;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            state.message_overlay_scroll = state.message_overlay_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            state.message_overlay_scroll = state.message_overlay_scroll.saturating_sub(1);
        }
        KeyCode::PageDown => {
            state.message_overlay_scroll = state.message_overlay_scroll.saturating_add(10);
        }
        KeyCode::PageUp => {
            state.message_overlay_scroll = state.message_overlay_scroll.saturating_sub(10);
        }
        KeyCode::Char('g') | KeyCode::Home => {
            state.message_overlay_scroll = 0;
        }
        _ => {}
    }
}

/// Re-fetch every open store-backed overlay. Errors silently
/// degrade to empty results — the overlay just shows stale
/// data on the next render rather than crashing.
pub(crate) async fn force_refresh_open_overlays(
    store: &Arc<SessionStore>,
    state: &mut ViewerState,
) {
    if state.forge_runs.is_some() {
        let rows = store.list_forge_runs(50).await.unwrap_or_default();
        state.forge_runs = Some(rows);
    }
    if state.skills_overlay.is_some() {
        let skills = argo_skill::SkillStore::new(Arc::clone(store));
        let rows = skills.list_active().await.unwrap_or_default();
        state.skills_overlay = Some(rows);
    }
    if state.cost_overlay.is_some() {
        let snap = store.load_global_spend().await.unwrap_or_default();
        state.cost_overlay = Some(snap);
    }
    if state.audit_overlay.is_some() {
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
    if state.memory_overlay.is_some() {
        let mut all = Vec::new();
        if let Ok(Some(ws)) = store.first_workspace().await {
            let mem = argo_memory::MemoryStore::new(Arc::clone(store));
            for f in [
                argo_memory::MemoryFile::Memory,
                argo_memory::MemoryFile::User,
                argo_memory::MemoryFile::Soul,
                argo_memory::MemoryFile::Agents,
            ] {
                if let Ok(rows) = mem.list_active(ws, f).await {
                    all.extend(rows);
                }
            }
        }
        state.memory_overlay = Some(all);
    }
}

/// Walk one directory and return every basename whose extension
/// matches `ext`, sorted lexically. Errors / missing dir → empty
/// vec (so an overlay just shows nothing instead of erroring out).
/// Shared between the K-key open path and the 1s tick refresh
/// for both the cron + providers overlays.
pub(crate) fn scan_dir_basenames(dir: &std::path::Path, ext: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().is_some_and(|x| x == ext) {
                if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                    names.push(stem.to_string());
                }
            }
        }
    }
    names.sort();
    names
}

/// Choose what `y` copies: prefer the most recent message body,
/// fall back to the selected session id when the messages pane
/// is empty.
pub(crate) fn pick_yank_text(state: &ViewerState) -> Option<String> {
    if let Some(last) = state.selected_messages.last() {
        if let Some(t) = extract_first_text(&last.content_json) {
            return Some(t);
        }
    }
    state.selected_session().map(|s| s.id.0.to_string())
}

/// Choose what `Y` copies: the message at the current
/// `messages_scroll` cursor (i.e. the topmost visible row in
/// the messages pane). Falls back to the most-recent yank
/// behaviour when nothing's scrolled or the cursor is out of
/// range.
pub(crate) fn pick_yank_at_cursor(state: &ViewerState) -> Option<String> {
    let idx = state.messages_scroll;
    if let Some(row) = state.selected_messages.get(idx) {
        if let Some(t) = extract_first_text(&row.content_json) {
            return Some(t);
        }
    }
    pick_yank_text(state)
}

fn extract_first_text(content_json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(content_json).ok()?;
    parsed
        .as_array()
        .and_then(|arr| arr.iter().find_map(|b| b.get("text").and_then(|v| v.as_str())))
        .map(str::to_string)
}
