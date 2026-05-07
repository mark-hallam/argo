//! Phrase grammar for the voice TUI.
//!
//! A recognized utterance (already wake-word-stripped) is split
//! into a leading verb token and an optional remainder. Grammar
//! is intentionally tiny — five verbs, optional name argument:
//!
//! ```text
//! approve [<name>]
//! deny    [<name>]
//! details [<name>]
//! next
//! quit
//! ```
//!
//! Token-level normalization: lowercase, strip punctuation, collapse
//! whitespace. Returns [`None`] for unrecognized utterances —
//! callers should treat that as "unknown phrase, prompt again".

use serde::{Deserialize, Serialize};

/// A parsed voice phrase. Wraps the [`Verb`] and the optional
/// remainder of the utterance (used as the worker-name argument for
/// approve / deny / details).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Phrase {
    /// The leading verb of the utterance.
    pub verb: Verb,
    /// Remaining tokens joined by single spaces. Empty for `next`
    /// and `quit`; may be empty for the others if the operator did
    /// not name a worker. Always lowercase, punctuation-stripped.
    pub argument: String,
}

/// The five verbs the voice TUI accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Verb {
    /// `approve [<name>]` — approve a pending action.
    Approve,
    /// `deny [<name>]` — deny a pending action.
    Deny,
    /// `details [<name>]` — show full action detail (does not act).
    Details,
    /// `next` — advance to the next pending action.
    Next,
    /// `quit` — exit voice mode.
    Quit,
}

impl Verb {
    fn from_token(t: &str) -> Option<Self> {
        match t {
            "approve" => Some(Self::Approve),
            "deny" => Some(Self::Deny),
            "details" | "detail" => Some(Self::Details),
            "next" => Some(Self::Next),
            "quit" | "exit" => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Lowercase + punctuation-strip + whitespace-split.
///
/// Whisper.cpp output often arrives with trailing punctuation
/// ("approve.") and casing ("Approve"). Normalize both before
/// matching the verb table.
pub fn tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|tok| {
            tok.chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
                .flat_map(|c| c.to_lowercase())
                .collect::<String>()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a recognized phrase. Returns [`None`] when the leading
/// token is not a known verb — caller should reprompt.
pub fn parse(text: &str) -> Option<Phrase> {
    let toks = tokens(text);
    let (head, rest) = toks.split_first()?;
    let verb = Verb::from_token(head)?;
    let argument = rest.join(" ");
    // `next` and `quit` take no argument; if any extra tokens were
    // captured, drop them — the verb still wins.
    let argument = match verb {
        Verb::Next | Verb::Quit => String::new(),
        _ => argument,
    };
    Some(Phrase { verb, argument })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approve_with_name() {
        let p = parse("approve Maria").expect("parses");
        assert_eq!(p.verb, Verb::Approve);
        assert_eq!(p.argument, "maria");
    }

    #[test]
    fn deny_with_punctuation() {
        let p = parse("Deny, Maya.").expect("parses");
        assert_eq!(p.verb, Verb::Deny);
        assert_eq!(p.argument, "maya");
    }

    #[test]
    fn details_alias() {
        let p = parse("detail riley").expect("parses");
        assert_eq!(p.verb, Verb::Details);
        assert_eq!(p.argument, "riley");
    }

    #[test]
    fn next_no_argument() {
        let p = parse("next worker").expect("parses");
        assert_eq!(p.verb, Verb::Next);
        // arg dropped for next/quit verbs
        assert_eq!(p.argument, "");
    }

    #[test]
    fn quit_alias() {
        let p = parse("Exit").expect("parses");
        assert_eq!(p.verb, Verb::Quit);
        assert_eq!(p.argument, "");
    }

    #[test]
    fn unknown_phrase_returns_none() {
        assert!(parse("schedule a meeting").is_none());
        assert!(parse("").is_none());
        assert!(parse("    ").is_none());
    }

    #[test]
    fn name_with_hyphen_preserved() {
        let p = parse("approve Mary-Jane Smith").expect("parses");
        assert_eq!(p.verb, Verb::Approve);
        assert_eq!(p.argument, "mary-jane smith");
    }
}
