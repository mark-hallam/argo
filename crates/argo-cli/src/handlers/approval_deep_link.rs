//! Pure-Rust parser for `argo://` deep links arriving from mobile
//! push notifications.
//!
//! The mobile shell (`mobile/argo-approve/`) handles deep links in
//! TypeScript today, but the URL grammar is defined by Rust call
//! sites — see `crates/argo-channels/src/push.rs` (constructs
//! `argo://inbox/approval/<token>`) and
//! `crates/argo-cli/src/handlers/worker_actions_notify.rs`
//! (constructs `argo://approve/<action_id>` and
//! `argo://reject/<action_id>`). This module gives the gateway and
//! any future Rust-side renderer (e.g. a desktop approval UI) a
//! shared, unit-tested parser so the grammar stays consistent.
//!
//! The parser is total: malformed input returns a typed
//! [`DeepLinkError`] rather than panicking. An empty or
//! authority-only URL falls back to [`DeepLinkIntent::Inbox`] so the
//! caller can render the inbox view rather than an error screen.

// `argo-cli` is a binary-only crate (no `lib.rs`), so dead-code
// analysis flags `pub` helpers that no `main.rs` arm has wired up
// yet. This module is preparatory: it ships a unit-tested grammar
// today so the desktop / gateway approval UI can call it without
// duplicating the parser. The follow-up brief that wires the
// handler removes this allowance.
#![allow(dead_code)]

/// What the user wants to do, derived from a deep-link URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLinkIntent {
    /// `argo://inbox/approval/<token>` — open the inbox view and
    /// highlight the approval identified by the push-decision token.
    InboxApproval { token: String },
    /// `argo://approve/<action_id>` — open the approve flow for the
    /// given action.
    Approve { action_id: String },
    /// `argo://reject/<action_id>` — open the reject flow for the
    /// given action.
    Reject { action_id: String },
    /// `argo://`, `argo:///`, or `argo://inbox` — fall back to the
    /// inbox view with no specific approval highlighted.
    Inbox,
}

/// Why a URL could not be turned into a [`DeepLinkIntent`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLinkError {
    /// Empty input, or input that does not contain a `<scheme>://`
    /// separator.
    Malformed,
    /// Scheme was something other than `argo`. The captured scheme
    /// is included so the caller can log the unexpected value.
    WrongScheme(String),
    /// A recognised verb was missing its required path tail (for
    /// example, `argo://approve` with no action id).
    MissingPath(String),
    /// The first path segment is not a recognised verb.
    UnknownAction(String),
}

/// Parse a deep-link URL into a [`DeepLinkIntent`].
///
/// The grammar mirrors the URLs constructed in `argo-channels` and
/// `argo-cli`'s notify handler. Query strings and fragments are
/// stripped before routing — they are reserved for forward
/// compatibility (e.g. analytics tags) and do not affect intent
/// today.
///
/// # Examples
///
/// ```text
/// argo://approve/act-1            → DeepLinkIntent::Approve { action_id: "act-1" }
/// argo://reject/act-2             → DeepLinkIntent::Reject  { action_id: "act-2" }
/// argo://inbox/approval/<token>   → DeepLinkIntent::InboxApproval { token }
/// argo://                         → DeepLinkIntent::Inbox  (fallback)
/// argo://snooze/x                 → Err(DeepLinkError::UnknownAction("snooze"))
/// ```
///
/// (Block is `text` rather than `rust` because `argo-cli` is a
/// binary-only crate with no `lib.rs` for `cargo test --doc` to
/// resolve `argo_cli::handlers::...`.)
pub fn parse(raw: &str) -> Result<DeepLinkIntent, DeepLinkError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(DeepLinkError::Malformed);
    }
    let (scheme, rest) = trimmed.split_once("://").ok_or(DeepLinkError::Malformed)?;
    if !scheme.eq_ignore_ascii_case("argo") {
        return Err(DeepLinkError::WrongScheme(scheme.to_string()));
    }

    // Strip optional query / fragment; they are not load-bearing
    // for routing today.
    let path = rest.split(['?', '#']).next().unwrap_or("");
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    match segments.as_slice() {
        [] | ["inbox"] => Ok(DeepLinkIntent::Inbox),
        ["inbox", "approval"] => Err(DeepLinkError::MissingPath("inbox/approval".into())),
        ["inbox", "approval", token, ..] => Ok(DeepLinkIntent::InboxApproval {
            token: (*token).to_string(),
        }),
        ["approve"] => Err(DeepLinkError::MissingPath("approve".into())),
        ["approve", action_id, ..] => Ok(DeepLinkIntent::Approve {
            action_id: (*action_id).to_string(),
        }),
        ["reject"] => Err(DeepLinkError::MissingPath("reject".into())),
        ["reject", action_id, ..] => Ok(DeepLinkIntent::Reject {
            action_id: (*action_id).to_string(),
        }),
        [action, ..] => Err(DeepLinkError::UnknownAction((*action).to_string())),
    }
}

#[cfg(test)]
#[path = "approval_deep_link_tests.rs"]
mod tests;
