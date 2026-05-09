//! Navigation impls for [`ViewerState`] — pane focus, message-pane
//! scrolling, session-list filter + selection. Split out of
//! `app.rs` (claude-tui-app-split) to keep that file under the
//! 500-LOC HARD RULE 6 ceiling. Pure refactor; no behaviour
//! change. Methods retain their original visibility so existing
//! call sites are unchanged.
use argo_store::SessionHeader;

use crate::app::{PaneFocus, ViewerState};

impl ViewerState {
    /// Toggle the navigation focus between the session list and
    /// the messages pane. Resets the messages-scroll offset on
    /// transitions back to Sessions so the next switch starts at
    /// the top.
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            PaneFocus::Sessions => PaneFocus::Messages,
            PaneFocus::Messages => {
                self.messages_scroll = 0;
                PaneFocus::Sessions
            }
        };
    }

    /// Bump the messages scroll down by one row (clamped at the
    /// last visible row).
    pub fn scroll_messages_down(&mut self) {
        let max = self.selected_messages.len().saturating_sub(1);
        if self.messages_scroll < max {
            self.messages_scroll += 1;
        }
    }

    /// Bump the messages scroll up by one row.
    pub fn scroll_messages_up(&mut self) {
        self.messages_scroll = self.messages_scroll.saturating_sub(1);
    }

    /// Scroll messages by `step` rows (positive = down, clamped
    /// at the last row). Used by Page Down + the `G` jump.
    pub fn scroll_messages_down_n(&mut self, step: usize) {
        let max = self.selected_messages.len().saturating_sub(1);
        self.messages_scroll = (self.messages_scroll + step).min(max);
    }

    /// Scroll messages up by `step` rows (clamped at zero).
    pub fn scroll_messages_up_n(&mut self, step: usize) {
        self.messages_scroll = self.messages_scroll.saturating_sub(step);
    }

    /// Snap messages scroll to the top.
    pub fn scroll_messages_first(&mut self) {
        self.messages_scroll = 0;
    }

    /// Snap messages scroll to the last row.
    pub fn scroll_messages_last(&mut self) {
        self.messages_scroll = self.selected_messages.len().saturating_sub(1);
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

    pub(crate) fn invalidate(&mut self) {
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
