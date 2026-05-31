//! Unit tests for Marcus's marketing draft-response endpoint. These
//! drive the pure pieces (mode/channel parsing, prompt build across
//! both modes × both channels, the truth-boundary swap, system-prompt
//! safety) without a live provider — the live-LLM path is exercised by
//! the gateway smoke verification, not here.

use super::*;

// ───────────────────────────── parsing ──────────────────────────────

#[test]
fn mode_parse_accepts_reply_and_article_case_insensitive() {
    assert_eq!(Mode::parse("reply"), Some(Mode::Reply));
    assert_eq!(Mode::parse(" Article "), Some(Mode::Article));
    assert_eq!(Mode::parse("REPLY"), Some(Mode::Reply));
    assert_eq!(Mode::parse("post"), None);
}

#[test]
fn channel_parse_accepts_x_twitter_and_linkedin() {
    assert_eq!(Channel::parse("x"), Some(Channel::X));
    assert_eq!(Channel::parse("twitter"), Some(Channel::X));
    assert_eq!(Channel::parse("LinkedIn"), Some(Channel::LinkedIn));
    assert_eq!(Channel::parse("facebook"), None);
}

#[test]
fn channel_max_tokens_differs_short_vs_long() {
    assert!(
        Channel::X.max_tokens() < Channel::LinkedIn.max_tokens(),
        "X (short) must cap lower than LinkedIn (long-form)"
    );
}

// ─────────────────────── prompt build: reply ────────────────────────

#[test]
fn reply_prompt_embeds_the_pasted_source_and_x_format() {
    let prompt = build_user_prompt(
        Mode::Reply,
        Channel::X,
        "Someone tweeted: AI agents are just fancy autocomplete.",
        "",
    );
    assert!(
        prompt.contains("AI agents are just fancy autocomplete"),
        "the pasted post must appear verbatim: {prompt}"
    );
    assert!(prompt.contains("reply"), "reply framing must appear: {prompt}");
    assert!(
        prompt.contains("X/Twitter"),
        "X format reminder must appear: {prompt}"
    );
    assert!(
        prompt.contains("no sign-off"),
        "X format forbids a sign-off: {prompt}"
    );
}

#[test]
fn reply_prompt_linkedin_asks_for_long_form_and_signoff() {
    let prompt = build_user_prompt(Mode::Reply, Channel::LinkedIn, "a post about drift", "");
    assert!(prompt.contains("a post about drift"));
    assert!(prompt.contains("LinkedIn"), "channel reminder: {prompt}");
    assert!(
        prompt.contains("— Mark + the team"),
        "LinkedIn long-form ends with the sign-off line: {prompt}"
    );
}

// ────────────────────── prompt build: article ───────────────────────

#[test]
fn article_prompt_embeds_topic_and_extra_framing() {
    let prompt = build_user_prompt(
        Mode::Article,
        Channel::LinkedIn,
        "lean into the coal-face angle",
        "Why ideas are the human moat",
    );
    assert!(
        prompt.contains("Why ideas are the human moat"),
        "topic must appear: {prompt}"
    );
    assert!(
        prompt.contains("lean into the coal-face angle"),
        "extra framing (source) must appear in article mode: {prompt}"
    );
    assert!(prompt.contains("thought-leadership"), "article framing: {prompt}");
}

#[test]
fn article_prompt_x_is_short_and_omits_empty_framing() {
    let prompt = build_user_prompt(Mode::Article, Channel::X, "", "drift prevention");
    assert!(prompt.contains("drift prevention"));
    assert!(prompt.contains("X/Twitter"), "channel reminder: {prompt}");
    assert!(
        !prompt.contains("Extra framing"),
        "no 'Extra framing' line when source is empty: {prompt}"
    );
}

// ─────────────────────── truth-boundary swap ────────────────────────

#[test]
fn clean_draft_is_allowed_unchanged() {
    let draft = "Ideas are still the human moat. AI executes; humans generate. \
                 That's the whole game right now. — Mark + the team";
    let (out, disposition) = finalize_draft(draft);
    assert_eq!(disposition, "Allow");
    assert_eq!(out, draft, "an allowed draft passes through verbatim");
}

#[test]
fn draft_touching_valuation_is_escalated_and_swapped() {
    let draft = "We are valued at $1B and growing fast — come work with us.";
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
    let draft = "Our ARR is huge — our revenue is $40M this year.";
    let (_out, disposition) = finalize_draft(draft);
    assert_eq!(disposition, "Escalate");
}

#[test]
fn draft_touching_customer_count_is_escalated() {
    let draft = "We have over 10,000 customers already using SKYCOT every day.";
    let (out, disposition) = finalize_draft(draft);
    assert_eq!(disposition, "Escalate");
    assert!(out.contains("Held for founder review"));
}

// ─────────────────── system prompt safety (both) ────────────────────

#[test]
fn system_prompt_carries_truth_boundary_and_no_forbidden_phrasing() {
    // Every combination of mode × channel must be truth-boundary-clean:
    // a faithful draft echoing the system prompt should never trip the
    // forbidden-class extractor.
    for mode in [Mode::Reply, Mode::Article] {
        for channel in [Channel::X, Channel::LinkedIn] {
            let sys = marketing_system_prompt(mode, channel);
            assert!(
                sys.contains("Truth boundary"),
                "{:?}/{:?} must declare the truth boundary",
                mode,
                channel
            );
            assert!(
                sys.contains("Melbourne"),
                "approved facts include the Melbourne origin"
            );
            assert!(
                sys.contains("Marcus"),
                "persona must name Marcus: {:?}/{:?}",
                mode,
                channel
            );
            assert!(
                extract_forbidden_class(&sys).is_none(),
                "system prompt must not contain forbidden phrasings ({:?}/{:?}): matched {:?}",
                mode,
                channel,
                extract_forbidden_class(&sys)
            );
        }
    }
}

#[test]
fn linkedin_system_prompt_requests_signoff_x_does_not() {
    let li = marketing_system_prompt(Mode::Article, Channel::LinkedIn);
    let x = marketing_system_prompt(Mode::Article, Channel::X);
    assert!(li.contains("— Mark + the team"), "LinkedIn asks for the sign-off");
    assert!(x.contains("no sign-off"), "X explicitly omits a sign-off");
}

// ──────────────────────────── envelopes ─────────────────────────────

#[test]
fn empty_body_is_a_bad_request() {
    let (status, _body) = bad_request("`request.mode` must be \"reply\" or \"article\"");
    assert_eq!(status, "400 Bad Request");
}

#[test]
fn service_unavailable_envelope_is_503() {
    let (status, body) = service_unavailable("marketing llm call: boom");
    assert_eq!(status, "503 Service Unavailable");
    assert!(body.contains("boom"));
}
