//! `R` trajectory-replay overlay (Stream
//! `claude-trajectory-replay-ui`). Mirrors the existing
//! H-stack overlay pattern (modal popup, Esc/q closes,
//! j/k scrolls).
//!
//! Shows a scrollable left pane of trajectory steps + a right
//! pane with the selected step's content. Three step kinds are
//! rendered:
//! - `ComputerUse` — screenshot path + assistant rationale.
//!   Image protocols (sixel / iTerm) are not wired yet, so
//!   the pane gracefully degrades to "(screenshot at <path>)".
//! - `Mcp` — tool name + JSON args + result preview.
//! - `Human` — prompt + reply text.
//!
//! Data is supplied by the caller (the cli loads it via the
//! `argo action replay` data path); this module is renderer-only.
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use serde_json::Value;

/// One trajectory step. Stable wire shape so the cli + future
/// gateway endpoint can both feed the overlay.
#[derive(Debug, Clone)]
pub struct ReplayStep {
    /// 1-based step index within the action.
    pub idx: usize,
    /// Step kind label (`computer_use`, `mcp`, `human`).
    pub kind: String,
    /// Headline for the left-pane action list — e.g.
    /// `"click button #signup"` or `"tool: search_emails"`.
    pub headline: String,
    /// Free-form body for the right pane. Rendered as text;
    /// the CU variant inserts a "(screenshot at ...)" line
    /// when `screenshot_path` is set.
    pub body: String,
    /// Optional screenshot path for ComputerUse steps.
    pub screenshot_path: Option<String>,
    /// Optional JSON payload (tool args / human reply) — pretty-
    /// printed under the body.
    pub payload: Option<Value>,
}

/// Replay overlay state. Tracks the selected step + an optional
/// search query for filtering.
#[derive(Debug, Clone, Default)]
pub struct ReplayState {
    /// All steps in chronological order.
    pub steps: Vec<ReplayStep>,
    /// Index into `steps` that's currently focused.
    pub selected: usize,
    /// Active search query (empty = no filter).
    pub search: String,
}

impl ReplayState {
    /// Build state from a step list. `selected = 0` initially.
    #[must_use]
    pub fn new(steps: Vec<ReplayStep>) -> Self {
        Self {
            steps,
            selected: 0,
            search: String::new(),
        }
    }

    /// Indices into `steps` that match the current search query.
    /// Empty `search` returns every index.
    #[must_use]
    pub fn filtered_indices(&self) -> Vec<usize> {
        if self.search.is_empty() {
            return (0..self.steps.len()).collect();
        }
        let q = self.search.to_lowercase();
        self.steps
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.headline.to_lowercase().contains(&q)
                    || s.body.to_lowercase().contains(&q)
                    || s.kind.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Move selection forward by one filtered step (clamped).
    pub fn step_next(&mut self) {
        let f = self.filtered_indices();
        if let Some(pos) = f.iter().position(|&i| i == self.selected) {
            if pos + 1 < f.len() {
                self.selected = f[pos + 1];
            }
        } else if let Some(&first) = f.first() {
            self.selected = first;
        }
    }

    /// Move selection backward by one filtered step (clamped).
    pub fn step_prev(&mut self) {
        let f = self.filtered_indices();
        if let Some(pos) = f.iter().position(|&i| i == self.selected) {
            if pos > 0 {
                self.selected = f[pos - 1];
            }
        } else if let Some(&first) = f.first() {
            self.selected = first;
        }
    }
}

/// Draw the replay overlay.
pub(crate) fn draw_replay(frame: &mut Frame<'_>, area: Rect, state: &ReplayState) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(9) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(60), h.max(20));
    frame.render_widget(Clear, panel);

    // Vertical: top bar, body (left+right), bottom bar.
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(panel);
    draw_top_bar(frame, outer[0], state);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Min(20)])
        .split(outer[1]);
    draw_action_list(frame, body[0], state);
    draw_step_detail(frame, body[1], state);
    draw_bottom_bar(frame, outer[2], state);
}

fn draw_top_bar(frame: &mut Frame<'_>, area: Rect, state: &ReplayState) {
    let total = state.steps.len();
    let cur = state
        .filtered_indices()
        .iter()
        .position(|&i| i == state.selected)
        .map(|p| p + 1)
        .unwrap_or(0);
    let title = format!(
        " trajectory replay — step {cur}/{total} {} ",
        if state.search.is_empty() {
            String::new()
        } else {
            format!("(search: {})", state.search)
        }
    );
    let para = Paragraph::new(Line::from(""))
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(para, area);
}

fn draw_action_list(frame: &mut Frame<'_>, area: Rect, state: &ReplayState) {
    let mut lines: Vec<Line> = Vec::new();
    for &i in &state.filtered_indices() {
        let s = &state.steps[i];
        let prefix = if i == state.selected { "▸ " } else { "  " };
        let text = format!("{prefix}[{:>3}] {:<14} {}", s.idx, s.kind, s.headline);
        let line = if i == state.selected {
            Line::from(text).style(Style::default().add_modifier(Modifier::BOLD))
        } else {
            Line::from(text)
        };
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(Line::from("  (no matching steps)"));
    }
    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" steps "));
    frame.render_widget(para, area);
}

