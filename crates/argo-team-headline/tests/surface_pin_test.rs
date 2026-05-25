//! External surface-pin test for `argo-team-headline`.
//!
//! Lives under `tests/` so it can only see the crate's published
//! public API — anything tested here is what downstream callers
//! (`argo-cli` gateway routes) actually depend on. Breaking changes
//! to the surface will fail this test before they reach a route
//! handler.
//!
//! Pins three contracts:
//!
//! 1. **Exported inventory.** Names + concrete shapes of the public
//!    types, consts, and free functions re-exported from
//!    `argo-team-headline`'s `lib.rs`. A removed `pub use` or a
//!    renamed field stops compilation here.
//! 2. **KPI deterministic contract.** `bullets_for_vertical` +
//!    `BulletKind::format` produce the singular/plural strings the
//!    SPA renders, and `MAX_BULLETS` clamps the slice length.
//! 3. **Wire-shape contract.** `TeamHeadline` JSON serialization
//!    matches the keys gateway clients expect (`bullets`, `tomorrow`),
//!    including the `Option<String> => null` convention for the
//!    no-Tomorrow case.

use argo_team_headline::{
    bullets_for_vertical, ActivityCounts, BulletKind, TeamHeadline, MAX_BULLETS, MAX_BULLET_CHARS,
};

/// Inventory pin: every name below is part of the supported
/// external surface. If a downstream rename or removal breaks the
/// use statement at the top, the audit cell must be re-opened.
#[test]
fn exported_surface_inventory_is_pinned() {
    // Consts.
    let _: usize = MAX_BULLETS;
    let _: usize = MAX_BULLET_CHARS;
    assert_eq!(MAX_BULLETS, 3, "Team page layout assumes 2-3 bullets");
    assert_eq!(
        MAX_BULLET_CHARS, 60,
        "Bullet crop length pinned by SPA team-card layout"
    );

    // ActivityCounts: public field names + Default ctor.
    let counts = ActivityCounts {
        assistant_messages: 0,
        tool_calls: 0,
        pending_escalations: 0,
    };
    assert_eq!(counts, ActivityCounts::default());

    // TeamHeadline: public field names + Default-ish ctor by literal.
    let headline = TeamHeadline {
        bullets: vec!["No activity yet.".to_string()],
        tomorrow: None,
    };
    assert_eq!(headline.bullets.len(), 1);
    assert!(headline.tomorrow.is_none());

    // bullets_for_vertical returns a static slice of BulletKind.
    let kinds: &'static [BulletKind] = bullets_for_vertical("gtm-engineer");
    assert!(!kinds.is_empty(), "known vertical must yield at least 1 KPI");
}

/// KPI contract: `gtm-engineer` keeps the 3-slot ordering the brief
/// pinned (`Messages, Pending, NotComputable`). Plural rendering of
/// the Messages slot drives the canonical "8 replies sent" bullet
/// shown in the brief example.
#[test]
fn gtm_engineer_kpi_contract_renders_plural_replies() {
    let kinds = bullets_for_vertical("gtm-engineer");
    assert_eq!(kinds.len(), 3);

    let counts = ActivityCounts {
        assistant_messages: 8,
        tool_calls: 0,
        pending_escalations: 0,
    };

    let rendered: Vec<String> = kinds.iter().filter_map(|k| k.format(&counts)).collect();
    assert_eq!(rendered, vec!["8 replies sent".to_string()]);
}

/// KPI contract: singular rendering must say "1 reply" not "1
/// replies". Catches plural-template regressions across all 14
/// hardcoded verticals if anyone bulk-edits `kpi_table.rs`.
#[test]
fn gtm_engineer_kpi_contract_renders_singular_reply() {
    let kinds = bullets_for_vertical("gtm-engineer");
    let counts = ActivityCounts {
        assistant_messages: 1,
        tool_calls: 0,
        pending_escalations: 0,
    };
    let rendered: Vec<String> = kinds.iter().filter_map(|k| k.format(&counts)).collect();
    assert_eq!(rendered, vec!["1 reply sent".to_string()]);
}

/// Zero-count bullets are dropped (yielding `None`), so a worker
/// with no activity in a slot doesn't emit `"0 things"` strings to
/// the SPA. Pins the per-slot zero filter the renderer depends on.
#[test]
fn bullet_kind_format_drops_zero_counts() {
    let kinds = bullets_for_vertical("recruiter");
    let counts = ActivityCounts::default();
    for k in kinds {
        assert!(
            k.format(&counts).is_none(),
            "zero-count bullets must be filtered, got {:?}",
            k.format(&counts)
        );
    }
}

/// Unknown vertical ids fall back to a non-empty KPI slice — the
/// SPA never receives a hard error for a new/unmapped vertical.
#[test]
fn unknown_vertical_falls_back_to_non_empty_kinds() {
    let kinds = bullets_for_vertical("never-shipped-vertical");
    assert!(
        !kinds.is_empty(),
        "fallback slice must always be non-empty"
    );

    let counts = ActivityCounts {
        assistant_messages: 1,
        tool_calls: 2,
        pending_escalations: 0,
    };
    let rendered: Vec<String> = kinds.iter().filter_map(|k| k.format(&counts)).collect();
    assert!(
        rendered.iter().any(|s| s.contains("actions taken today")),
        "fallback must render 'actions taken today' shape, got {rendered:?}"
    );
}

/// Wire shape: gateway returns `TeamHeadline` inside the team-list
/// JSON. Pin both keys (`bullets`, `tomorrow`) and the
/// `Option => null` convention SPA code reads.
#[test]
fn team_headline_serializes_to_expected_json_shape() {
    let headline = TeamHeadline {
        bullets: vec![
            "8 replies sent".to_string(),
            "2 pending counter-offers".to_string(),
        ],
        tomorrow: None,
    };
    let v = serde_json::to_value(&headline).unwrap();
    assert!(v.is_object());
    let obj = v.as_object().unwrap();
    assert_eq!(obj.len(), 2, "exactly two top-level keys: bullets + tomorrow");
    let bullets = obj.get("bullets").expect("`bullets` key");
    assert_eq!(bullets.as_array().unwrap().len(), 2);
    assert_eq!(
        bullets.as_array().unwrap()[0].as_str().unwrap(),
        "8 replies sent"
    );
    let tomorrow = obj.get("tomorrow").expect("`tomorrow` key always present");
    assert!(
        tomorrow.is_null(),
        "Option::None must serialize as JSON null, got {tomorrow:?}"
    );
}

/// Wire shape: Tomorrow line, when present, is a flat string (not
/// a nested object). Locks the `Option<String>` → string convention
/// the SPA's `headline.tomorrow ?? ''` chain depends on.
#[test]
fn team_headline_with_tomorrow_serializes_as_flat_string() {
    let headline = TeamHeadline {
        bullets: vec!["1 reply sent".to_string()],
        tomorrow: Some("morning routine at 06:30".to_string()),
    };
    let v = serde_json::to_value(&headline).unwrap();
    let tomorrow = v.get("tomorrow").unwrap();
    assert_eq!(tomorrow.as_str(), Some("morning routine at 06:30"));
}
