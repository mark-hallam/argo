//! State machine for the session viewer.
//!
//! The crossterm event-loop body lives in `input.rs`; the entry
//! point + terminal lifecycle + fetch helpers live in `viewer.rs`.
//! This file stays under 500 LOC by keeping only the `ViewerState`
//! type + its impls here.
use argo_store::{MessageRow, SessionHeader};
use argo_types::ArgoResult;
use tokio::task::JoinHandle;

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
    /// detail pane so the user can confirm the agent picked it up.
    pub last_sent: Option<String>,
    /// Background `argo_agent_runner::run_turn` task spawned when
    /// the user hit Enter in composer mode. The event loop checks
    /// `is_finished()` each tick; when it goes from `Some` to
    /// `None`, the assistant reply (and any tool dispatches) have
    /// already been persisted into the store and the auto-refresh
    /// path will pick them up.
    pub agent_handle: Option<JoinHandle<ArgoResult<()>>>,
    /// `Some(t)` while `agent_handle` is set, where `t` is the
    /// `Instant` at which it was spawned. Drives the elapsed-
    /// time indicator in the footer.
    pub agent_started_at: Option<std::time::Instant>,
    /// Last error string from a finished agent task, if any. The
    /// footer renders this so e.g. missing provider keys aren't
    /// silently swallowed.
    pub last_agent_error: Option<String>,
    /// Active rename buffer: `Some` in rename mode (entered with `r`).
    pub rename_buffer: Option<String>,
    /// Active steer buffer: `Some` in steer mode (entered with `s`).
    /// On Enter the contents are persisted as a system-role
    /// message prefixed with `[steer]`, mirroring `argo steer`.
    pub steer_buffer: Option<String>,
    /// `Some(sid)` while a pending-delete confirm is in flight
    /// (entered with `d`); next `y` deletes, anything else cancels.
    pub pending_delete: Option<argo_types::SessionId>,
    /// Live event log appended as the agent task emits
    /// `TurnStart` / `AssistantMessage` / `ToolCall` / `ToolResult`
    /// / `TurnEnd` chunks. The render layer surfaces the most
    /// recent few lines as a "stream" trail in the messages pane.
    /// Capped at [`MAX_LIVE_EVENTS`] entries.
    pub live_events: Vec<String>,
    /// Which pane currently captures j/k navigation. Tab toggles
    /// between the session list (default) and the messages pane.
    pub focus: PaneFocus,
    /// Scroll offset (in rows) into the messages pane. Honoured
    /// when `focus == Messages`.
    pub messages_scroll: usize,
    /// True while the user has the keybinding-help overlay open
    /// (entered with `?`). Any key while open closes it.
    pub help_overlay: bool,
    /// `Some(idx)` while the user has a message expanded (entered
    /// with `o` while focused on the messages pane); the index
    /// points into `selected_messages`. Esc / o / q closes.
    pub message_overlay: Option<usize>,
    /// When `true`, the message overlay renders the extracted
    /// content-block text values instead of the full pretty-
    /// printed JSON. Toggled with `t` while the overlay is open
    /// — useful for drilling into a tool_result without the
    /// JSON noise.
    pub message_overlay_text_only: bool,
    /// Vertical scroll offset (in rows) for the message-detail
    /// overlay. Bumped by j/k/PgDn/PgUp while the overlay is
    /// open.
    pub message_overlay_scroll: u16,
    /// Loaded forge_run rows when the user has the forge-runs
    /// overlay open (entered with `F`). `None` when closed.
    pub forge_runs: Option<Vec<argo_store::ForgeRunRow>>,
    /// Scroll offset (in rows) into the forge-runs overlay,
    /// honoured by `j`/`k`/`PgUp`/`PgDn` while it's open.
    pub forge_runs_scroll: usize,
    /// Loaded active-skill rows when the user has the skills
    /// overlay open (entered with `S`). `None` when closed.
    pub skills_overlay: Option<Vec<argo_skill::Skill>>,
    /// Scroll offset (in rows) into the skills overlay.
    pub skills_overlay_scroll: usize,
    /// Loaded active memory entries when the user has the memory
    /// overlay open (entered with `M`). `None` when closed.
    pub memory_overlay: Option<Vec<argo_memory::MemoryEntry>>,
    /// Scroll offset (in rows) into the memory overlay.
    pub memory_overlay_scroll: usize,
    /// Loaded cross-tenant spend rollup when the user has the
    /// cost overlay open (entered with `$`). `None` when closed.
    pub cost_overlay: Option<argo_store::GlobalSpend>,
    /// Loaded audit-chain rows for the selected session when
    /// the user has the audit overlay open (entered with `A`).
    /// `None` when closed.
    pub audit_overlay: Option<Vec<argo_store::AuditRow>>,
    /// Scroll offset (in rows) into the audit overlay. Honoured
    /// by `j`/`k`/`PgUp`/`PgDn` while the overlay is open.
    pub audit_overlay_scroll: usize,
    /// Provider ids found in `manifests/providers/*.toml` when
    /// the providers overlay is open (entered with `P`). Just
    /// the basenames — no manifest-parse load on render.
    pub providers_overlay: Option<Vec<String>>,
    /// Scroll offset (in rows) into the providers overlay.
    pub providers_overlay_scroll: usize,
    /// Cron job basenames found in `cron/jobs/*.json` when the
    /// cron overlay is open (entered with `K`). No JSON parse
    /// load — just the file basenames.
    pub cron_overlay: Option<Vec<String>>,
    /// Scroll offset (in rows) into the cron overlay.
    pub cron_overlay_scroll: usize,
    /// Registered in-core tool names when the tools overlay is
    /// open (entered with `T`). Snapshot of the ToolRegistry
    /// (built from `argo_tool::builtin_tools(cwd)`).
    pub tools_overlay: Option<Vec<String>>,
    /// Scroll offset (in rows) into the tools overlay.
    pub tools_overlay_scroll: usize,
    /// Tool-error histogram from the audit chain when the
    /// tool-errors overlay is open (entered with `E`). Each entry
    /// is `(tool_name, count)` ordered desc by count. Mirrors the
    /// gateway's `/audit/tool-errors` (#617).
    pub tool_errors_overlay: Option<Vec<(String, i64)>>,
    /// Scroll offset (in rows) into the tool-errors overlay.
    pub tool_errors_overlay_scroll: usize,
    /// Orchestrator-sessions panel state when the overlay is open
    /// (entered with `O`). Reads from `GET /orchestrator/sessions`
    /// — until H-orchestrator-routes lands the panel renders an
    /// "Gateway unreachable" hint cleanly.
    pub h_sessions_overlay: Option<crate::overlays_h_stack::PanelState>,
    /// Scroll offset (in rows) into the orchestrator-sessions overlay.
    pub h_sessions_overlay_scroll: usize,
    /// Orchestrator-handoffs panel state when the overlay is open
    /// (entered with `H`). Reads from
    /// `GET /orchestrator/handoffs?status=pending`.
    pub h_handoffs_overlay: Option<crate::overlays_h_stack::PanelState>,
    /// Scroll offset (in rows) into the orchestrator-handoffs overlay.
    pub h_handoffs_overlay_scroll: usize,
    /// Budget summary panel state when the overlay is open
    /// (entered with `B`). Reads from `GET /budget/summary`.
    pub h_budget_overlay: Option<crate::overlays_h_stack::PanelState>,
    /// Trajectory replay overlay state when the overlay is open
    /// (entered with `R`). Stream `claude-trajectory-replay-ui`.
    pub replay_overlay: Option<crate::overlays_replay::ReplayState>,
    /// Real-time dashboard overlay state when open (entered with
    /// `D`). Stream `claude-tui-realtime-dashboard`.
    pub dashboard_overlay: Option<crate::overlays_dashboard::DashboardState>,
    /// Name of the workspace this TUI session is viewing. Set
    /// once at startup via `SessionStore::first_workspace_name`.
    /// Rendered as a `[ws=<name>]` prefix in the footer keymap
    /// so the user always knows which workspace is in focus.
    pub workspace_label: Option<String>,
}

