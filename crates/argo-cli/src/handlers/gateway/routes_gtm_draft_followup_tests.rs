//! Unit tests for Grant's GTM draft-followup endpoint. These drive the
//! pure pieces (prompt build, truth-boundary swap, system-prompt safety)
//! without a live provider — the live-LLM path is exercised by the
//! gateway smoke verification, not here.

use super::*;

fn signup(name: &str, email: &str, context: Option<&str>) -> Signup {
    Signup {
        name: name.to_string(),
        email: email.to_string(),
        context: context.map(str::to_string),
    }
}

#[test]
fn user_prompt_includes_name_email_and_context() {
    let s = signup("Dana Reed", "dana@acme.io", Some("Runs ops at a 40-person startup"));
    let prompt = build_user_prompt(&s, &qualify(&s));
    assert!(prompt.contains("Dana Reed"), "name must appear: {prompt}");
    assert!(prompt.contains("dana@acme.io"), "email must appear: {prompt}");
    assert!(
        prompt.contains("Runs ops at a 40-person startup"),
        "context must appear: {prompt}"
    );
}

#[test]
fn user_prompt_without_context_instructs_generic_and_no_invention() {
    let s = signup("Sam", "", None);
    let prompt = build_user_prompt(&s, &qualify(&s));
    assert!(prompt.contains("Sam"));
    assert!(
        prompt.contains("do not invent"),
        "no-context prompt must forbid inventing detail: {prompt}"
    );
    // No email line when email is blank.
    assert!(!prompt.contains("Email:"), "blank email must be omitted: {prompt}");
}

#[test]
fn clean_draft_is_allowed_unchanged() {
    let draft = "Hi Dana — great to have you on the SKYCOT waitlist. \
                 Happy to set up a short call whenever suits. — Grant, SKYCOT";
    let (out, disposition) = finalize_draft(draft);
    assert_eq!(disposition, "Allow");
    assert_eq!(out, draft, "an allowed draft passes through verbatim");
}

#[test]
fn draft_touching_valuation_is_escalated_and_swapped() {
    let draft = "Hi Dana — we are valued at $1B and growing fast. — Grant";
    let (out, disposition) = finalize_draft(draft);
    assert_eq!(disposition, "Escalate");
    assert!(
        out.contains("Held for founder review"),
        "guarded draft must be swapped for the safe note: {out}"
    );
    assert!(
        !out.contains("valued at"),
        "the unsafe draft text must NOT survive the swap: {out}"
    );
}

#[test]
fn draft_touching_revenue_is_escalated() {
    let draft = "Our ARR is huge and our revenue is $40M. — Grant";
    let (_out, disposition) = finalize_draft(draft);
    assert_eq!(disposition, "Escalate");
}

#[test]
fn system_prompt_carries_truth_boundary_and_no_forbidden_phrasing() {
    let sys = gtm_system_prompt();
    assert!(sys.contains("Truth boundary"), "must declare the truth boundary");
    assert!(
        sys.contains("Melbourne"),
        "approved facts include the Melbourne origin"
    );
    // The system prompt itself must be clean — a faithful draft echoing
    // it should never trip the forbidden-class extractor.
    assert!(
        extract_forbidden_class(&sys).is_none(),
        "system prompt must not contain forbidden phrasings: matched {:?}",
        extract_forbidden_class(&sys)
    );
}

#[test]
fn empty_body_is_a_bad_request() {
    let (status, _body) = bad_request("`signup.name` is required");
    assert_eq!(status, "400 Bad Request");
}

#[test]
fn qualifies_a_senior_intent_corporate_lead_as_hot() {
    let s = signup(
        "Avery Stone",
        "avery@brightloop.io",
        Some("VP Eng at a 120-person fintech, evaluating AI teammates to automate on-call triage; we want to cut manual toil this quarter."),
    );
    let q = qualify(&s);
    assert_eq!(q.tier, "Hot", "senior + intent + corporate + scale + detail should be Hot");
    assert!(q.signals.contains(&"senior decision-maker".to_string()));
    assert!(q.signals.contains(&"active buying intent".to_string()));
    assert!(q.signals.contains(&"corporate email domain".to_string()));
    assert!(!q.rationale.is_empty());
}

#[test]
fn qualifies_a_single_signal_lead_as_warm() {
    // Corporate email only (one weak signal) → Warm boundary is score>=2,
    // so a single +1 signal stays Cold; give it intent (+2) to land Warm
    // with no other signals.
    let s = signup("Sam Lee", "sam@gmail.com", Some("just looking for something to help out"));
    let q = qualify(&s);
    assert_eq!(q.tier, "Warm", "active intent alone (score 2) is Warm; got {:?}", q);
    assert!(q.signals.contains(&"active buying intent".to_string()));
}

#[test]
fn qualifies_a_no_signal_lead_as_cold_without_crashing() {
    let s = signup("Pat", "pat@gmail.com", None);
    let q = qualify(&s);
    assert_eq!(q.tier, "Cold");
    assert!(q.signals.is_empty());
    assert!(q.rationale.contains("Limited signal"));
}
