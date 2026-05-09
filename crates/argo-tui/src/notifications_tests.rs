//! Unit tests for the toast queue. Stick to the 7 cases listed in
//! the brief: lifecycle, eviction, render @ widths, color-per-kind,
//! dismiss topmost, non-dismissable ignores `x`, ttl boundary.

use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::style::Color;
use ratatui::Terminal;

fn make_toast(id: u64, ttl: Duration, dismissable: bool) -> Toast {
    Toast {
        id,
        kind: ToastKind::Info,
        title: format!("t{id}"),
        body: format!("body-{id}"),
        created_at: Instant::now(),
        ttl,
        dismissable,
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

#[test]
fn push_tick_expire_lifecycle() {
    let mut q = ToastQueue::default();
    assert!(q.is_empty());
    let t = make_toast(1, Duration::from_millis(50), true);
    let created = t.created_at;
    q.push(t);
    assert_eq!(q.len(), 1);

    let dropped_before_expiry = q.tick(created + Duration::from_millis(10));
    assert_eq!(dropped_before_expiry, 0);
    assert_eq!(q.len(), 1);

    let dropped_after_expiry = q.tick(created + Duration::from_millis(60));
    assert_eq!(dropped_after_expiry, 1);
    assert!(q.is_empty());
}

#[test]
fn max_visible_evicts_oldest_when_full() {
    let mut q = ToastQueue::new(5);
    for i in 0..6_u64 {
        q.push(make_toast(i, Duration::from_secs(60), true));
    }
    assert_eq!(q.len(), 5, "queue caps at max_visible");
    let ids: Vec<u64> = q.iter().map(|t| t.id).collect();
    assert_eq!(ids, vec![1, 2, 3, 4, 5], "oldest (id=0) evicted");
}

#[test]
fn render_empty_one_three_at_various_widths() {
    let theme = Theme::dark();

    // 0 toasts → no output.
    {
        let mut term = Terminal::new(TestBackend::new(40, 10)).expect("term");
        let q = ToastQueue::default();
        term.draw(|f| q.render(f, f.area(), &theme)).expect("draw");
        let buf = term.backend().buffer();
        let painted = buf
            .content
            .iter()
            .any(|c| c.symbol() != " " && !c.symbol().is_empty());
        assert!(!painted, "empty queue paints nothing");
    }

    // 1 toast at width 80 — paints.
    {
        let mut term = Terminal::new(TestBackend::new(80, 10)).expect("term");
        let mut q = ToastQueue::default();
        q.push(Toast::approval(1, "approval", "session abc"));
        term.draw(|f| q.render(f, f.area(), &theme)).expect("draw");
        let buf = term.backend().buffer();
        let dump: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(dump.contains("approval"), "title rendered: {dump:?}");
    }

    // 3 toasts at width 80 — all three titles appear.
    {
        let mut term = Terminal::new(TestBackend::new(80, 20)).expect("term");
        let mut q = ToastQueue::default();
        q.push(Toast::approval(1, "alpha", "first"));
        q.push(Toast::error(2, "bravo", "second"));
        q.push(Toast::info(3, "charlie", "third"));
        term.draw(|f| q.render(f, f.area(), &theme)).expect("draw");
        let buf = term.backend().buffer();
        let dump: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(dump.contains("alpha"), "alpha: {dump:?}");
        assert!(dump.contains("bravo"), "bravo: {dump:?}");
        assert!(dump.contains("charlie"), "charlie: {dump:?}");
    }

    // Narrow width (< 8 cells of usable column) — render is a no-op.
    {
        let mut term = Terminal::new(TestBackend::new(6, 10)).expect("term");
        let mut q = ToastQueue::default();
        q.push(Toast::approval(1, "narrow", "x"));
        term.draw(|f| q.render(f, f.area(), &theme)).expect("draw");
        let buf = term.backend().buffer();
        let dump: String = buf.content.iter().map(|c| c.symbol()).collect();
        assert!(
            !dump.contains("narrow"),
            "narrow frame skips render: {dump:?}"
        );
    }
}

#[test]
fn color_matches_theme_per_kind() {
    let theme = Theme::dark();
    assert_eq!(
        color_for_kind(ToastKind::ApprovalRequested, &theme),
        theme.accent
    );
    assert_eq!(color_for_kind(ToastKind::Error, &theme), theme.error);
    assert_eq!(color_for_kind(ToastKind::Warning, &theme), theme.warning);
    assert_eq!(color_for_kind(ToastKind::Info, &theme), theme.muted);

    // A non-default theme threads through too.
    let custom = Theme {
        accent: Color::Rgb(0xab, 0xcd, 0xef),
        error: Color::Rgb(0xff, 0x00, 0x00),
        warning: Color::Rgb(0x00, 0xff, 0x00),
        muted: Color::Rgb(0x33, 0x33, 0x33),
        ..Theme::dark()
    };
    assert_eq!(
        color_for_kind(ToastKind::ApprovalRequested, &custom),
        Color::Rgb(0xab, 0xcd, 0xef)
    );
}

#[test]
fn x_dismisses_topmost_dismissable_toast() {
    let mut q = ToastQueue::default();
    q.push(make_toast(1, Duration::from_secs(60), true));
    q.push(make_toast(2, Duration::from_secs(60), true));
    q.push(make_toast(3, Duration::from_secs(60), true));

    let outcome = q.handle_key(key(KeyCode::Char('x')));
    assert_eq!(outcome, ToastOutcome::Dismissed);
    assert_eq!(q.len(), 2);
    let ids: Vec<u64> = q.iter().map(|t| t.id).collect();
    assert_eq!(ids, vec![1, 2], "newest (id=3) dismissed first");
}

#[test]
fn non_dismissable_toast_ignores_x() {
    let mut q = ToastQueue::default();
    q.push(make_toast(1, Duration::from_secs(60), false));

    let outcome = q.handle_key(key(KeyCode::Char('x')));
    assert_eq!(
        outcome,
        ToastOutcome::Pass,
        "x with no dismissable toast falls through"
    );
    assert_eq!(q.len(), 1);

    // Other keys also pass through unchanged.
    let outcome = q.handle_key(key(KeyCode::Char('q')));
    assert_eq!(outcome, ToastOutcome::Pass);
    assert_eq!(q.len(), 1);
}

#[test]
fn tick_at_exact_ttl_boundary_expires() {
    let mut q = ToastQueue::default();
    let ttl = Duration::from_millis(100);
    let toast = make_toast(1, ttl, true);
    let created = toast.created_at;
    q.push(toast);

    // Strictly before expiry — keeps the toast.
    assert_eq!(q.tick(created + ttl - Duration::from_millis(1)), 0);
    assert_eq!(q.len(), 1);

    // At exact expiry — `created_at + ttl <= now` so it expires.
    assert_eq!(q.tick(created + ttl), 1);
    assert!(q.is_empty());
}
