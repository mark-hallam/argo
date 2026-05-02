//! `E` tool-errors overlay (#645) — extracted from `overlays.rs`
//! once that file went over the 500-LOC budget on the new draw fn.
//! Sole responsibility: render the histogram of failed tool calls
//! sourced from `audit_record` rows where `kind = 'tool_result'
//! AND $.is_error = true`.
use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Tool-error histogram overlay. Triggered by `E`. Each row is
/// `(tool_name, count)` ordered desc — operationally important
/// because a non-empty list means at least one tool integration
/// is broken in the workspace. Mirrors the gateway's
/// `/audit/tool-errors` (#617). Empty list shows a "clean run"
/// hint. Scrollable; Esc/E/q close.
pub(crate) fn draw_tool_errors(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[(String, i64)],
    scroll: usize,
) {
    let w = area.width.min(60);
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from("    failures  tool"));
    if rows.is_empty() {
        lines.push(Line::from("  (no tool errors — clean run)"));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for (tool, n) in &rows[clamped..] {
            lines.push(Line::from(format!("  {n:>10}  {tool}")));
        }
    }
    let total: i64 = rows.iter().map(|(_, n)| *n).sum();
    let para = Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).title(format!(
            " tool errors ({total} failures across {} tool{}) — Esc/E/q closes ",
            rows.len(),
            if rows.len() == 1 { "" } else { "s" }
        )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}
