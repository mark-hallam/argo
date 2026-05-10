//! Per-tab rendering for [`SessionDetailPane`]. Split out of
//! `session_detail.rs` to keep the host file under the 500-LOC
//! ceiling. The orchestrating [`SessionDetailPane::render`] still
//! lives next door; this file provides the per-tab `draw_*` methods
//! plus three module-private text helpers.
use argo_store::MessageRow;
use argo_types::SessionId;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::session_detail::{SessionDetailPane, Tab};

impl SessionDetailPane {
    pub(crate) fn draw_tab_bar(&self, frame: &mut Frame<'_>, area: Rect) {
        let mut spans: Vec<Span<'_>> = Vec::with_capacity(Tab::ALL.len() * 2);
        for (idx, t) in Tab::ALL.iter().enumerate() {
            let label = format!(" {} ", t.label());
            let style = if t.index() == self.tab.index() {
                Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().add_modifier(Modifier::DIM)
            };
            spans.push(Span::styled(label, style));
            if idx + 1 < Tab::ALL.len() {
                spans.push(Span::raw("·"));
            }
        }
        let title = format!(
            " session {} — Tab/Shift-Tab cycle · q/Esc closes ",
            short_id(&self.session_id)
        );
        let para = Paragraph::new(Line::from(spans))
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(para, area);
    }

    pub(crate) fn draw_messages(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = format!(" Messages ({}) — j/k scroll ", self.messages.len());
        if self.messages.is_empty() {
            draw_empty(frame, area, &title, "(no messages on this session)");
            return;
        }
        let scroll = self.scroll.min(self.messages.len().saturating_sub(1));
        let lines: Vec<Line<'_>> = self
            .messages
            .iter()
            .skip(scroll)
            .map(|m| {
                Line::from(vec![
                    Span::raw(format!("[{:>3}] ", m.seq)),
                    Span::styled(
                        format!("{:<10}", m.role),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                    Span::raw(message_summary(m)),
                ])
            })
            .collect();
        let para = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(para, area);
    }

    pub(crate) fn draw_audit(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = format!(" Audit ({}) — j/k scroll ", self.audit.len());
        if self.audit.is_empty() {
            draw_empty(frame, area, &title, "(no audit rows for this session)");
            return;
        }
        let scroll = self.scroll.min(self.audit.len().saturating_sub(1));
        let mut lines = Vec::with_capacity(self.audit.len() - scroll + 1);
        lines.push(Line::from(format!(
            " {:>6}  {:<22} {}",
            "seq", "kind", "payload (truncated)"
        )));
        for r in self.audit.iter().skip(scroll) {
            let preview: String = r
                .payload_json
                .chars()
                .filter(|c| !matches!(c, '\n' | '\r'))
                .take(60)
                .collect();
            lines.push(Line::from(format!(
                " {:>6}  {:<22} {}",
                r.seq, r.kind, preview,
            )));
        }
        let para = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(para, area);
    }

    pub(crate) fn draw_cost(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = " Cost (derived) ".to_string();
        let lines = vec![
            Line::from("  session cost summary"),
            Line::from(""),
            Line::from(format!(
                "    messages:        {:>10}",
                self.cost.message_count
            )),
            Line::from(format!(
                "    content chars:   {:>10}",
                self.cost.total_chars
            )),
            Line::from(format!(
                "    spend (USD):     ${:>10.4}",
                self.cost.spend_usd()
            )),
            Line::from(""),
            Line::from("  (precise rollup: $ overlay / `argo costs show`)"),
        ];
        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(para, area);
    }

    pub(crate) fn draw_skills(&self, frame: &mut Frame<'_>, area: Rect) {
        let title = format!(" Skills ({}) — j/k scroll ", self.skills.len());
        if self.skills.is_empty() {
            draw_empty(frame, area, &title, "(no active skills installed)");
            return;
        }
        let scroll = self.scroll.min(self.skills.len().saturating_sub(1));
        let mut lines = Vec::with_capacity(self.skills.len() - scroll + 1);
        lines.push(Line::from(format!(
            " {:<20} {:<10} {}",
            "name", "version", "hash (12)",
        )));
        for s in self.skills.iter().skip(scroll) {
            let hash_short: String = hex::encode(s.hash).chars().take(12).collect();
            lines.push(Line::from(format!(
                " {:<20} {:<10} {}",
                s.frontmatter.name, s.frontmatter.version, hash_short,
            )));
        }
        let para = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(title));
        frame.render_widget(para, area);
    }
}

fn draw_empty(frame: &mut Frame<'_>, area: Rect, title: &str, msg: &str) {
    let para = Paragraph::new(Line::from(format!("  {msg}")))
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(para, area);
}

fn short_id(sid: &SessionId) -> String {
    sid.0.to_string().chars().take(8).collect()
}

fn message_summary(m: &MessageRow) -> String {
    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&m.content_json) {
        if let Some(blocks) = arr.as_array() {
            for b in blocks {
                if let Some(text) = b.get("text").and_then(|v| v.as_str()) {
                    if let Some(line) = text.lines().next() {
                        return line.chars().take(80).collect();
                    }
                }
            }
        }
    }
    m.content_json.chars().take(80).collect()
}
