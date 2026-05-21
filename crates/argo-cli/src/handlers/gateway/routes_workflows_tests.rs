//! Unit coverage for the rollback route shipped in
//! `web-workflow-rollback-feature`. The `parse_route` assertion below
//! is the lightweight smoke that the new POST path actually round-
//! trips through the route table — the full end-to-end is exercised by
//! the Playwright spec `web/tests/e2e/workflow-rollback.spec.ts` (when
//! a real gateway is running) and the store-layer behaviour by
//! `workflow_definition_rollback_tests.rs`.

use super::{parse_route, Route};

#[test]
fn parse_route_recognises_rollback_post() {
    let req = "POST /api/v1/verticals/pr_checker/workflows/standard_review/rollback HTTP/1.1\r\nHost: x\r\n\r\n";
    let route = parse_route(req, "POST ").expect("rollback route must parse");
    match route {
        Route::Rollback { vertical, name } => {
            assert_eq!(vertical, "pr_checker");
            assert_eq!(name, "standard_review");
        }
        other => panic!("expected Route::Rollback; got {other:?}"),
    }
}

#[test]
fn parse_route_rejects_rollback_on_non_post_verbs() {
    for verb in ["GET ", "PUT ", "DELETE "] {
        let req = format!(
            "{verb}/api/v1/verticals/v/workflows/n/rollback HTTP/1.1\r\nHost: x\r\n\r\n"
        );
        assert!(
            parse_route(&req, verb).is_none(),
            "{verb} on /rollback must not match the Rollback route"
        );
    }
}

#[test]
fn parse_route_still_recognises_existing_routes() {
    // Regression guard: the new Rollback arm sits beside the existing
    // SetHead / Archive arms — we re-confirm those still match.
    let put = "PUT /api/v1/verticals/v/workflows/n/head HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(matches!(parse_route(put, "PUT ").expect("set head"), Route::SetHead { .. }));
    let del = "DELETE /api/v1/verticals/v/workflows/n/versions/id-1 HTTP/1.1\r\nHost: x\r\n\r\n";
    assert!(matches!(parse_route(del, "DELETE ").expect("archive"), Route::Archive { .. }));
}
