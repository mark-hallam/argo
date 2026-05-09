//! Tests for the Genesis prompt-engineering revision loop.

use super::*;

fn fifty_word_role() -> String {
    [
        "Be my customer support teammate for priority enterprise ticket triage, renewal-risk",
        "routing, customer update drafting, evidence collection, escalation summaries, account",
        "context recall, weekly trend reporting, incident follow-up, bug reproduction notes,",
        "and operator-approved response suggestions across email, Slack, and web inbox",
        "channels during business hours only.",
    ]
    .join(" ")
}

#[test]
fn start_emits_non_empty_prompt_for_one_line_role() {
    let session = start("coordinate customer support tickets and draft careful replies").unwrap();
    assert_eq!(session.history.len(), 1);
    assert_eq!(session.history[0].rev, 0);
    assert!(!session.current_prompt.trim().is_empty());
}

#[test]
fn start_emits_at_least_two_hundred_words_for_fifty_word_role() {
    let session = start(&fifty_word_role()).unwrap();
    assert!(
        word_count(&session.current_prompt) >= MIN_INITIAL_PROMPT_WORDS,
        "prompt too short: {} words",
        word_count(&session.current_prompt)
    );
}

#[test]
fn revise_with_terse_feedback_shortens_prompt() {
    let mut session = start(&fifty_word_role()).unwrap();
    let before_len = session.current_prompt.len();
    let revision = revise(&mut session, "be more terse").unwrap();
    assert_eq!(revision.rev, 1);
    assert!(
        revision.prompt.len() < before_len,
        "revision was not shorter: {} >= {}",
        revision.prompt.len(),
        before_len
    );
}

#[test]
fn locked_sections_survive_revision_verbatim() {
    let mut session = start("be my marketing intern and draft social campaigns").unwrap();
    let locked = "[KEEP]Always cite campaign source URLs before drafting.[/KEEP]";
    session.current_prompt.push_str("\n\n");
    session.current_prompt.push_str(locked);
    let revision = revise(&mut session, "shorter").unwrap();
    assert!(revision.prompt.contains(locked));
    assert_eq!(revision.locked_sections, vec![locked.to_string()]);
}

#[test]
fn four_revisions_are_monotonic() {
    let mut session = start("be my project pm for sprint milestones and status reports").unwrap();
    for feedback in [
        "shorter",
        "add blockers",
        "be more conservative",
        "shorter again",
    ] {
        revise(&mut session, feedback).unwrap();
    }
    let revs: Vec<u32> = session.history.iter().map(|r| r.rev).collect();
    assert_eq!(revs, vec![0, 1, 2, 3, 4]);
}

#[test]
fn empty_role_description_errors_with_validation() {
    let err = start("   ").expect_err("empty role should fail");
    assert!(matches!(
        err,
        GenesisError::Validation {
            stage: "prompt_engineer.start",
            ..
        }
    ));
}
