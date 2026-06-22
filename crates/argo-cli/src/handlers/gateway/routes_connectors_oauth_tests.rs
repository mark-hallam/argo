//! Unit tests for the connector OAuth connect/callback routing + readiness.
//! Pure (no TCP, no env mutation) so they stay deterministic.

use super::*;

#[test]
fn matches_only_connect_and_callback_under_connectors() {
    assert!(is_connector_oauth_request(
        "GET /api/v1/connectors/gmail/connect HTTP/1.1\r\n"
    ));
    assert!(is_connector_oauth_request(
        "GET /api/v1/connectors/hubspot/callback?code=x&state=y HTTP/1.1\r\n"
    ));
    // Not the credential-POST authorize path, not the roster, not other routes.
    assert!(!is_connector_oauth_request(
        "POST /api/v1/connectors/gmail/authorize HTTP/1.1\r\n"
    ));
    assert!(!is_connector_oauth_request("GET /api/v1/connectors HTTP/1.1\r\n"));
    assert!(!is_connector_oauth_request(
        "GET /api/v1/workers/w1/connectors/gmail/connect HTTP/1.1\r\n"
    ));
    // Method-gated: only GET (the browser navigation) is the OAuth dance.
    assert!(!is_connector_oauth_request(
        "POST /api/v1/connectors/gmail/connect HTTP/1.1\r\n"
    ));
}

#[test]
fn oauth_ready_rejects_a_token_connector_before_touching_env() {
    // slack is token-auth — must short-circuit to `not_oauth` (no client-id read).
    assert_eq!(oauth_ready("slack").err(), Some("not_oauth"));
}

#[test]
fn oauth_ready_rejects_an_unknown_connector() {
    assert_eq!(oauth_ready("not-a-real-connector").err(), Some("unknown_connector"));
}

#[test]
fn oauth_ready_treats_gmail_as_a_known_oauth_connector() {
    // gmail IS an OAuth connector. Without a registered app it returns
    // `not_configured` (the inert-until-operator-registers state); with one it
    // returns Ok. Either way it must NOT be `not_oauth`/`unknown_connector`.
    let err = oauth_ready("gmail").err();
    assert_ne!(err, Some("not_oauth"));
    assert_ne!(err, Some("unknown_connector"));
}

#[test]
fn callback_uri_always_targets_this_connector_callback_path() {
    let uri = callback_uri("GET /x HTTP/1.1\r\nHost: api.skycot.com\r\n", "gmail");
    assert!(
        uri.ends_with("/api/v1/connectors/gmail/callback"),
        "got {uri}"
    );
    // Derives the host from the request when no override env is set.
    if std::env::var("ARGO_OAUTH_REDIRECT_BASE").is_err() {
        assert_eq!(uri, "https://api.skycot.com/api/v1/connectors/gmail/callback");
    }
}
