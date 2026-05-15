//! Context-aware keybinding help overlay.
use crate::app::{PaneFocus, ViewerState};
use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// One visible keybinding row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyBinding {
    /// Key or chord shown to the operator.
    pub keys: &'static str,
    /// Short action description.
    pub description: &'static str,
}

/// A titled group of related keybindings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HelpSection {
    /// Section title.
    pub title: &'static str,
    /// Rows rendered under the section.
    pub bindings: Vec<KeyBinding>,
}

/// The surface whose local bindings should be shown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParentSurface {
    /// No local surface; only global bindings are shown.
    GlobalOnly,
    /// Session list pane has focus.
    SessionsList,
    /// Messages pane has focus.
    MessagesPane,
    /// Composer text input.
    Composer,
    /// Rename text input.
    Rename,
    /// Steer text input.
    Steer,
    /// Delete confirmation prompt.
    DeleteConfirm,
    /// Session filter input.
    Filter,
    /// Message detail modal.
    MessageDetail,
    /// List-style modal overlay.
    ListOverlay(&'static str),
    /// H-stack, replay, or dashboard modal overlay.
    AuxOverlay(&'static str),
}

/// Result of a keypress while the help overlay is open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HelpOutcome {
    /// Close the overlay.
    Close,
    /// Keep the overlay open and consume the key.
    StayOpen,
}

const GLOBAL: &[KeyBinding] = &[
    KeyBinding {
        keys: "?",
        description: "toggle keybindings help",
    },
    KeyBinding {
        keys: "q / Esc",
        description: "quit viewer or close active modal",
    },
    KeyBinding {
        keys: "Tab / Ctrl-K",
        description: "toggle pane focus or search everywhere",
    },
    KeyBinding {
        keys: "R / C",
        description: "refresh panes or clear live-event trail",
    },
    KeyBinding {
        keys: "F / S / M",
        description: "forge runs, active skills, memory overlays",
    },
    KeyBinding {
        keys: "$ / A / E",
        description: "cost rollup, audit chain, tool errors overlays",
    },
    KeyBinding {
        keys: "P / K / T",
        description: "providers, cron, tools overlays",
    },
    KeyBinding {
        keys: "O / H / B / V / D",
        description: "h-stack panels, replay, dashboard overlays",
    },
];

const SESSIONS: &[KeyBinding] = &[
    KeyBinding {
        keys: "j / k",
        description: "move selected session down or up",
    },
    KeyBinding {
        keys: "g / G",
        description: "jump to first or last session",
    },
    KeyBinding {
        keys: "/",
        description: "filter sessions",
    },
    KeyBinding {
        keys: "n",
        description: "clone selected session",
    },
    KeyBinding {
        keys: "r",
        description: "rename selected session",
    },
    KeyBinding {
        keys: "d",
        description: "delete selected session with confirmation",
    },
    KeyBinding {
        keys: "e",
        description: "stamp selected session ended",
    },
    KeyBinding {
        keys: "i / s",
        description: "compose or steer selected session",
    },
];

const MESSAGES: &[KeyBinding] = &[
    KeyBinding {
        keys: "j / k",
        description: "scroll messages down or up",
    },
    KeyBinding {
        keys: "PgDn / PgUp",
        description: "page message pane",
    },
    KeyBinding {
        keys: "o",
        description: "open selected message detail",
    },
    KeyBinding {
        keys: "y",
        description: "yank latest message",
    },
    KeyBinding {
        keys: "Y",
        description: "yank message at scroll cursor",
    },
];

const TEXT_INPUT: &[KeyBinding] = &[
    KeyBinding {
        keys: "type",
        description: "edit current buffer",
    },
    KeyBinding {
        keys: "Enter",
        description: "commit input",
    },
    KeyBinding {
        keys: "Backspace",
        description: "delete previous character",
    },
    KeyBinding {
        keys: "Esc",
        description: "cancel input",
    },
];

const MESSAGE_DETAIL: &[KeyBinding] = &[
    KeyBinding {
        keys: "j / k",
        description: "scroll detail",
    },
    KeyBinding {
        keys: "PgDn / PgUp",
        description: "page detail",
    },
    KeyBinding {
        keys: "t",
        description: "toggle JSON or extracted text",
    },
    KeyBinding {
        keys: "o / q / Esc",
        description: "close message detail",
    },
];

const LIST_MODAL: &[KeyBinding] = &[
    KeyBinding {
        keys: "j / k",
        description: "scroll rows down or up",
    },
    KeyBinding {
        keys: "PgDn / PgUp",
        description: "page rows",
    },
    KeyBinding {
        keys: "g / G",
        description: "jump to first or last row",
    },
    KeyBinding {
        keys: "q / Esc",
        description: "close overlay",
    },
];

