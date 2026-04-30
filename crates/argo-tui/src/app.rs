//! State machine for the session viewer. The crossterm event-loop
//! body lives in `input.rs` so this file stays under 500 LOC.
use argo_store::{MessageRow, SessionHeader, SessionStore};
use argo_types::{ArgoError, ArgoResult, SessionId};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Stdout};

/// Viewer state — sessions list, current selection, the detail-
/// pane payload for the selected session, and a lazily-fetched
/// message preview list. `filter` + `filter_mode` drive the
/// `/`-style search-in-list (Vim convention).
pub struct ViewerState {
    /// Loaded session headers (reverse-chronological).
    pub sessions: Vec<SessionHeader>,
    /// Currently-selected index into the **filtered** view (see
    /// [`Self::filtered_indices`]).
    pub selected: usize,
    /// Message count for the selected session (lazily fetched).
    pub selected_message_count: Option<u64>,
    /// First few messages for the selected session, lazily fetched
    /// on selection change. Used by the right pane's preview list.
    pub selected_messages: Vec<MessageRow>,
    /// Whether the messages list has been fetched for the current
    /// selection (so we don't re-query when nothing changed).
    pub messages_loaded: bool,
    /// Active filter substring (empty = no filter).
    pub filter: String,
    /// `true` when the user is currently typing the filter (the
    /// composer pane in the footer captures keystrokes).
    pub filter_mode: bool,
    /// Draft text in the sticky composer pane. Cleared after
    /// "send" (currently a no-op staging slot — wiring it through
    /// to `argo chat` lands in a later PR; for now Enter just
    /// snapshots the draft into `last_sent`).
    pub composer: String,
    /// True while the user is editing the composer (vim-`i` mode).
    pub composer_mode: bool,
    /// Last "sent" composer draft (Enter snapshot). Visible in the
    /// detail pane so the user can confirm the round-trip will
    /// pick this up once live wiring lands.
    pub last_sent: Option<String>,
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
            filter: String::new(),
            filter_mode: false,
            composer: String::new(),
            composer_mode: false,
            last_sent: None,
        }
    }

    /// Append a char to the composer draft.
    pub fn composer_push(&mut self, c: char) {
        self.composer.push(c);
    }

    /// Pop the last char from the composer draft (Backspace).
    pub fn composer_pop(&mut self) {
        self.composer.pop();
    }

    /// Snapshot the draft into `last_sent` and clear it. Today the
    /// snapshot is purely visible — wiring it through to `argo
    /// chat` lands when the TUI gets a `SessionStore` handle.
    pub fn composer_send(&mut self) {
        if !self.composer.is_empty() {
            self.last_sent = Some(std::mem::take(&mut self.composer));
        }
        self.composer_mode = false;
    }

    /// Discard the draft and exit composer mode.
    pub fn composer_cancel(&mut self) {
        self.composer.clear();
        self.composer_mode = false;
    }

    /// Indices into `sessions` for rows matching `filter`. Empty
    /// filter returns every index in order.
    #[must_use]
    pub fn filtered_indices(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.sessions.len()).collect();
        }
        let needle = self.filter.to_lowercase();
        self.sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.title
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle)
                    || s.model.to_lowercase().contains(&needle)
                    || s.provider.to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Move the selection down (if not already at the end).
    pub fn next(&mut self) {
        let len = self.filtered_indices().len();
        if len == 0 {
            return;
        }
        if self.selected + 1 < len {
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
        let len = self.filtered_indices().len();
        if len == 0 {
            return;
        }
        let max_idx = len - 1;
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

    /// Jump to the first row.
    pub fn first(&mut self) {
        if self.selected != 0 {
            self.selected = 0;
            self.invalidate();
        }
    }

    /// Jump to the last row.
    pub fn last(&mut self) {
        let len = self.filtered_indices().len();
        if len == 0 {
            return;
        }
        let max_idx = len - 1;
        if self.selected != max_idx {
            self.selected = max_idx;
            self.invalidate();
        }
    }

    /// Enter filter-input mode (the footer becomes a composer that
    /// captures keystrokes).
    pub fn enter_filter_mode(&mut self) {
        self.filter_mode = true;
    }

    /// Leave filter-input mode (filter remains applied).
    pub fn exit_filter_mode(&mut self) {
        self.filter_mode = false;
    }

    /// Push a character into the filter; clamps selection back to
    /// 0 since the filtered list shape changed.
    pub fn filter_push(&mut self, c: char) {
        self.filter.push(c);
        self.selected = 0;
        self.invalidate();
    }

    /// Backspace one character from the filter.
    pub fn filter_pop(&mut self) {
        self.filter.pop();
        self.selected = 0;
        self.invalidate();
    }

    /// Clear the filter and exit filter mode.
    pub fn filter_clear(&mut self) {
        self.filter.clear();
        self.filter_mode = false;
        self.selected = 0;
        self.invalidate();
    }

    fn invalidate(&mut self) {
        self.selected_message_count = None;
        self.selected_messages.clear();
        self.messages_loaded = false;
    }

    /// Currently-selected session, honoring the filter.
    #[must_use]
    pub fn selected_session(&self) -> Option<&SessionHeader> {
        let indices = self.filtered_indices();
        let raw = indices.get(self.selected).copied()?;
        self.sessions.get(raw)
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
    let outcome = crate::input::run(&mut terminal, store, &mut state).await;
    teardown_terminal(&mut terminal)?;
    outcome
}


pub(crate) async fn fetch_message_count(store: &SessionStore, state: &mut ViewerState) -> ArgoResult<()> {
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

pub(crate) async fn fetch_messages(store: &SessionStore, state: &mut ViewerState) -> ArgoResult<()> {
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

    #[test]
    fn empty_filter_shows_all() {
        let s = ViewerState::new(vec![fake_session("morning"), fake_session("afternoon")]);
        assert_eq!(s.filtered_indices(), vec![0, 1]);
    }

    #[test]
    fn filter_substring_match() {
        let mut s = ViewerState::new(vec![
            fake_session("morning summary"),
            fake_session("afternoon stand-up"),
            fake_session("evening review"),
        ]);
        s.filter_push('m');
        s.filter_push('o');
        s.filter_push('r');
        assert_eq!(s.filtered_indices(), vec![0]);
    }

    #[test]
    fn filter_is_case_insensitive() {
        let mut s = ViewerState::new(vec![fake_session("Morning")]);
        s.filter_push('m');
        assert_eq!(s.filtered_indices(), vec![0]);
    }

    #[test]
    fn changing_filter_resets_selection() {
        let mut s = ViewerState::new(vec![
            fake_session("alpha"),
            fake_session("beta"),
            fake_session("gamma"),
        ]);
        s.selected = 2;
        s.filter_push('a');
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn selected_session_honors_filter() {
        let mut s = ViewerState::new(vec![
            fake_session("a-thing"),
            fake_session("b-thing"),
            fake_session("c-thing"),
        ]);
        s.filter_push('b');
        let title = s.selected_session().unwrap().title.as_deref().unwrap();
        assert_eq!(title, "b-thing");
    }

    #[test]
    fn filter_clear_returns_to_full_list() {
        let mut s = ViewerState::new(vec![fake_session("alpha"), fake_session("beta")]);
        s.filter_push('a');
        s.filter_clear();
        assert_eq!(s.filter, "");
        assert!(!s.filter_mode);
        assert_eq!(s.filtered_indices(), vec![0, 1]);
    }
}
