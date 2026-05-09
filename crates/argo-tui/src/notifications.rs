//! Toast notifications — top-right transient surface for approval
//! requests + error events.
//!
//! Designed as a passive overlay: the queue renders above every
//! other surface but only consumes the `x` key (dismiss topmost
//! dismissable toast). Every other key falls through so the active
//! overlay below stays interactive.
//!
//! Lifecycle: callers `push` toasts as gateway events arrive; the
//! event loop calls `tick` once per frame to expire toasts whose
//! `created_at + ttl` has passed; `render` paints the visible
//! window in the top-right of the frame.
//!
//! Wiring to the gateway event stream + frame-draw call site is
//! tracked in a follow-up: `crates/argo-tui/src/input.rs` is at the
//! 500-LOC ceiling and `render.rs` is outside this brief's
//! may-touch list, so this module ships the queue + render
//! primitives only.
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::theme::Theme;

/// Default visible-toast cap. New pushes beyond the cap evict the
/// oldest toast.
pub const DEFAULT_MAX_VISIBLE: usize = 5;

/// Default time-to-live for a toast before auto-dismiss. Matches
/// the brief's `[tui.notifications] ttl_ms = 5000` default.
pub const DEFAULT_TTL: Duration = Duration::from_secs(5);

const TOAST_WIDTH: u16 = 40;
const TOAST_HEIGHT: u16 = 3;
const RIGHT_MARGIN: u16 = 1;
const TOP_MARGIN: u16 = 1;

/// Category of a toast — drives the color shown to the operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    /// Worker hit an approval checkpoint and is awaiting a yes/no.
    ApprovalRequested,
    /// Worker (or session) emitted an `Error` event.
    Error,
    /// Generic informational message — neutral color.
    Info,
    /// Non-fatal warning — warning color.
    Warning,
}

/// One toast entry on the queue.
#[derive(Debug, Clone)]
pub struct Toast {
    /// Stable id so callers can dedupe re-pushes of the same event.
    pub id: u64,
    /// Visual category — drives the border + title color.
    pub kind: ToastKind,
    /// One-line title — surfaced as the block title.
    pub title: String,
    /// Body text — rendered inside the block, wrapped to width.
    pub body: String,
    /// Wall-clock instant the toast entered the queue. Combined
    /// with `ttl` to compute expiry inside [`ToastQueue::tick`].
    pub created_at: Instant,
    /// Auto-dismiss interval. `created_at + ttl` is the expiry.
    pub ttl: Duration,
    /// `true` when the operator can press `x` to dismiss the toast.
    pub dismissable: bool,
}

impl Toast {
    /// Build an `ApprovalRequested` toast with the default TTL.
    #[must_use]
    pub fn approval(id: u64, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id,
            kind: ToastKind::ApprovalRequested,
            title: title.into(),
            body: body.into(),
            created_at: Instant::now(),
            ttl: DEFAULT_TTL,
            dismissable: true,
        }
    }

    /// Build an `Error` toast with the default TTL.
    #[must_use]
    pub fn error(id: u64, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id,
            kind: ToastKind::Error,
            title: title.into(),
            body: body.into(),
            created_at: Instant::now(),
            ttl: DEFAULT_TTL,
            dismissable: true,
        }
    }

    /// Build an `Info` toast with the default TTL.
    #[must_use]
    pub fn info(id: u64, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            id,
            kind: ToastKind::Info,
            title: title.into(),
            body: body.into(),
            created_at: Instant::now(),
            ttl: DEFAULT_TTL,
            dismissable: true,
        }
    }
}

/// Result of a [`ToastQueue::handle_key`] call. `Pass` means the
/// queue did not consume the key, so the caller should forward it
/// to the active overlay or the main dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastOutcome {
    /// Queue handled the key (currently only `x` consumes).
    Dismissed,
    /// Queue ignored the key — pass through.
    Pass,
}

/// Bounded FIFO of toasts with auto-dismiss + render support.
///
/// Caps at `max_visible` entries; pushes beyond the cap evict the
/// front. `tick(now)` drops entries whose `created_at + ttl <= now`;
/// `render` paints them into the top-right column.
#[derive(Debug, Clone)]
pub struct ToastQueue {
    toasts: VecDeque<Toast>,
    max_visible: usize,
}

