//! Unit coverage for the `argo://` deep-link parser.
//!
//! Each branch of [`super::parse`] is exercised: happy path for
//! every recognised verb, every error variant, and the
//! query-string / fragment / whitespace / case-insensitive scheme
//! edge cases.

use super::{parse, DeepLinkError, DeepLinkIntent};

#[test]
fn approve_url_returns_approve_intent() {
    assert_eq!(
        parse("argo://approve/act-123").unwrap(),
        DeepLinkIntent::Approve {
            action_id: "act-123".into(),
        },
    );
}

#[test]
fn reject_url_returns_reject_intent() {
    assert_eq!(
        parse("argo://reject/act-456").unwrap(),
        DeepLinkIntent::Reject {
            action_id: "act-456".into(),
        },
    );
}

#[test]
fn inbox_approval_url_returns_token() {
    let token = "act-1.approve.deadbeef";
    assert_eq!(
        parse(&format!("argo://inbox/approval/{token}")).unwrap(),
        DeepLinkIntent::InboxApproval {
            token: token.into(),
        },
    );
}

#[test]
fn authority_only_falls_back_to_inbox() {
    // Empty path or `inbox`-only authority both render the inbox
    // view rather than erroring — the user tapped a notification
    // with no specific approval id, so show them the list.
    assert_eq!(parse("argo://").unwrap(), DeepLinkIntent::Inbox);
    assert_eq!(parse("argo:///").unwrap(), DeepLinkIntent::Inbox);
    assert_eq!(parse("argo://inbox").unwrap(), DeepLinkIntent::Inbox);
    assert_eq!(parse("argo://inbox/").unwrap(), DeepLinkIntent::Inbox);
}

#[test]
fn empty_or_non_url_input_is_malformed() {
    assert_eq!(parse("").unwrap_err(), DeepLinkError::Malformed);
    assert_eq!(parse("   ").unwrap_err(), DeepLinkError::Malformed);
    assert_eq!(parse("not a url").unwrap_err(), DeepLinkError::Malformed);
    assert_eq!(parse("argo:approve/foo").unwrap_err(), DeepLinkError::Malformed);
}

#[test]
fn wrong_scheme_is_rejected_with_captured_scheme() {
    assert_eq!(
        parse("https://example.com/approve/act-1").unwrap_err(),
        DeepLinkError::WrongScheme("https".into()),
    );
    assert_eq!(
        parse("skycot://approve/act-1").unwrap_err(),
        DeepLinkError::WrongScheme("skycot".into()),
    );
}

#[test]
fn missing_path_returns_missing_path_error() {
    assert_eq!(
        parse("argo://approve").unwrap_err(),
        DeepLinkError::MissingPath("approve".into()),
    );
    assert_eq!(
        parse("argo://approve/").unwrap_err(),
        DeepLinkError::MissingPath("approve".into()),
    );
    assert_eq!(
        parse("argo://reject").unwrap_err(),
        DeepLinkError::MissingPath("reject".into()),
    );
    assert_eq!(
        parse("argo://inbox/approval").unwrap_err(),
        DeepLinkError::MissingPath("inbox/approval".into()),
    );
    assert_eq!(
        parse("argo://inbox/approval/").unwrap_err(),
        DeepLinkError::MissingPath("inbox/approval".into()),
    );
}

#[test]
fn unrecognised_verb_returns_unknown_action() {
    assert_eq!(
        parse("argo://snooze/act-1").unwrap_err(),
        DeepLinkError::UnknownAction("snooze".into()),
    );
    assert_eq!(
        parse("argo://chain/act-1").unwrap_err(),
        DeepLinkError::UnknownAction("chain".into()),
    );
}

#[test]
fn query_and_fragment_are_ignored_for_routing() {
    // Query/fragment exist for forward compatibility (analytics,
    // UTM tags) and must not change the intent.
    assert_eq!(
        parse("argo://approve/act-1?utm_source=push").unwrap(),
        DeepLinkIntent::Approve {
            action_id: "act-1".into(),
        },
    );
    assert_eq!(
        parse("argo://approve/act-1#focus").unwrap(),
        DeepLinkIntent::Approve {
            action_id: "act-1".into(),
        },
    );
    assert_eq!(
        parse("argo://inbox/approval/tok.1.sig?from=email").unwrap(),
        DeepLinkIntent::InboxApproval {
            token: "tok.1.sig".into(),
        },
    );
}

#[test]
fn scheme_match_is_case_insensitive() {
    // Some platforms (notably some Android intent handlers) upper-
    // case the scheme. Treat `ARGO://` as equivalent to `argo://`.
    assert_eq!(
        parse("ARGO://approve/act-1").unwrap(),
        DeepLinkIntent::Approve {
            action_id: "act-1".into(),
        },
    );
}

#[test]
fn surrounding_whitespace_is_tolerated() {
    // Push payloads occasionally arrive with stray newline padding
    // (vendor-side templating). Don't fail the parse over it.
    assert_eq!(
        parse("  argo://approve/act-1\n").unwrap(),
        DeepLinkIntent::Approve {
            action_id: "act-1".into(),
        },
    );
}
