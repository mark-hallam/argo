//! Confirmation prompt formatting + parsing.
//!
//! Approve / deny intents do NOT take effect on first utterance —
//! the runner formats a one-line summary and waits for an explicit
//! "yes" / "no". This is the safety guard for voice (the operator
//! can mis-speak, the recognizer can mis-hear).
//!
//! Pure-string layer: takes a worker name + action summary +
//! optional spend amount, returns the text to render. Parsing the
//! follow-up response to Yes / No / Other is also here for
//! symmetry — the runner pipes recognized text into
//! [`parse_confirmation_response`].

/// Format the safety prompt the operator hears before a destructive
/// action takes effect.
///
/// Example output:
///
/// ```text
/// approve Maya for $25 invoice charge — say YES to confirm.
/// ```
///
/// `spend` is optional because not every action has a cost (e.g.
/// approving a non-spending email-send still wants a confirmation).
pub fn format_confirmation(verb: &str, worker: &str, action_summary: &str, spend: Option<f64>) -> String {
    let spend_chunk = match spend {
        Some(amount) if amount > 0.0 => format!(" for ${amount:.2}"),
        _ => String::new(),
    };
    format!(
        "{verb} {worker}{spend_chunk} {action_summary} — say YES to confirm."
    )
}

/// Operator response to a confirmation prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationResponse {
    /// `yes` / `confirm` / `yeah` — proceed with the action.
    Yes,
    /// `no` / `cancel` / `nope` — abort the action.
    No,
    /// Anything else — runner should reprompt or fall back to
    /// silent abort after a timeout.
    Other,
}

/// Parse a recognized response phrase into [`ConfirmationResponse`].
///
/// Liberal in what it accepts: case insensitive, single-token match.
/// Multi-word responses use only the first content word —
/// "yes please" → Yes, "no thanks" → No.
pub fn parse_confirmation_response(text: &str) -> ConfirmationResponse {
    let normalized: String = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect();
    let head = normalized.split_whitespace().next().unwrap_or("");
    match head {
        "yes" | "yeah" | "confirm" | "ok" | "okay" | "y" => ConfirmationResponse::Yes,
        "no" | "nope" | "cancel" | "abort" | "n" => ConfirmationResponse::No,
        _ => ConfirmationResponse::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_with_spend() {
        let out = format_confirmation("approve", "Maya", "invoice charge", Some(25.0));
        assert!(out.contains("approve Maya"));
        assert!(out.contains("$25.00"));
        assert!(out.contains("YES to confirm"));
    }

    #[test]
    fn confirmation_without_spend() {
        let out = format_confirmation("deny", "Riley", "send email", None);
        assert!(out.contains("deny Riley"));
        assert!(!out.contains("$"));
    }

    #[test]
    fn parse_yes_variants() {
        assert_eq!(parse_confirmation_response("yes"), ConfirmationResponse::Yes);
        assert_eq!(parse_confirmation_response("Yeah!"), ConfirmationResponse::Yes);
        assert_eq!(
            parse_confirmation_response("Yes please"),
            ConfirmationResponse::Yes
        );
        assert_eq!(parse_confirmation_response("CONFIRM"), ConfirmationResponse::Yes);
    }

    #[test]
    fn parse_no_variants() {
        assert_eq!(parse_confirmation_response("no"), ConfirmationResponse::No);
        assert_eq!(parse_confirmation_response("Cancel"), ConfirmationResponse::No);
        assert_eq!(
            parse_confirmation_response("Nope, hold up"),
            ConfirmationResponse::No
        );
    }

    #[test]
    fn parse_other_falls_through() {
        assert_eq!(
            parse_confirmation_response("maybe"),
            ConfirmationResponse::Other
        );
        assert_eq!(parse_confirmation_response(""), ConfirmationResponse::Other);
    }
}