impl Default for ToastQueue {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_VISIBLE)
    }
}

impl ToastQueue {
    /// Build a queue with the given visible cap. Pushes beyond the
    /// cap evict the oldest toast.
    #[must_use]
    pub fn new(max_visible: usize) -> Self {
        Self {
            toasts: VecDeque::new(),
            max_visible: max_visible.max(1),
        }
    }

    /// Number of toasts currently on the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    /// `true` when no toasts are currently visible.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    /// Cap on simultaneously-visible toasts.
    #[must_use]
    pub fn max_visible(&self) -> usize {
        self.max_visible
    }

    /// Iterate over the queued toasts, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter()
    }

    /// Append a toast. When the queue is at `max_visible`, evicts
    /// the oldest entry first so the newest is always shown.
    pub fn push(&mut self, toast: Toast) {
        if self.toasts.len() >= self.max_visible {
            self.toasts.pop_front();
        }
        self.toasts.push_back(toast);
    }

    /// Drop every toast whose expiry (`created_at + ttl`) is at or
    /// before `now`. Returns the number of toasts removed.
    pub fn tick(&mut self, now: Instant) -> usize {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.created_at + t.ttl > now);
        before - self.toasts.len()
    }

    /// Dismiss the topmost (newest) dismissable toast. Returns
    /// [`ToastOutcome::Dismissed`] when one was removed,
    /// [`ToastOutcome::Pass`] otherwise.
    pub fn dismiss_topmost(&mut self) -> ToastOutcome {
        for idx in (0..self.toasts.len()).rev() {
            if self.toasts.get(idx).is_some_and(|t| t.dismissable) {
                self.toasts.remove(idx);
                return ToastOutcome::Dismissed;
            }
        }
        ToastOutcome::Pass
    }

    /// Inspect a key event without consuming it from the caller's
    /// stream. Returns [`ToastOutcome::Dismissed`] when the key was
    /// `x` AND a dismissable toast was removed; otherwise
    /// [`ToastOutcome::Pass`].
    pub fn handle_key(&mut self, key: KeyEvent) -> ToastOutcome {
        if matches!(key.code, KeyCode::Char('x')) {
            return self.dismiss_topmost();
        }
        ToastOutcome::Pass
    }

    /// Paint visible toasts into the top-right column of `area`.
    /// Each toast is a 3-row bordered block themed by kind.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, theme: &Theme) {
        if self.toasts.is_empty() || area.width <= RIGHT_MARGIN || area.height <= TOP_MARGIN {
            return;
        }
        let width = TOAST_WIDTH.min(area.width.saturating_sub(RIGHT_MARGIN));
        if width < 8 {
            return;
        }
        let x = area.x + area.width.saturating_sub(width + RIGHT_MARGIN);
        let mut y = area.y + TOP_MARGIN;
        for toast in self.toasts.iter().rev() {
            if y + TOAST_HEIGHT > area.y + area.height {
                break;
            }
            let toast_area = Rect {
                x,
                y,
                width,
                height: TOAST_HEIGHT,
            };
            draw_one(frame, toast_area, toast, theme);
            y += TOAST_HEIGHT;
        }
    }
}

fn draw_one(frame: &mut Frame<'_>, area: Rect, toast: &Toast, theme: &Theme) {
    let color = color_for_kind(toast.kind, theme);
    let title_text = format!(" {} ", toast.title);
    let title_span = Span::styled(
        title_text,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Line::from(title_span));
    let body_style = Style::default().fg(theme.fg);
    let para = Paragraph::new(toast.body.as_str())
        .style(body_style)
        .wrap(Wrap { trim: true })
        .block(block);
    frame.render_widget(para, area);
}

/// Theme-aware color lookup for a [`ToastKind`]. Public so tests
/// and the eventual frame.draw call-site can assert color parity
/// without re-deriving the mapping.
#[must_use]
pub fn color_for_kind(kind: ToastKind, theme: &Theme) -> ratatui::style::Color {
    match kind {
        ToastKind::ApprovalRequested => theme.accent,
        ToastKind::Error => theme.error,
        ToastKind::Info => theme.muted,
        ToastKind::Warning => theme.warning,
    }
}

#[cfg(test)]
#[path = "notifications_tests.rs"]
mod tests;
