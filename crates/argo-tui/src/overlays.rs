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
        Line::from("    t  inside msg detail: toggle drill-into-text view"),
        Line::from(""),
        Line::from("  Forge / Skills"),
        Line::from("    F  open recent forge_run rows (auto-refresh)"),
        Line::from("    S  open active-skill list"),
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
/// outside compose / rename / delete. The overlay auto-
/// refreshes every second, so an in-flight run kicked off
/// from the CLI (or `POST /forge/optimize`) ticks along here
/// live. Esc / F / q close. Running rows get a `●` prefix so
/// the user can see at a glance which runs are still ticking.
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
        "   {:<8} {:<18} {:<8} {:<8} {:<6} {}",
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
            let prefix = if r.status == "running" { "● " } else { "  " };
            lines.push(Line::from(format!(
                " {prefix}{:<8} {:<18} {:<8} {:<8} {:<6} {}",
                id_short, r.skill_name, r.base_version, cand, score, r.status,
            )));
        }
    }
    let running = rows.iter().filter(|r| r.status == "running").count();
    let title = if running > 0 {
        format!(
            " forge runs ({} · {running} ●) — auto-refresh · Esc/F/q closes ",
            rows.len()
        )
    } else {
        format!(
            " forge runs ({}) — auto-refresh · Esc/F/q closes ",
            rows.len()
        )
    };
    let para = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Active-skill list overlay. Triggered by `S` (capital S to
/// avoid the `s`=steer clash). Read-only; auto-refresh isn't
/// useful here since active skills change rarely. Esc / S / q
/// close.
pub(crate) fn draw_skills(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[argo_skill::Skill],
) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(format!(
        " {:<20} {:<10} {:<8} {}",
        "name", "version", "lock", "hash (sha256, 12 chars)",
    )));
    if rows.is_empty() {
        lines.push(Line::from("  (no active skills installed)"));
    } else {
        for s in rows {
            let lock = format!("{:?}", s.frontmatter.lock).to_lowercase();
            let hash_short: String = hex::encode(s.hash).chars().take(12).collect();
            lines.push(Line::from(format!(
                " {:<20} {:<10} {:<8} {}",
                s.frontmatter.name, s.frontmatter.version, lock, hash_short,
            )));
        }
    }
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " active skills ({}) — Esc/S/q closes ",
                rows.len()
            )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Detail view for one persisted message — pretty-prints the
/// content_json (default) or, when `message_overlay_text_only`
/// is set, drills into the extracted tool_result / tool_use /
/// text payloads so the user can see them without JSON noise.
/// Triggered by `o` while focused on the messages pane;
/// toggled by `t`.
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

    let body = if state.message_overlay_text_only {
        extract_block_text(&msg.content_json).unwrap_or_else(|| {
            "(no tool_result / tool_use / text blocks — press t to see raw JSON)".to_string()
        })
    } else {
        match serde_json::from_str::<serde_json::Value>(&msg.content_json) {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| msg.content_json.clone()),
            Err(_) => msg.content_json.clone(),
        }
    };
    let mode_tag = if state.message_overlay_text_only {
        "drill"
    } else {
        "json"
    };
    let title = format!(
        " message [seq={} role={} {mode_tag}]  (t=toggle · Esc/o/q closes) ",
        msg.seq, msg.role
    );
    let para = Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Pull the human-readable payload out of every content block
/// in a `content_json` array. tool_result blocks contribute the
/// joined `.content[*].text` strings (with an `[error]` tag
/// when `is_error`); tool_use blocks become `→ <name>(<args>)`;
/// plain text blocks pass through. Multiple blocks are
/// separated by a `---` rule. Returns `None` when the input
/// isn't a JSON array of blocks (e.g. plain string content).
pub(crate) fn extract_block_text(content_json: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(content_json).ok()?;
    let arr = parsed.as_array()?;
    let mut chunks = Vec::with_capacity(arr.len());
    for b in arr {
        let kind = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match kind {
            "text" => {
                if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                    chunks.push(t.to_string());
                }
            }
            "tool_use" => {
                let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                let args = b
                    .get("input")
                    .or_else(|| b.get("arguments"))
                    .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                    .unwrap_or_default();
                chunks.push(format!("→ {name}\n{args}"));
            }
            "tool_result" => {
                let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let is_error = b.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                let tag = if is_error { " [error]" } else { "" };
                let text = b
                    .get("content")
                    .and_then(|c| c.as_array())
                    .map(|inner| {
                        inner
                            .iter()
                            .filter_map(|cb| cb.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                chunks.push(format!("← {id}{tag}\n{text}"));
            }
            _ => {}
        }
    }
    if chunks.is_empty() {
        return None;
    }
    Some(chunks.join("\n\n---\n\n"))
}

#[cfg(test)]
mod tests {
    use super::extract_block_text;

    #[test]
    fn extract_unwraps_tool_result_text_blocks() {
        let json = r#"[{"type":"tool_result","id":"abc","is_error":false,
                       "content":[{"type":"text","text":"hello"},
                                  {"type":"text","text":"world"}]}]"#;
        let drill = extract_block_text(json).unwrap();
        assert!(drill.contains("← abc"));
        assert!(drill.contains("hello"));
        assert!(drill.contains("world"));
        assert!(!drill.contains("[error]"));
    }

    #[test]
    fn extract_marks_errors_and_handles_multiple_blocks() {
        let json = r#"[{"type":"text","text":"prelude"},
                      {"type":"tool_use","id":"01","name":"file_read",
                       "input":{"path":"/x"}},
                      {"type":"tool_result","id":"01","is_error":true,
                       "content":[{"type":"text","text":"boom"}]}]"#;
        let drill = extract_block_text(json).unwrap();
        assert!(drill.contains("prelude"));
        assert!(drill.contains("→ file_read"));
        assert!(drill.contains("/x"));
        assert!(drill.contains("← 01 [error]"));
        assert!(drill.contains("boom"));
        assert!(drill.contains("---")); // block separator
    }

    #[test]
    fn extract_returns_none_for_plain_string_content() {
        assert!(extract_block_text("\"plain string\"").is_none());
    }
}
