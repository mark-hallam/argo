use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn result(label: &str, score: usize) -> SearchResult {
    SearchResult::Skill(label.into(), String::new(), score)
}

#[test]
fn empty_query_starts_with_empty_results() {
    let overlay = SearchOverlay::default();
    assert!(overlay.query.is_empty());
    assert!(overlay.results.is_empty());
}

#[test]
fn typing_populates_query() {
    let mut overlay = SearchOverlay::default();
    assert_eq!(
        overlay.handle_key(key(KeyCode::Char('a'))),
        SearchOutcome::Stay
    );
    assert_eq!(
        overlay.handle_key(key(KeyCode::Char('r'))),
        SearchOutcome::Stay
    );
    assert_eq!(overlay.query, "ar");
}

#[test]
fn results_score_and_order_descending() {
    let mut overlay = SearchOverlay {
        results: vec![result("low", 10), result("high", 99), result("mid", 50)],
        ..SearchOverlay::default()
    };
    overlay
        .results
        .sort_by_key(|r| std::cmp::Reverse(r.score()));
    let names: Vec<String> = overlay
        .results
        .iter()
        .map(|r| match r {
            SearchResult::Skill(name, ..) => name.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(names, vec!["high", "mid", "low"]);
    assert!(
        score_text("alpha beta", "alpha").unwrap() > score_text("beta alpha", "alpha").unwrap()
    );
}

#[test]
fn tab_cycles_scope_filter() {
    let mut overlay = SearchOverlay::default();
    overlay.handle_key(key(KeyCode::Tab));
    assert_eq!(overlay.scope, ScopeFilter::Sessions);
    overlay.handle_key(key(KeyCode::Tab));
    assert_eq!(overlay.scope, ScopeFilter::Messages);
}

#[test]
fn navigation_wraps() {
    let mut overlay = SearchOverlay {
        results: vec![result("a", 1), result("b", 2)],
        ..SearchOverlay::default()
    };
    overlay.handle_key(key(KeyCode::Char('k')));
    assert_eq!(overlay.selected, 1);
    overlay.handle_key(key(KeyCode::Char('j')));
    assert_eq!(overlay.selected, 0);
}

#[test]
fn enter_returns_navigate_for_each_result_kind() {
    let sid = SessionId::new();
    let cases = vec![
        SearchResult::Session(sid, "s".into(), 1),
        SearchResult::Message(sid, 7, "m".into(), 1),
        SearchResult::Audit(sid, 8, "a".into(), 1),
        SearchResult::Skill("skill".into(), "d".into(), 1),
    ];
    for result in cases {
        let mut overlay = SearchOverlay {
            results: vec![result.clone()],
            ..SearchOverlay::default()
        };
        assert_eq!(
            overlay.handle_key(key(KeyCode::Enter)),
            SearchOutcome::Navigate(result)
        );
    }
}

#[test]
fn esc_returns_close_outcome() {
    let mut overlay = SearchOverlay::default();
    assert_eq!(overlay.handle_key(key(KeyCode::Esc)), SearchOutcome::Close);
}

#[test]
fn input_globals_maps_ctrl_k_to_search_everywhere() {
    let registry = crate::input_globals::default_global_keys();
    assert_eq!(
        crate::input_globals::lookup_global_key_with_modifiers(
            &registry,
            KeyCode::Char('k'),
            KeyModifiers::CONTROL
        ),
        Some(crate::input_globals::GlobalKeyAction::SearchEverywhereOverlay)
    );
    assert_eq!(
        crate::input_globals::lookup_global_key(&registry, KeyCode::Char('k')),
        None
    );
}
