//! Recognized phrase → high-level [`Intent`] for the runner to
//! dispatch.
//!
//! Distinguishes [`Intent::Approve`] from [`Intent::ApproveSpecific`]
//! (with a worker-name argument) so the runner can handle the
//! "no worker named — apply to currently-focused approval" case
//! cleanly.

use serde::{Deserialize, Serialize};

use crate::grammar::{parse, Verb};

/// High-level voice command, ready for the runner to dispatch
/// against the pending-approval list. Each variant maps 1:1 to a
/// CLI action shape; the runner is the single place that turns
/// these into HTTP calls (or simulated decisions in tests).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    /// `approve` (no argument) — operate on the currently-focused
    /// pending approval.
    ApproveCurrent,
    /// `approve <name>` — fuzzy-resolve `name` against pending list.
    ApproveSpecific(String),
    /// `deny` (no argument) — currently-focused pending approval.
    DenyCurrent,
    /// `deny <name>` — fuzzy-resolve `name`.
    DenySpecific(String),
    /// `details` / `details <name>` — show full action payload.
    Details(Option<String>),
    /// `next` — advance focus to the next pending approval.
    Next,
    /// `quit` / `exit` — leave voice mode.
    Quit,
    /// Unrecognized utterance — runner reprompts.
    Unknown,
}

/// Classify a recognized utterance into an [`Intent`].
///
/// Returns [`Intent::Unknown`] for anything the [`grammar`] layer
/// rejects. Wake-word stripping is the caller's responsibility —
/// this function trusts the input.
///
/// ```
/// use argo_voice_tui::{classify, Intent};
/// assert_eq!(classify("approve Maya"), Intent::ApproveSpecific("maya".into()));
/// assert_eq!(classify("next"), Intent::Next);
/// assert_eq!(classify("schedule meeting"), Intent::Unknown);
/// ```
pub fn classify(text: &str) -> Intent {
    let Some(phrase) = parse(text) else {
        return Intent::Unknown;
    };
    let arg = phrase.argument;
    match phrase.verb {
        Verb::Approve if arg.is_empty() => Intent::ApproveCurrent,
        Verb::Approve => Intent::ApproveSpecific(arg),
        Verb::Deny if arg.is_empty() => Intent::DenyCurrent,
        Verb::Deny => Intent::DenySpecific(arg),
        Verb::Details if arg.is_empty() => Intent::Details(None),
        Verb::Details => Intent::Details(Some(arg)),
        Verb::Next => Intent::Next,
        Verb::Quit => Intent::Quit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_with_worker_name() {
        assert_eq!(
            classify("approve Maya"),
            Intent::ApproveSpecific("maya".into())
        );
    }

    #[test]
    fn approve_no_argument_is_current() {
        assert_eq!(classify("approve"), Intent::ApproveCurrent);
    }

    #[test]
    fn details_optional_argument() {
        assert_eq!(classify("details"), Intent::Details(None));
        assert_eq!(
            classify("details riley"),
            Intent::Details(Some("riley".into()))
        );
    }

    #[test]
    fn next_and_quit() {
        assert_eq!(classify("next"), Intent::Next);
        assert_eq!(classify("quit"), Intent::Quit);
        assert_eq!(classify("exit"), Intent::Quit);
    }

    #[test]
    fn unknown_returns_unknown_variant() {
        assert_eq!(classify("schedule a call"), Intent::Unknown);
        assert_eq!(classify(""), Intent::Unknown);
    }

    #[test]
    fn deny_with_two_word_name() {
        assert_eq!(
            classify("deny Maria Hernandez"),
            Intent::DenySpecific("maria hernandez".into())
        );
    }
}
