//! Modal overlay renderers — help cheatsheet (`?`) and message
//! detail (`o`). Lives in its own file so `render.rs` stays under
//! the 500-LOC budget.
use crate::app::ViewerState;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// Centered keybinding cheatsheet shown while
/// `state.help_overlay` is true. Any keypress closes it (handled
/// in `input.rs`).
pub(crate) fn draw_help(frame: &mut Frame<'_>, area: Rect) {
    let w = area.width.min(60);
    let h = area.height.min(16);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w, h);
    let body = vec![
        Line::from("  Navigation"),
        Line::from("    j/k        move down/up"),
        Line::from("    PgDn/PgUp  page down/up        g/G  top/bottom"),
        Line::from("    Tab        toggle pane focus (sessions / messages)"),
        Line::from(""),
        Line::from("  Session ops"),
        Line::from("    n  new (clone selected)        r  rename"),
        Line::from("    d  delete (y to confirm)       /  filter"),
        Line::from(""),
        Line::from("  Messaging"),
        Line::from("    i  compose (Enter sends)       s  steer (system msg)"),
        Line::from("    o  open msg detail (focused)   y  yank (OSC-52 copy)"),
        Line::from(""),
        Line::from("  Forge"),
        Line::from("    F  open recent forge_run rows (any key closes)"),
        Line::from(""),
        Line::from("  q / Esc  quit       ?  this help (any key closes)"),
    ];
    let para = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(" keybindings "))
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Recent forge_run rows. Triggered by `F` from anywhere
/// outside compose / rename / delete. Read-only — any keypress
/// closes the overlay.
pub(crate) fn draw_forge_runs(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[argo_store::ForgeRunRow],
) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(format!(
        " {:<8} {:<18} {:<8} {:<8} {:<6} {}",
        "id", "skill", "base", "candidate", "score", "status",
    )));
    if rows.is_empty() {
        lines.push(Line::from("  (no forge runs recorded yet)"));
    } else {
        for r in rows {
            let id_short: String = hex::encode(&r.id).chars().take(8).collect();
            let cand = r.candidate_version.clone().unwrap_or_else(|| "-".into());
            let score = r
                .final_score
                .map(|s| format!("{s:.3}"))
                .unwrap_or_else(|| "-".into());
            lines.push(Line::from(format!(
                " {:<8} {:<18} {:<8} {:<8} {:<6} {}",
                id_short, r.skill_name, r.base_version, cand, score, r.status,
            )));
        }
    }
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" forge runs ({}) — any key closes ", rows.len())),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Detail view for one persisted message — full content_json,
/// pretty-printed if it parses as JSON. Triggered by `o` while
/// focused on the messages pane.
pub(crate) fn draw_message(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &ViewerState,
    idx: usize,
) {
    let Some(msg) = state.selected_messages.get(idx) else {
        return;
    };
    let w = area.width.saturating_mul(8) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(20), h.max(6));

    let pretty = match serde_json::from_str::<serde_json::Value>(&msg.content_json) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| msg.content_json.clone()),
        Err(_) => msg.content_json.clone(),
    };
    let title = format!(
        " message [seq={} role={}]  (Esc/o/q closes) ",
        msg.seq, msg.role
    );
    let para = Paragraph::new(pretty)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}
