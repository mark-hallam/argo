//! Modal overlay renderers — help cheatsheet (`?`) and message
//! detail (`o`). Lives in its own file so `render.rs` stays under
//! the 500-LOC budget. Tabular overlays (forge runs / skills /
//! memory / audit) live in the sibling `overlays_tables` module
//! and are re-exported below so existing call sites
//! (`crate::overlays::draw_forge_runs` etc.) keep working.
use crate::app::ViewerState;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub(crate) use crate::overlays_tables::{draw_audit, draw_forge_runs, draw_memory, draw_skills};

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
        Line::from("    d  delete (y to confirm)       e  end (stamp ended_at)"),
        Line::from("    /  filter"),
        Line::from(""),
        Line::from("  Messaging"),
        Line::from("    i  compose (Enter sends)       s  steer (system msg)"),
        Line::from("    o  open msg detail (focused)   y  yank latest message"),
        Line::from("    Y  yank message at scroll cursor"),
        Line::from("    t  inside msg detail: toggle drill-into-text view"),
        Line::from(""),
        Line::from("  Forge / Skills / Memory / Cost / Audit / Providers / Cron / Tools"),
        Line::from("    F  forge runs (auto-refresh)   S  active skills"),
        Line::from("    M  active memory               $  cost rollup"),
        Line::from("    A  audit chain (selected ses)  P  providers   K  cron"),
        Line::from("    T  tool registry"),
        Line::from("    R  force-refresh   C  clear live-event trail"),
        Line::from(""),
        Line::from("  q / Esc  quit       ?  this help (any key closes)"),
    ];
    let para = Paragraph::new(body)
        .block(Block::default().borders(Borders::ALL).title(" keybindings "))
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Cron job list overlay. Triggered by `K`. Lists every
/// `cron/jobs/*.json` basename in cwd. Esc / K / q close.
pub(crate) fn draw_cron(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[String],
    scroll: usize,
) {
    let w = area.width.min(60);
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(" cron job"));
    if rows.is_empty() {
        lines.push(Line::from("  (no cron/jobs/*.json found in cwd)"));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for name in &rows[clamped..] {
            lines.push(Line::from(format!("  {name}")));
        }
    }
    let para = Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).title(format!(
            " cron jobs ({}) — Esc/K/q closes ",
            rows.len()
        )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Provider list overlay. Triggered by `P`. Lists every
/// `manifests/providers/*.toml` basename (no manifest parse —
/// the basename IS the provider id by convention). Esc / P /
/// q close.
pub(crate) fn draw_providers(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[String],
    scroll: usize,
) {
    let w = area.width.min(60);
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(" provider id"));
    if rows.is_empty() {
        lines.push(Line::from(
            "  (no manifests/providers/*.toml found in cwd)",
        ));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for id in &rows[clamped..] {
            lines.push(Line::from(format!("  {id}")));
        }
    }
    let para = Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).title(format!(
            " providers ({}) — Esc/P/q closes ",
            rows.len()
        )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Tools overlay. Triggered by `T`. Snapshot of the in-core
/// `ToolRegistry` (built from `argo_tool::builtin_tools(cwd)`).
/// Names only — full schemas via the gateway's GET /tools.
pub(crate) fn draw_tools(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[String],
    scroll: usize,
) {
    let w = area.width.min(50);
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(30), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(" tool name"));
    if rows.is_empty() {
        lines.push(Line::from("  (no tools registered)"));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for name in &rows[clamped..] {
            lines.push(Line::from(format!("  {name}")));
        }
    }
    let para = Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).title(format!(
            " tools ({}) — Esc/T/q closes ",
            rows.len()
        )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Cross-tenant spend rollup overlay. Triggered by `$`. Shows
/// today / month-to-date / all-time totals; ceilings live in
/// argo.toml and aren't rendered here (the CLI already gives
/// them with `argo costs show`). Esc / `$` / q close.
pub(crate) fn draw_cost(
    frame: &mut Frame<'_>,
    area: Rect,
    snap: &argo_store::GlobalSpend,
) {
    let w = area.width.min(60);
    let h = area.height.min(10);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(7));

    let to_usd = |micros: u64| micros as f64 / 1_000_000.0;
    let lines = vec![
        Line::from("  spend rollup (USD, all tenants)"),
        Line::from(""),
        Line::from(format!("    today:        ${:>10.2}", to_usd(snap.today))),
        Line::from(format!("    this month:   ${:>10.2}", to_usd(snap.this_month))),
        Line::from(format!("    all time:     ${:>10.2}", to_usd(snap.all_time))),
        Line::from(""),
        Line::from("  (ceilings live in argo.toml; see `argo costs show`)"),
    ];
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" cost rollup — Esc/$/q closes "),
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
        crate::overlay_text::extract_block_text(&msg.content_json).unwrap_or_else(|| {
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
    let scroll_tag = if state.message_overlay_scroll > 0 {
        format!(" @{}", state.message_overlay_scroll)
    } else {
        String::new()
    };
    let title = format!(
        " message [seq={} role={} {mode_tag}{scroll_tag}]  (t=toggle · j/k scroll · Esc/o/q closes) ",
        msg.seq, msg.role
    );
    let para = Paragraph::new(body)
        .wrap(Wrap { trim: false })
        .scroll((state.message_overlay_scroll, 0))
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

// `extract_block_text` (and its tests) live in
// `super::overlay_text` to keep this file under the 500-LOC
// budget. The single caller in draw_message imports it directly.