/// Which pane has navigation focus inside the TUI. When focus is
/// on the session list, `j/k` change selection (the existing
/// behaviour); when focus is on the messages pane, `j/k` scroll
/// the message rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaneFocus {
    /// j/k change session selection.
    #[default]
    Sessions,
    /// j/k scroll the messages pane.
    Messages,
}

/// Hard cap on the live-event ring so a long agent run can't
/// blow up TUI memory. Bumped 32 → 200 in #301 — with `C` now
/// clearing the trail on demand (#300), the previous cap was
/// unnecessarily aggressive: a multi-tool turn would scroll
/// `TurnStart` and the early `ToolCall` events out before the
/// run finished.
pub const MAX_LIVE_EVENTS: usize = 200;

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
            agent_handle: None,
            agent_started_at: None,
            last_agent_error: None,
            live_events: Vec::new(),
            rename_buffer: None,
            steer_buffer: None,
            pending_delete: None,
            focus: PaneFocus::Sessions,
            messages_scroll: 0,
            help_overlay: false,
            message_overlay: None,
            message_overlay_text_only: false,
            message_overlay_scroll: 0,
            forge_runs: None,
            forge_runs_scroll: 0,
            skills_overlay: None,
            skills_overlay_scroll: 0,
            memory_overlay: None,
            memory_overlay_scroll: 0,
            cost_overlay: None,
            audit_overlay: None,
            audit_overlay_scroll: 0,
            providers_overlay: None,
            providers_overlay_scroll: 0,
            cron_overlay: None,
            cron_overlay_scroll: 0,
            tools_overlay: None,
            tools_overlay_scroll: 0,
            tool_errors_overlay: None,
            tool_errors_overlay_scroll: 0,
            h_sessions_overlay: None,
            h_sessions_overlay_scroll: 0,
            h_handoffs_overlay: None,
            h_handoffs_overlay_scroll: 0,
            h_budget_overlay: None,
            replay_overlay: None,
            dashboard_overlay: None,
            workspace_label: None,
        }
    }

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

    // Rename + steer text-input methods live in `text_input.rs`.

    /// `true` while a spawned `run_turn` task is still in flight.
    #[must_use]
    pub fn agent_running(&self) -> bool {
        self.agent_handle.is_some()
    }

    /// Append a `RunEvent` envelope (one JSON value per call)
    /// formatted as a one-line summary. Capped at
    /// [`MAX_LIVE_EVENTS`]; oldest entries are dropped from the
    /// front when the cap is reached.
    pub fn push_live_event(&mut self, env: &serde_json::Value) {
        let line = crate::live_event::format_live_event(env);
        self.live_events.push(line);
        if self.live_events.len() > MAX_LIVE_EVENTS {
            let overflow = self.live_events.len() - MAX_LIVE_EVENTS;
            self.live_events.drain(..overflow);
        }
    }

    /// Reset the live-event ring (called when a new agent turn
    /// is spawned so old chunks don't bleed into the new run).
    pub fn clear_live_events(&mut self) {
        self.live_events.clear();
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

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;


