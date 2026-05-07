//! Voice loop runner.
//!
//! [`run_one_tick`] takes a recognizer + the current pending
//! approvals + the focused index and produces a [`TickOutcome`]
//! describing what the operator wants to do (approve / deny /
//! advance / quit). The CLI handler is the single place that
//! turns those outcomes into HTTP calls to `/decisions/<token>`;
//! this module is pure logic so it stays unit-testable without
//! gateway state.

use crate::confirm::{format_confirmation, parse_confirmation_response, ConfirmationResponse};
use crate::fuzzy::best_match;
use crate::intent::{classify, Intent};
use crate::recognizer::{Recognizer, RecognizerError};
use crate::wake_word::strip_wake_word;

/// One pending approval, summarized for the voice runner.
///
/// Keeps the field set minimal — the runner only needs to identify
/// the worker for fuzzy matching, render a confirmation summary,
/// and (eventually) hand the token back to the CLI handler so it
/// can call the HTTP decision route.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingApproval {
    /// Approval token (HMAC-signed, used in the decision URL).
    pub token: String,
    /// Display name of the worker (used for fuzzy matching).
    pub worker_name: String,
    /// One-line summary of what the action would do.
    pub action_summary: String,
    /// Spend amount in dollars, if the action declares one.
    /// `None` for actions with no cost.
    pub spend_usd: Option<f64>,
}

/// Configuration for one runner tick. Currently just the fuzzy-match
/// edit-distance cap; future config (timeout, min-confidence, etc.)
/// joins this struct.
#[derive(Debug, Clone)]
pub struct RunnerConfig {
    /// Max Levenshtein distance for worker-name fuzzy match.
    /// Brief default: 2.
    pub max_fuzzy_distance: usize,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self { max_fuzzy_distance: 2 }
    }
}

/// Result of one runner tick. Pure data — the CLI handler
/// transports these over HTTP.
#[derive(Debug, Clone, PartialEq)]
pub enum TickOutcome {
    /// Operator confirmed an approve action. Payload: token.
    Approved(String),
    /// Operator confirmed a deny action. Payload: token.
    Denied(String),
    /// Operator asked for action details. Payload: token (so the
    /// CLI can fetch + render the full payload).
    DetailsRequested(String),
    /// Operator wants to advance focus to the next pending action.
    Next,
    /// Operator confirmed quit.
    Quit,
    /// Operator was reprompted (unknown phrase, missing wake word,
    /// fuzzy match failed, confirmation was "no" or unclear). The
    /// runner emitted a message which the CLI handler renders;
    /// loop continues.
    Reprompted(String),
    /// Recognizer signalled a backend error (no mic, no model,
    /// etc.). CLI handler should fall back to the TUI list view.
    Error(String),
}

/// Drive the recognizer for one tick.
///
/// Pulls one utterance, gates on wake word, classifies the intent,
/// resolves worker name (if any) against `pending`, prompts +
/// reads confirmation for approve / deny, and returns the resulting
/// [`TickOutcome`]. Recognizer is mutated (mocks consume from
/// their script).
///
/// The `current_focus_index` is the operator's currently-focused
/// pending approval, used when the operator says `approve` /
/// `deny` without naming a worker.
pub fn run_one_tick(
    recognizer: &mut dyn Recognizer,
    pending: &[PendingApproval],
    current_focus_index: usize,
    cfg: &RunnerConfig,
) -> TickOutcome {
    let utterance = match recognizer.transcribe_next() {
        Ok(t) if t.trim().is_empty() => return TickOutcome::Reprompted("no speech detected".into()),
        Ok(t) => t,
        Err(RecognizerError::Backend(msg)) => return TickOutcome::Error(msg),
        Err(RecognizerError::Exhausted) => {
            return TickOutcome::Error("recognizer exhausted".into())
        }
    };
    let Some(after_wake) = strip_wake_word(&utterance) else {
        return TickOutcome::Reprompted(format!(
            "ignored (no wake word): {}",
            utterance.trim()
        ));
    };
    if after_wake.trim().is_empty() {
        return TickOutcome::Reprompted("wake word only — say a command".into());
    }
    match classify(&after_wake) {
        Intent::ApproveCurrent => decide_current("approve", pending, current_focus_index, recognizer),
        Intent::DenyCurrent => decide_current("deny", pending, current_focus_index, recognizer),
        Intent::ApproveSpecific(name) => {
            decide_specific("approve", pending, &name, cfg, recognizer)
        }
        Intent::DenySpecific(name) => decide_specific("deny", pending, &name, cfg, recognizer),
        Intent::Details(None) => {
            if let Some(p) = pending.get(current_focus_index) {
                TickOutcome::DetailsRequested(p.token.clone())
            } else {
                TickOutcome::Reprompted("no pending approval focused".into())
            }
        }
        Intent::Details(Some(name)) => {
            let pool: Vec<&str> = pending.iter().map(|p| p.worker_name.as_str()).collect();
            match best_match(&name, &pool, cfg.max_fuzzy_distance) {
                Some(matched) => {
                    if let Some(p) =
                        pending.iter().find(|p| p.worker_name.as_str() == matched)
                    {
                        TickOutcome::DetailsRequested(p.token.clone())
                    } else {
                        TickOutcome::Reprompted(format!("worker '{name}' not found"))
                    }
                }
                None => TickOutcome::Reprompted(format!("worker '{name}' not found")),
            }
        }
        Intent::Next => TickOutcome::Next,
        Intent::Quit => TickOutcome::Quit,
        Intent::Unknown => {
            TickOutcome::Reprompted(format!("unknown phrase: {}", after_wake.trim()))
        }
    }
}