/// Pick the local surface from the current viewer state.
#[must_use]
pub fn surface_for_state(state: &ViewerState) -> ParentSurface {
    if state.filter_mode {
        return ParentSurface::Filter;
    }
    if state.composer_mode {
        return ParentSurface::Composer;
    }
    if state.rename_mode() {
        return ParentSurface::Rename;
    }
    if state.steer_mode() {
        return ParentSurface::Steer;
    }
    if state.pending_delete.is_some() {
        return ParentSurface::DeleteConfirm;
    }
    if state.message_overlay.is_some() {
        return ParentSurface::MessageDetail;
    }
    for (is_open, name) in [
        (state.forge_runs.is_some(), "Forge runs"),
        (state.skills_overlay.is_some(), "Active skills"),
        (state.memory_overlay.is_some(), "Memory"),
        (state.cost_overlay.is_some(), "Cost rollup"),
        (state.audit_overlay.is_some(), "Audit chain"),
        (state.providers_overlay.is_some(), "Providers"),
        (state.cron_overlay.is_some(), "Cron"),
        (state.tools_overlay.is_some(), "Tools"),
        (state.tool_errors_overlay.is_some(), "Tool errors"),
    ] {
        if is_open {
            return ParentSurface::ListOverlay(name);
        }
    }
    for (is_open, name) in [
        (state.h_sessions_overlay.is_some(), "Orchestrator sessions"),
        (state.h_handoffs_overlay.is_some(), "Orchestrator handoffs"),
        (state.h_budget_overlay.is_some(), "Budget"),
        (state.replay_overlay.is_some(), "Replay"),
        (state.dashboard_overlay.is_some(), "Realtime dashboard"),
    ] {
        if is_open {
            return ParentSurface::AuxOverlay(name);
        }
    }
    match state.focus {
        PaneFocus::Sessions => ParentSurface::SessionsList,
        PaneFocus::Messages => ParentSurface::MessagesPane,
    }
}

/// Build the sections rendered for a surface.
#[must_use]
pub fn sections_for_surface(surface: ParentSurface) -> Vec<HelpSection> {
    let mut sections = vec![section("Global", GLOBAL)];
    match surface {
        ParentSurface::GlobalOnly => {}
        ParentSurface::SessionsList => sections.push(section("Sessions list", SESSIONS)),
        ParentSurface::MessagesPane => sections.push(section("Messages pane", MESSAGES)),
        ParentSurface::Composer => sections.push(section("Composer", TEXT_INPUT)),
        ParentSurface::Rename => sections.push(section("Rename", TEXT_INPUT)),
        ParentSurface::Steer => sections.push(section("Steer", TEXT_INPUT)),
        ParentSurface::DeleteConfirm => sections.push(HelpSection {
            title: "Delete confirmation",
            bindings: vec![
                KeyBinding {
                    keys: "y",
                    description: "confirm delete",
                },
                KeyBinding {
                    keys: "any other key",
                    description: "cancel delete",
                },
            ],
        }),
        ParentSurface::Filter => sections.push(section("Filter", TEXT_INPUT)),
        ParentSurface::MessageDetail => sections.push(section("Message detail", MESSAGE_DETAIL)),
        ParentSurface::ListOverlay(name) | ParentSurface::AuxOverlay(name) => {
            sections.push(section(name, LIST_MODAL));
        }
    }
    sections
}

/// Handle a key while the help overlay is active.
#[must_use]
pub fn handle_key(code: KeyCode) -> HelpOutcome {
    match code {
        KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => HelpOutcome::Close,
        _ => HelpOutcome::StayOpen,
    }
}

/// Render the overlay for the current viewer state.
pub fn render(frame: &mut Frame<'_>, area: Rect, state: &ViewerState) {
    render_sections(frame, area, &sections_for_surface(surface_for_state(state)));
}

fn section(title: &'static str, bindings: &[KeyBinding]) -> HelpSection {
    HelpSection {
        title,
        bindings: bindings.to_vec(),
    }
}

fn render_sections(frame: &mut Frame<'_>, area: Rect, sections: &[HelpSection]) {
    let w = area.width.saturating_mul(7) / 10;
    let h = area.height.saturating_mul(8) / 10;
    let panel = Rect::new(
        area.x + (area.width.saturating_sub(w)) / 2,
        area.y + (area.height.saturating_sub(h)) / 2,
        w.max(48).min(area.width),
        h.max(10).min(area.height),
    );
    let lines = lines_for_sections(sections);
    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" keybindings "),
        )
        .style(Style::default());
    frame.render_widget(Clear, panel);
    frame.render_widget(para, panel);
}

fn lines_for_sections(sections: &[HelpSection]) -> Vec<Line<'static>> {
    if sections.iter().all(|section| section.bindings.is_empty()) {
        return vec![Line::from("  No keybindings for this surface")];
    }
    let mut lines = Vec::new();
    for (idx, section) in sections.iter().enumerate() {
        if idx > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            section.title,
            Style::default().add_modifier(Modifier::BOLD),
        )));
        if section.bindings.is_empty() {
            lines.push(Line::from("  (none)"));
            continue;
        }
        for binding in &section.bindings {
            lines.push(Line::from(vec![
                Span::raw(format!("  {:<12}", binding.keys)),
                Span::raw(binding.description),
            ]));
        }
    }
    lines
}

#[cfg(test)]
#[path = "keybindings_help_tests.rs"]
mod keybindings_help_tests;
