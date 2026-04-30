//! Event loop + state machine for the session viewer.
use crate::render::draw;
use argo_store::{MessageRow, SessionHeader, SessionStore};
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};
use std::time::Duration;

/// Viewer state — sessions list, current selection, the detail-
/// pane payload for the selected session, and a lazily-fetched
/// message preview list.
pub struct ViewerState {
    /// Loaded session headers (reverse-chronological).
    pub sessions: Vec<SessionHeader>,
    /// Currently-selected index into `sessions`.
    pub selected: usize,
    /// Message count for the selected session (lazily fetched).
    pub selected_message_count: Option<u64>,
    /// First few messages for the selected session, lazily fetched
    /// on selection change. Used by the right pane's preview list.
    pub selected_messages: Vec<MessageRow>,
    /// Whether the messages list has been fetched for the current
    /// selection (so we don't re-query when nothing changed).
    pub messages_loaded: bool,
}

impl ViewerState {
    /// Build state from a freshly-loaded session list.
    #[must_use]
    pub fn new(sessions: Vec<SessionHeader>) -> Self {
        Self {
            sessions,
            selected: 0,
            selected_message_count: None,
            selected_messages: Vec::new(),
            messages_loaded: false,
        }
    }

    /// Move the selection down (if not already at the end).
    pub fn next(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let max_idx = self.sessions.len() - 1;
        if self.selected < max_idx {
            self.selected += 1;
            self.invalidate();
        }
    }

    /// Move the selection up.
    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.invalidate();
        }
    }

    /// Jump down by `step` (clamped at the end).
    pub fn next_n(&mut self, step: usize) {
        if self.sessions.is_empty() {
            return;
        }
        let max_idx = self.sessions.len() - 1;
        let new = (self.selected + step).min(max_idx);
        if new != self.selected {
            self.selected = new;
            self.invalidate();
        }
    }

    /// Jump up by `step` (clamped at zero).
    pub fn prev_n(&mut self, step: usize) {
        let new = self.selected.saturating_sub(step);
        if new != self.selected {
            self.selected = new;
            self.invalidate();
        }
    }

    /// Jump to the first session.
    pub fn first(&mut self) {
        if self.selected != 0 {
            self.selected = 0;
            self.invalidate();
        }
    }

    /// Jump to the last session.
    pub fn last(&mut self) {
        if self.sessions.is_empty() {
            return;
        }
        let max_idx = self.sessions.len() - 1;
        if self.selected != max_idx {
            self.selected = max_idx;
            self.invalidate();
        }
    }

    fn invalidate(&mut self) {
        self.selected_message_count = None;
        self.selected_messages.clear();
        self.messages_loaded = false;
    }

    /// Currently-selected session, if any.
    #[must_use]
    pub fn selected_session(&self) -> Option<&SessionHeader> {
        self.sessions.get(self.selected)
    }
}

