//! Wake-word gating.
//!
//! Voice mode does not act on every recognized utterance — it
//! requires the prefix `argo` (case insensitive). This is the
//! battery + privacy gate: anything you say without leading "argo"
//! is dropped, never dispatched.
//!
//! The whisper.cpp output is normalized — leading punctuation and
//! casing are stripped before the wake-word check.

/// If `text` begins with the wake word `argo` (case insensitive,
/// any leading whitespace / punctuation tolerated), return the
/// remainder with the wake word stripped. Otherwise return [`None`]
/// — caller should drop the utterance.
///
/// ```
/// use argo_voice_tui::strip_wake_word;
/// assert_eq!(strip_wake_word("Argo approve Maya"), Some("approve Maya".to_string()));
/// assert_eq!(strip_wake_word("approve Maya"), None);
/// ```
pub fn strip_wake_word(text: &str) -> Option<String> {
    let trimmed = text
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim();
    let mut tokens = trimmed.split_whitespace();
    let head = tokens.next()?.to_lowercase();
    // Strip trailing punctuation so "Argo," still matches.
    let head_clean: String = head
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    if head_clean != "argo" {
        return None;
    }
    let rest: Vec<&str> = tokens.collect();
    if rest.is_empty() {
        // Just the wake word with no command — caller should
        // reprompt rather than treat this as a valid intent.
        return Some(String::new());
    }
    Some(rest.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_word_stripped() {
        assert_eq!(
            strip_wake_word("Argo approve Maya"),
            Some("approve Maya".into())
        );
    }

    #[test]
    fn wake_word_case_insensitive() {
        assert_eq!(
            strip_wake_word("ARGO next"),
            Some("next".into())
        );
    }

    #[test]
    fn missing_wake_word_returns_none() {
        assert!(strip_wake_word("approve Maya").is_none());
        assert!(strip_wake_word("").is_none());
    }

    #[test]
    fn wake_word_with_trailing_punctuation() {
        assert_eq!(
            strip_wake_word("Argo, approve Maria"),
            Some("approve Maria".into())
        );
    }

    #[test]
    fn wake_word_alone_returns_empty_remainder() {
        assert_eq!(strip_wake_word("Argo"), Some(String::new()));
    }

    #[test]
    fn similar_word_does_not_trigger() {
        // "argos" / "argon" must NOT match — strict equality.
        assert!(strip_wake_word("argon approve").is_none());
        assert!(strip_wake_word("argos next").is_none());
    }

    #[test]
    fn leading_whitespace_and_punctuation_tolerated() {
        assert_eq!(
            strip_wake_word("  ... Argo deny Bob"),
            Some("deny Bob".into())
        );
    }
}
