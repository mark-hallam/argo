//! Multi-column tabular overlays — forge runs (`F`), active skills
//! (`S`), active memory (`M`), audit chain (`A`). Split out of
//! `overlays.rs` to keep that file under the 500-LOC budget. Each
//! function here renders rows pulled from a typed struct
//! (`ForgeRunRow`, `Skill`, `MemoryEntry`, `AuditRow`) into a
//! padded, scrollable table.
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

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
    scroll: usize,
) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(format!(
        "   {:<8} {:<14} {:<6} {:<8} {:<5} {:<6} {}",
        "id", "skill", "base", "candidate", "iters", "score", "status",
    )));
    if rows.is_empty() {
        lines.push(Line::from("  (no forge runs recorded yet)"));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for r in &rows[clamped..] {
            let id_short: String = hex::encode(&r.id).chars().take(8).collect();
            let cand = r.candidate_version.clone().unwrap_or_else(|| "-".into());
            let score = r
                .final_score
                .map(|s| format!("{s:.3}"))
                .unwrap_or_else(|| "-".into());
            let prefix = if r.status == "running" { "● " } else { "  " };
            // Truncate-and-pad: keep columns aligned even when
            // a value overflows its slot.
            let skill = clip(&r.skill_name, 14);
            let base = clip(&r.base_version, 6);
            let cand = clip(&cand, 8);
            lines.push(Line::from(format!(
                " {prefix}{:<8} {:<14} {:<6} {:<8} {:<5} {:<6} {}",
                id_short, skill, base, cand, r.iterations, score, r.status,
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
    scroll: usize,
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
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for s in &rows[clamped..] {
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

/// Active-memory list overlay. Triggered by `M`. One row per
/// active entry across all four canonical files
/// (Memory / User / Soul / Agents). Esc / M / q close.
pub(crate) fn draw_memory(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[argo_memory::MemoryEntry],
    scroll: usize,
) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(format!(
        " {:<10} {:<8} {:<6} {:<8} {}",
        "file", "id", "bytes", "lock", "hash (12)",
    )));
    if rows.is_empty() {
        lines.push(Line::from(
            "  (no active memory entries — this workspace hasn't recorded any)",
        ));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for e in &rows[clamped..] {
            let id_short: String = e.id.to_string().chars().take(8).collect();
            let lock = format!("{:?}", e.lock).to_lowercase();
            let hash_short: String = hex::encode(e.hash).chars().take(12).collect();
            lines.push(Line::from(format!(
                " {:<10} {:<8} {:<6} {:<8} {}",
                format!("{:?}", e.file).to_lowercase(),
                id_short,
                e.body.len(),
                lock,
                hash_short,
            )));
        }
    }
    let para = Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).title(format!(
            " active memory ({}) — Esc/M/q closes ",
            rows.len()
        )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Per-session audit chain overlay. Triggered by `A`. Shows
/// every audit row scoped to the currently-selected session
/// (TurnStart / AssistantMessage / ToolCall / ToolResult /
/// TurnEnd / etc.) with seq + kind + a 60-char payload preview.
/// Scrollable with `j`/`k`/`PgUp`/`PgDn` (#336). Esc / A / q
/// close.
pub(crate) fn draw_audit(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[argo_store::AuditRow],
    scroll: usize,
) {
    let w = area.width.saturating_mul(9) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let panel = Rect::new(x, y, w.max(40), h.max(8));

    let mut lines: Vec<Line> = Vec::with_capacity(rows.len() + 2);
    lines.push(Line::from(format!(
        " {:>6}  {:<22} {}",
        "seq", "kind", "payload (truncated)"
    )));
    if rows.is_empty() {
        lines.push(Line::from(
            "  (no audit rows for this session — start a turn first)",
        ));
    } else {
        let clamped = scroll.min(rows.len().saturating_sub(1));
        for r in &rows[clamped..] {
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
    }
    let scroll_tag = if scroll > 0 {
        format!(" @{scroll}")
    } else {
        String::new()
    };
    let para = Paragraph::new(lines).block(
        Block::default().borders(Borders::ALL).title(format!(
            " audit chain ({}{scroll_tag}) — j/k scroll · Esc/A/q closes ",
            rows.len()
        )),
    );
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

/// Truncate `s` to at most `n` characters; trailing char is `…`
/// when clipped. Used by `draw_forge_runs` to keep table
/// columns aligned even when a skill name / version overflows.
fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}