/// Drive the TUI event loop. Quits on `q` or `Ctrl-C`. The
/// returned result is `Ok(())` on a clean quit, `Err` if terminal
/// setup or rendering blew up.
///
/// # Errors
/// Returns [`ArgoError::Provider`] for terminal setup, draw, or
/// store-query failures.
pub async fn run_session_viewer(store: &SessionStore) -> ArgoResult<()> {
    let sessions = store.list_all_sessions(200).await?;
    let mut state = ViewerState::new(sessions);
    fetch_message_count(store, &mut state).await?;
    fetch_messages(store, &mut state).await?;
    let mut terminal = setup_terminal()?;
    let outcome = event_loop(&mut terminal, store, &mut state).await;
    teardown_terminal(&mut terminal)?;
    outcome
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    store: &SessionStore,
    state: &mut ViewerState,
) -> ArgoResult<()> {
    loop {
        terminal
            .draw(|frame| draw(frame, state))
            .map_err(|e| ArgoError::Provider(format!("tui draw: {e}")))?;
        // 200ms poll keeps the loop responsive without a busy-spin;
        // ratatui's recommended idiom is poll-then-read.
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
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    state.next();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    state.prev();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::PageDown => {
                    state.next_n(10);
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::PageUp => {
                    state.prev_n(10);
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::Home | KeyCode::Char('g') => {
                    state.first();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                KeyCode::End | KeyCode::Char('G') => {
                    state.last();
                    fetch_message_count(store, state).await?;
                    fetch_messages(store, state).await?;
                }
                _ => {}
            }
        }
    }
}

async fn fetch_message_count(store: &SessionStore, state: &mut ViewerState) -> ArgoResult<()> {
    if state.selected_message_count.is_some() {
        return Ok(());
    }
    let Some(session) = state.selected_session() else {
        return Ok(());
    };
    let count = store.message_count(SessionId(session.id.0)).await?;
    state.selected_message_count = Some(count);
    Ok(())
}

async fn fetch_messages(store: &SessionStore, state: &mut ViewerState) -> ArgoResult<()> {
    if state.messages_loaded {
        return Ok(());
    }
    let Some(session) = state.selected_session() else {
        return Ok(());
    };
    let mut rows = store.list_messages(SessionId(session.id.0)).await?;
    // Cap at 50 rows so the preview pane stays bounded.
    rows.truncate(50);
    state.selected_messages = rows;
    state.messages_loaded = true;
    Ok(())
}

fn setup_terminal() -> ArgoResult<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().map_err(|e| ArgoError::Provider(format!("enable_raw_mode: {e}")))?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)
        .map_err(|e| ArgoError::Provider(format!("enter_alt_screen: {e}")))?;
    Terminal::new(CrosstermBackend::new(out))
        .map_err(|e| ArgoError::Provider(format!("Terminal::new: {e}")))
}

fn teardown_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> ArgoResult<()> {
    disable_raw_mode().map_err(|e| ArgoError::Provider(format!("disable_raw_mode: {e}")))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|e| ArgoError::Provider(format!("leave_alt_screen: {e}")))?;
    terminal
        .show_cursor()
        .map_err(|e| ArgoError::Provider(format!("show_cursor: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{TenantId, WorkspaceId};
    use chrono::Utc;

    fn fake_session(title: &str) -> SessionHeader {
        SessionHeader {
            id: argo_types::SessionId::new(),
            tenant_id: TenantId::new(),
            workspace_id: WorkspaceId::new(),
            model: "mock".into(),
            provider: "mock".into(),
            title: Some(title.into()),
            started_at: Utc::now(),
            ended_at: None,
        }
    }

    #[test]
    fn next_clamps_at_end() {
        let mut s = ViewerState::new(vec![fake_session("a"), fake_session("b")]);
        s.next();
        assert_eq!(s.selected, 1);
        s.next();
        assert_eq!(s.selected, 1, "must not pass the last index");
    }

    #[test]
    fn prev_clamps_at_zero() {
        let mut s = ViewerState::new(vec![fake_session("a")]);
        s.prev();
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn empty_list_does_not_panic_on_navigation() {
        let mut s = ViewerState::new(Vec::new());
        s.next();
        s.prev();
        assert!(s.selected_session().is_none());
    }

    #[test]
    fn navigation_invalidates_cached_message_count() {
        let mut s = ViewerState::new(vec![fake_session("a"), fake_session("b")]);
        s.selected_message_count = Some(7);
        s.next();
        assert_eq!(s.selected_message_count, None);
    }

    #[test]
    fn next_n_clamps_at_end() {
        let mut s = ViewerState::new((0..5).map(|i| fake_session(&i.to_string())).collect());
        s.next_n(3);
        assert_eq!(s.selected, 3);
        s.next_n(99);
        assert_eq!(s.selected, 4);
    }

    #[test]
    fn prev_n_clamps_at_zero() {
        let mut s = ViewerState::new((0..5).map(|i| fake_session(&i.to_string())).collect());
        s.selected = 4;
        s.prev_n(3);
        assert_eq!(s.selected, 1);
        s.prev_n(99);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn first_and_last_jump() {
        let mut s = ViewerState::new((0..5).map(|i| fake_session(&i.to_string())).collect());
        s.last();
        assert_eq!(s.selected, 4);
        s.first();
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn first_and_last_no_op_on_empty() {
        let mut s = ViewerState::new(Vec::new());
        s.first();
        s.last();
        assert!(s.selected_session().is_none());
    }
}