fn draw_step_detail(frame: &mut Frame<'_>, area: Rect, state: &ReplayState) {
    let title = format!(
        " step {} — {} ",
        state
            .steps
            .get(state.selected)
            .map(|s| s.idx)
            .unwrap_or(0),
        state
            .steps
            .get(state.selected)
            .map(|s| s.kind.as_str())
            .unwrap_or("(empty)")
    );
    let body = render_step_body(state.steps.get(state.selected));
    let para = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

/// Render the right pane body for a step. Pure function so it's
/// unit-testable without spinning up a `TestBackend`.
#[must_use]
pub fn render_step_body(step: Option<&ReplayStep>) -> Vec<Line<'static>> {
    let Some(s) = step else {
        return vec![Line::from("(no step selected)")];
    };
    let mut lines = vec![Line::from(format!("kind:     {}", s.kind))];
    lines.push(Line::from(format!("headline: {}", s.headline)));
    if let Some(p) = &s.screenshot_path {
        // No sixel/iTerm support yet — gracefully degrade.
        lines.push(Line::from(format!("(screenshot at {p}; ASCII rendering not wired)")));
    }
    lines.push(Line::from(""));
    for body_line in s.body.lines() {
        lines.push(Line::from(body_line.to_string()));
    }
    if let Some(p) = &s.payload {
        lines.push(Line::from(""));
        lines.push(Line::from("payload:"));
        for body_line in serde_json::to_string_pretty(p)
            .unwrap_or_else(|_| "{}".to_string())
            .lines()
        {
            lines.push(Line::from(format!("  {body_line}")));
        }
    }
    lines
}

fn draw_bottom_bar(frame: &mut Frame<'_>, area: Rect, _state: &ReplayState) {
    let para = Paragraph::new(Line::from(
        "[←/→] step  [j/k] list  [/] search  [Esc/q] close",
    ))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(para, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn step(idx: usize, kind: &str, headline: &str, body: &str) -> ReplayStep {
        ReplayStep {
            idx,
            kind: kind.to_string(),
            headline: headline.to_string(),
            body: body.to_string(),
            screenshot_path: None,
            payload: None,
        }
    }

    #[test]
    fn filtered_indices_returns_all_when_search_empty() {
        let s = ReplayState::new(vec![step(1, "mcp", "a", ""), step(2, "human", "b", "")]);
        assert_eq!(s.filtered_indices(), vec![0, 1]);
    }

    #[test]
    fn filtered_indices_matches_kind_headline_or_body() {
        let mut s = ReplayState::new(vec![
            step(1, "mcp", "search emails", ""),
            step(2, "computer_use", "click button", "screenshot of inbox"),
            step(3, "human", "reply", "thanks"),
        ]);
        s.search = "EMAIL".to_string();
        assert_eq!(s.filtered_indices(), vec![0]);
        s.search = "screenshot".to_string();
        assert_eq!(s.filtered_indices(), vec![1]);
        s.search = "human".to_string();
        assert_eq!(s.filtered_indices(), vec![2]);
        s.search = "no match".to_string();
        assert!(s.filtered_indices().is_empty());
    }

    #[test]
    fn step_next_clamps_at_end() {
        let mut s = ReplayState::new(vec![step(1, "k", "a", ""), step(2, "k", "b", "")]);
        assert_eq!(s.selected, 0);
        s.step_next();
        assert_eq!(s.selected, 1);
        s.step_next();
        assert_eq!(s.selected, 1, "should clamp");
    }

    #[test]
    fn step_prev_clamps_at_start() {
        let mut s = ReplayState::new(vec![step(1, "k", "a", ""), step(2, "k", "b", "")]);
        s.selected = 1;
        s.step_prev();
        assert_eq!(s.selected, 0);
        s.step_prev();
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn step_navigation_skips_filtered_out_steps() {
        let mut s = ReplayState::new(vec![
            step(1, "mcp", "match", ""),
            step(2, "mcp", "other", ""),
            step(3, "mcp", "match", ""),
        ]);
        s.search = "match".to_string();
        s.selected = 0;
        s.step_next();
        // Filtered list is [0, 2] — next from 0 is 2.
        assert_eq!(s.selected, 2);
    }

    #[test]
    fn render_step_body_renders_cu_screenshot_placeholder() {
        let mut s = step(1, "computer_use", "click", "");
        s.screenshot_path = Some("/tmp/argo/shot.png".into());
        let lines = render_step_body(Some(&s));
        let joined: String = lines.iter().map(|l| l.to_string()).collect();
        assert!(joined.contains("/tmp/argo/shot.png"));
        assert!(joined.contains("ASCII rendering"));
    }

    #[test]
    fn render_step_body_renders_mcp_payload() {
        let mut s = step(2, "mcp", "search", "tool: search");
        s.payload = Some(json!({"query": "skates"}));
        let lines = render_step_body(Some(&s));
        let joined: String = lines.iter().map(|l| l.to_string()).collect();
        assert!(joined.contains("search"));
        assert!(joined.contains("skates"));
    }

    #[test]
    fn render_step_body_renders_human_text() {
        let s = step(3, "human", "approve", "approved by ops");
        let lines = render_step_body(Some(&s));
        let joined: String = lines.iter().map(|l| l.to_string()).collect();
        assert!(joined.contains("approve"));
        assert!(joined.contains("approved by ops"));
    }

    #[test]
    fn render_step_body_handles_none() {
        let lines = render_step_body(None);
        assert_eq!(lines.len(), 1);
    }
}
