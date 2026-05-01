//! Crossterm event-loop for the TUI session viewer. Side-effecting
//! helpers (spawn agent turn, reap, delete, rename, clone) live in
//! the sibling `actions.rs` so this file stays under 500 LOC as
//! keybindings get added.
use crate::actions::{
    apply_delete, apply_rename, apply_steer, create_cloned_session, reap_agent, spawn_agent_turn,
};
use crate::app::{PaneFocus, ViewerState};
use crate::render::draw;
use crate::viewer::{fetch_message_count, fetch_messages};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::Stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedReceiver;

pub(crate) async fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: Arc<SessionStore>,
    cwd: PathBuf,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    let mut last_refresh = std::time::Instant::now();
    let mut events_rx: Option<UnboundedReceiver<serde_json::Value>> = None;
    loop {
        if let Some(rx) = events_rx.as_mut() {
            while let Ok(env) = rx.try_recv() {
                state.push_live_event(&env);
            }
        }
        if reap_agent(state).await {
            events_rx = None;
        }
        terminal
            .draw(|frame| draw(frame, state))
            .map_err(|e| ArgoError::Provider(format!("tui draw: {e}")))?;
        if last_refresh.elapsed() >= Duration::from_secs(1) {
            state.messages_loaded = false;
            fetch_messages(&store, state).await?;
            // While the forge-runs overlay is open, re-fetch the
            // 50-row window so an in-flight run's status flips
            // from "running" → "succeeded" without the user
            // having to close + re-open with `F`.
            if state.forge_runs.is_some() {
                let rows = store.list_forge_runs(50).await.unwrap_or_default();
                state.forge_runs = Some(rows);
            }
            last_refresh = std::time::Instant::now();
        }
        if !event::poll(Duration::from_millis(200))
            .map_err(|e| ArgoError::Provider(format!("tui poll: {e}")))?
        {
            continue;
        }
        let ev = event::read().map_err(|e| ArgoError::Provider(format!("tui read: {e}")))?;
        if let Event::Key(key) = ev {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if state.filter_mode {
                match key.code {
                    KeyCode::Esc => state.filter_clear(),
                    KeyCode::Enter => state.exit_filter_mode(),
                    KeyCode::Backspace => {
                        state.filter_pop();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.filter_push(c);
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    _ => {}
                }
                continue;
            }
            // Help overlay: any key closes it.
            if state.help_overlay {
                state.help_overlay = false;
                continue;
            }
            // Message-detail overlay: Esc/o/q close, t toggles
            // drill view, j/k/PgUp/PgDn scroll the body, other
            // keys ignored.
            if state.message_overlay.is_some() {
                match key.code {
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
                        state.message_overlay_scroll =
                            state.message_overlay_scroll.saturating_add(1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.message_overlay_scroll =
                            state.message_overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        state.message_overlay_scroll =
                            state.message_overlay_scroll.saturating_add(10);
                    }
                    KeyCode::PageUp => {
                        state.message_overlay_scroll =
                            state.message_overlay_scroll.saturating_sub(10);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.message_overlay_scroll = 0;
                    }
                    _ => {}
                }
                continue;
            }
            // Forge-runs overlay: Esc/F/q close, j/k/PgUp/PgDn
            // scroll. Loop body auto-refreshes the rows every
            // second so a CLI-side optimize ticks along live.
            if let Some(rows) = state.forge_runs.as_ref() {
                let n = rows.len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('F') | KeyCode::Char('q') => {
                        state.forge_runs = None;
                        state.forge_runs_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = n.saturating_sub(1);
                        if state.forge_runs_scroll < max {
                            state.forge_runs_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.forge_runs_scroll =
                            state.forge_runs_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        let max = n.saturating_sub(1);
                        state.forge_runs_scroll = (state.forge_runs_scroll + 10).min(max);
                    }
                    KeyCode::PageUp => {
                        state.forge_runs_scroll =
                            state.forge_runs_scroll.saturating_sub(10);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.forge_runs_scroll = 0;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        state.forge_runs_scroll = n.saturating_sub(1);
                    }
                    _ => {}
                }
                continue;
            }
            // Active-skills overlay: Esc/S/q close, j/k scroll.
            if let Some(rows) = state.skills_overlay.as_ref() {
                let n = rows.len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('S') | KeyCode::Char('q') => {
                        state.skills_overlay = None;
                        state.skills_overlay_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = n.saturating_sub(1);
                        if state.skills_overlay_scroll < max {
                            state.skills_overlay_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.skills_overlay_scroll =
                            state.skills_overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        let max = n.saturating_sub(1);
                        state.skills_overlay_scroll =
                            (state.skills_overlay_scroll + 10).min(max);
                    }
                    KeyCode::PageUp => {
                        state.skills_overlay_scroll =
                            state.skills_overlay_scroll.saturating_sub(10);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.skills_overlay_scroll = 0;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        state.skills_overlay_scroll = n.saturating_sub(1);
                    }
                    _ => {}
                }
                continue;
            }
            // Active-memory overlay: Esc/M/q close, j/k scroll.
            if let Some(rows) = state.memory_overlay.as_ref() {
                let n = rows.len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('M') | KeyCode::Char('q') => {
                        state.memory_overlay = None;
                        state.memory_overlay_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = n.saturating_sub(1);
                        if state.memory_overlay_scroll < max {
                            state.memory_overlay_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.memory_overlay_scroll =
                            state.memory_overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        let max = n.saturating_sub(1);
                        state.memory_overlay_scroll =
                            (state.memory_overlay_scroll + 10).min(max);
                    }
                    KeyCode::PageUp => {
                        state.memory_overlay_scroll =
                            state.memory_overlay_scroll.saturating_sub(10);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.memory_overlay_scroll = 0;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        state.memory_overlay_scroll = n.saturating_sub(1);
                    }
                    _ => {}
                }
                continue;
            }
            // Cost-rollup overlay: Esc/$/q close. Snapshot is
            // taken once at open — re-press `$` to refresh.
            if state.cost_overlay.is_some() {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Char('$') | KeyCode::Char('q')
                ) {
                    state.cost_overlay = None;
                }
                continue;
            }
            // Audit-chain overlay: Esc/A/q close, j/k/PgUp/PgDn
            // scroll, anything else is ignored. Snapshot at
            // open — re-press `A` (or `R` global) to refresh.
            if let Some(rows) = state.audit_overlay.as_ref() {
                let n = rows.len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('A') | KeyCode::Char('q') => {
                        state.audit_overlay = None;
                        state.audit_overlay_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = n.saturating_sub(1);
                        if state.audit_overlay_scroll < max {
                            state.audit_overlay_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.audit_overlay_scroll =
                            state.audit_overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::PageDown => {
                        let max = n.saturating_sub(1);
                        state.audit_overlay_scroll =
                            (state.audit_overlay_scroll + 10).min(max);
                    }
                    KeyCode::PageUp => {
                        state.audit_overlay_scroll =
                            state.audit_overlay_scroll.saturating_sub(10);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.audit_overlay_scroll = 0;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        state.audit_overlay_scroll = n.saturating_sub(1);
                    }
                    _ => {}
                }
                continue;
            }
            // Cron overlay: Esc/K/q close, j/k scroll.
            if let Some(rows) = state.cron_overlay.as_ref() {
                let n = rows.len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('K') | KeyCode::Char('q') => {
                        state.cron_overlay = None;
                        state.cron_overlay_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = n.saturating_sub(1);
                        if state.cron_overlay_scroll < max {
                            state.cron_overlay_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.cron_overlay_scroll =
                            state.cron_overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.cron_overlay_scroll = 0;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        state.cron_overlay_scroll = n.saturating_sub(1);
                    }
                    _ => {}
                }
                continue;
            }
            // Providers overlay: Esc/P/q close, j/k scroll.
            if let Some(rows) = state.providers_overlay.as_ref() {
                let n = rows.len();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('P') | KeyCode::Char('q') => {
                        state.providers_overlay = None;
                        state.providers_overlay_scroll = 0;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        let max = n.saturating_sub(1);
                        if state.providers_overlay_scroll < max {
                            state.providers_overlay_scroll += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.providers_overlay_scroll =
                            state.providers_overlay_scroll.saturating_sub(1);
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        state.providers_overlay_scroll = 0;
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        state.providers_overlay_scroll = n.saturating_sub(1);
                    }
                    _ => {}
                }
                continue;
            }
            // Pending-delete confirm: 'y' fires the delete, any
            // other key cancels.
            if state.pending_delete.is_some() {
                let sid = state.pending_delete.take();
                if matches!(key.code, KeyCode::Char('y')) {
                    if let Some(sid) = sid {
                        apply_delete(&store, state, sid).await;
                    }
                }
                continue;
            }
            // Rename mode: type freely, Enter persists, Esc cancels.
            if state.rename_mode() {
                match key.code {
                    KeyCode::Esc => state.rename_cancel(),
                    KeyCode::Enter => {
                        if let Some(new_title) = state.rename_take() {
                            apply_rename(&store, state, &new_title).await;
                        }
                    }
                    KeyCode::Backspace => state.rename_pop(),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.rename_push(c);
                    }
                    _ => {}
                }
                continue;
            }
            // Steer mode: same shape as rename, Enter persists a
            // system-role `[steer] <text>` message.
            if state.steer_mode() {
                match key.code {
                    KeyCode::Esc => state.steer_cancel(),
                    KeyCode::Enter => {
                        if let Some(text) = state.steer_take() {
                            apply_steer(&store, state, &text).await;
                        }
                    }
                    KeyCode::Backspace => state.steer_pop(),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.steer_push(c);
                    }
                    _ => {}
                }
                continue;
            }
            // Composer mode: Enter spawns a live agent turn.
            if state.composer_mode {
                match key.code {
                    KeyCode::Esc => state.composer_cancel(),
                    KeyCode::Enter => {
                        events_rx = spawn_agent_turn(&store, &cwd, state).await;
                    }
                    KeyCode::Backspace => state.composer_pop(),
                    KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                        state.composer_push(c);
                    }
                    _ => {}
                }
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Tab => state.toggle_focus(),
                KeyCode::Char('?') => state.help_overlay = true,
                KeyCode::Char('F') => {
                    // Load the forge_run rows once and stash on
                    // state. Errors → empty list (UI shows
                    // "(no forge runs recorded yet)").
                    let rows = store.list_forge_runs(50).await.unwrap_or_default();
                    state.forge_runs = Some(rows);
                }
                KeyCode::Char('C') => {
                    // Drop the in-memory live-event trail
                    // without spawning a new turn. Useful when
                    // the trail from an earlier run is cluttering
                    // the messages pane and the user just wants
                    // to focus on the persisted history.
                    state.clear_live_events();
                }
                KeyCode::Char('R') => {
                    // Force-refresh of every pane + any open
                    // overlay. Useful when a CLI in another
                    // terminal just changed something and the
                    // 1-Hz tick hasn't come round yet.
                    state.messages_loaded = false;
                    fetch_message_count(&store, state).await?;
                    fetch_messages(&store, state).await?;
                    if state.forge_runs.is_some() {
                        let rows = store.list_forge_runs(50).await.unwrap_or_default();
                        state.forge_runs = Some(rows);
                    }
                    if state.skills_overlay.is_some() {
                        let skills = argo_skill::SkillStore::new(Arc::clone(&store));
                        let rows = skills.list_active().await.unwrap_or_default();
                        state.skills_overlay = Some(rows);
                    }
                    if state.cost_overlay.is_some() {
                        let snap = store.load_global_spend().await.unwrap_or_default();
                        state.cost_overlay = Some(snap);
                    }
                    last_refresh = std::time::Instant::now();
                }
                KeyCode::Char('S') => {
                    // Load the active-skill rows. Errors → empty
                    // list (overlay shows "(no active skills)").
                    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
                    let rows = skills.list_active().await.unwrap_or_default();
                    state.skills_overlay = Some(rows);
                }
                KeyCode::Char('$') => {
                    // Snapshot cross-tenant spend rollup. Errors
                    // → zero-snapshot (overlay shows $0.00).
                    let snap = store.load_global_spend().await.unwrap_or_default();
                    state.cost_overlay = Some(snap);
                }
                KeyCode::Char('K') => {
                    // Read cron/jobs/*.json basenames from cwd.
                    let mut names: Vec<String> = Vec::new();
                    if let Ok(cwd) = std::env::current_dir() {
                        let dir = cwd.join("cron").join("jobs");
                        if let Ok(entries) = std::fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.extension().is_some_and(|x| x == "json") {
                                    if let Some(stem) =
                                        p.file_stem().and_then(|s| s.to_str())
                                    {
                                        names.push(stem.to_string());
                                    }
                                }
                            }
                        }
                    }
                    names.sort();
                    state.cron_overlay = Some(names);
                }
                KeyCode::Char('P') => {
                    // Read manifests/providers/*.toml basenames
                    // from cwd. Errors / missing dir → empty
                    // list (overlay says "no manifests found").
                    let mut ids: Vec<String> = Vec::new();
                    if let Ok(cwd) = std::env::current_dir() {
                        let dir = cwd.join("manifests").join("providers");
                        if let Ok(entries) = std::fs::read_dir(&dir) {
                            for entry in entries.flatten() {
                                let p = entry.path();
                                if p.extension().is_some_and(|x| x == "toml") {
                                    if let Some(stem) =
                                        p.file_stem().and_then(|s| s.to_str())
                                    {
                                        ids.push(stem.to_string());
                                    }
                                }
                            }
                        }
                    }
                    ids.sort();
                    state.providers_overlay = Some(ids);
                }
                KeyCode::Char('A') => {
                    // Audit chain for the selected session. No
                    // selection → empty overlay so the user can
                    // tell why nothing rendered.
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
                KeyCode::Char('M') => {
                    // Load the active-memory rows across all 4
                    // canonical files. No workspace → empty
                    // list (overlay shows "(no active memory)").
                    let mut all = Vec::new();
                    if let Ok(Some(ws)) = store.first_workspace().await {
                        let memory = argo_memory::MemoryStore::new(Arc::clone(&store));
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
                KeyCode::Char('o')
                    if matches!(state.focus, PaneFocus::Messages)
                        && !state.selected_messages.is_empty() =>
                {
                    // Open the highlighted message in a detail
                    // overlay. The "highlight" is the topmost
                    // visible row — i.e. `messages_scroll` — so
                    // page up/down + j/k work as the cursor.
                    state.message_overlay = Some(
                        state
                            .messages_scroll
                            .min(state.selected_messages.len() - 1),
                    );
                }
                KeyCode::Char('/') => state.enter_filter_mode(),
                KeyCode::Char('i') => state.composer_mode = true,
                KeyCode::Char('y') => {
                    if let Some(text) = pick_yank_text(state) {
                        crate::clipboard::write_to_clipboard(&text);
                    }
                }
                KeyCode::Char('n') => {
                    if let Err(e) = create_cloned_session(&store, &cwd, state).await {
                        state.last_agent_error = Some(format!("new session: {e}"));
                    }
                }
                KeyCode::Char('r') if state.selected_session().is_some() => {
                    state.enter_rename_mode();
                }
                KeyCode::Char('s') if state.selected_session().is_some() => {
                    state.enter_steer_mode();
                }
                KeyCode::Char('d') => {
                    if let Some(s) = state.selected_session() {
                        state.pending_delete = Some(SessionId(s.id.0));
                    }
                }
                KeyCode::Char('e') => {
                    // Mark the selected session as ended (stamps
                    // ended_at). Idempotent — re-pressing on an
                    // already-ended session re-stamps but causes
                    // no other behaviour change.
                    if let Some(s) = state.selected_session() {
                        let sid = SessionId(s.id.0);
                        if let Err(e) = store.end_session(sid).await {
                            state.last_agent_error = Some(format!("end_session: {e}"));
                        } else {
                            // Force a refresh so the marker
                            // (●→○) flips on the next draw.
                            state.sessions = store
                                .list_all_sessions(crate::viewer::MAX_SESSIONS_LOAD)
                                .await
                                .unwrap_or_default();
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => match state.focus {
                    PaneFocus::Sessions => {
                        state.next();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_down(),
                },
                KeyCode::Up | KeyCode::Char('k') => match state.focus {
                    PaneFocus::Sessions => {
                        state.prev();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_up(),
                },
                KeyCode::PageDown => match state.focus {
                    PaneFocus::Sessions => {
                        state.next_n(10);
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_down_n(10),
                },
                KeyCode::PageUp => match state.focus {
                    PaneFocus::Sessions => {
                        state.prev_n(10);
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_up_n(10),
                },
                KeyCode::Home | KeyCode::Char('g') => match state.focus {
                    PaneFocus::Sessions => {
                        state.first();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_first(),
                },
                KeyCode::End | KeyCode::Char('G') => match state.focus {
                    PaneFocus::Sessions => {
                        state.last();
                        fetch_message_count(&store, state).await?;
                        fetch_messages(&store, state).await?;
                    }
                    PaneFocus::Messages => state.scroll_messages_last(),
                },
                _ => {}
            }
        }
    }
}

/// Choose what `y` copies: prefer the most recent message body,
/// fall back to the selected session id when the messages pane
/// is empty.
fn pick_yank_text(state: &ViewerState) -> Option<String> {
    if let Some(last) = state.selected_messages.last() {
        let parsed: serde_json::Value =
            serde_json::from_str(&last.content_json).unwrap_or(serde_json::Value::Null);
        if let Some(t) = parsed
            .as_array()
            .and_then(|arr| arr.iter().find_map(|b| b.get("text").and_then(|v| v.as_str())))
        {
            return Some(t.to_string());
        }
    }
    state.selected_session().map(|s| s.id.0.to_string())
}