fn decide_current(
    verb: &str,
    pending: &[PendingApproval],
    idx: usize,
    recognizer: &mut dyn Recognizer,
) -> TickOutcome {
    let Some(p) = pending.get(idx) else {
        return TickOutcome::Reprompted("no pending approval focused".into());
    };
    confirm_then(verb, p, recognizer)
}

fn decide_specific(
    verb: &str,
    pending: &[PendingApproval],
    name: &str,
    cfg: &RunnerConfig,
    recognizer: &mut dyn Recognizer,
) -> TickOutcome {
    let pool: Vec<&str> = pending.iter().map(|p| p.worker_name.as_str()).collect();
    let Some(matched) = best_match(name, &pool, cfg.max_fuzzy_distance) else {
        return TickOutcome::Reprompted(format!("worker '{name}' not found"));
    };
    let Some(p) = pending.iter().find(|p| p.worker_name.as_str() == matched) else {
        return TickOutcome::Reprompted(format!("worker '{name}' not found"));
    };
    confirm_then(verb, p, recognizer)
}

fn confirm_then(
    verb: &str,
    p: &PendingApproval,
    recognizer: &mut dyn Recognizer,
) -> TickOutcome {
    let prompt =
        format_confirmation(verb, &p.worker_name, &p.action_summary, p.spend_usd);
    tracing::info!(target: "argo_voice_tui::runner", confirmation = %prompt);
    let response = match recognizer.transcribe_next() {
        Ok(t) => t,
        Err(RecognizerError::Backend(msg)) => return TickOutcome::Error(msg),
        Err(RecognizerError::Exhausted) => {
            return TickOutcome::Reprompted("confirmation timed out".into())
        }
    };
    match parse_confirmation_response(&response) {
        ConfirmationResponse::Yes => match verb {
            "approve" => TickOutcome::Approved(p.token.clone()),
            "deny" => TickOutcome::Denied(p.token.clone()),
            _ => TickOutcome::Reprompted(format!("unknown verb: {verb}")),
        },
        ConfirmationResponse::No => TickOutcome::Reprompted(format!("{verb} cancelled")),
        ConfirmationResponse::Other => {
            TickOutcome::Reprompted(format!("confirmation unclear: {}", response.trim()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recognizer::MockRecognizer;

    fn pending_fixture() -> Vec<PendingApproval> {
        vec![
            PendingApproval {
                token: "tok-maya".into(),
                worker_name: "Maya".into(),
                action_summary: "send invoice reminder".into(),
                spend_usd: Some(0.0),
            },
            PendingApproval {
                token: "tok-maria".into(),
                worker_name: "Maria Hernandez".into(),
                action_summary: "log payment receipt".into(),
                spend_usd: Some(25.0),
            },
        ]
    }

    #[test]
    fn approve_specific_with_yes_confirms() {
        let mut r =
            MockRecognizer::new(["Argo approve Maria", "yes"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        assert_eq!(outcome, TickOutcome::Approved("tok-maria".into()));
    }

    #[test]
    fn deny_with_no_response_cancels() {
        let mut r = MockRecognizer::new(["Argo deny Maya", "no"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        let TickOutcome::Reprompted(msg) = outcome else {
            panic!("expected reprompt, got {:?}", outcome);
        };
        assert!(msg.contains("cancelled"));
    }

    #[test]
    fn missing_wake_word_drops_utterance() {
        let mut r = MockRecognizer::new(["approve Maria"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        let TickOutcome::Reprompted(msg) = outcome else {
            panic!("expected reprompt, got {:?}", outcome);
        };
        assert!(msg.contains("no wake word"));
    }

    #[test]
    fn next_advances_focus_signal() {
        let mut r = MockRecognizer::new(["Argo next"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        assert_eq!(outcome, TickOutcome::Next);
    }

    #[test]
    fn quit_propagates() {
        let mut r = MockRecognizer::new(["Argo exit"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        assert_eq!(outcome, TickOutcome::Quit);
    }

    #[test]
    fn fuzzy_resolves_misheard_name() {
        // "Mariah" → fuzzy match "Maria Hernandez"
        let mut r = MockRecognizer::new(["Argo approve Mariah", "yes"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        assert_eq!(outcome, TickOutcome::Approved("tok-maria".into()));
    }

    #[test]
    fn unknown_worker_reprompts() {
        let mut r = MockRecognizer::new(["Argo approve Zaphod"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        let TickOutcome::Reprompted(msg) = outcome else {
            panic!("expected reprompt, got {:?}", outcome);
        };
        assert!(msg.contains("zaphod") || msg.contains("not found"));
    }

    #[test]
    fn backend_error_propagates() {
        // Empty mock → exhausted → Error outcome
        let mut r = MockRecognizer::new::<[String; 0], String>([]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 0, &RunnerConfig::default());
        assert!(matches!(outcome, TickOutcome::Error(_)));
    }

    #[test]
    fn details_uses_focused_index_when_no_argument() {
        let mut r = MockRecognizer::new(["Argo details"]);
        let outcome =
            run_one_tick(&mut r, &pending_fixture(), 1, &RunnerConfig::default());
        assert_eq!(outcome, TickOutcome::DetailsRequested("tok-maria".into()));
    }
}
