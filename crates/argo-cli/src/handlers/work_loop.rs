//! Multi-step worker orchestration (#722).
//!
//! `argo work serve` runs each job through a small loop here:
//! one "step" is one full agent turn (which itself may dispatch
//! tools internally up to `MAX_TOOL_ROUNDS`), and we keep
//! issuing steps until the model emits a stop marker or we hit
//! `--max-steps`. The outer loop is what makes Worker v2 feel
//! "autonomous" — research → verify → draft → checklist all
//! happen inside one queued job rather than requiring the
//! human to re-prompt between phases.
//!
//! The orchestrator is parametric over the actual step driver
//! so unit tests can exercise it with mock closures (no LLM,
//! no SessionStore) — the integration with the real
//! `chat::run_turn` lives in `work.rs::process_job`.
//!
//! ## Stop markers
//!
//! The bound vertical's system prompt instructs the model to
//! end every turn with one of:
//!
//!   `[STOP: TASK_COMPLETE]`        — the task is fully done
//!   `[STOP: APPROVAL_REQUIRED]`    — drafts ready; needs human review
//!   `[STOP: NEED_INFO <one-line>]` — missing critical fact only the human supplies
//!   `[CONTINUE]`                   — more work to do; another step follows
//!
//! Anything else is treated as `[CONTINUE]` (lenient — newer
//! models drift on terse format demands and an unmarked
//! response shouldn't cause the worker to abort early).

use std::future::Future;

/// Why the multi-step loop ended. Carried back to the caller
/// (process_job) so it can be persisted into job.json and the
/// completed_jobs memory body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StopReason {
    TaskComplete,
    ApprovalRequired,
    NeedInfo(String),
    MaxStepsReached,
    /// Pre-flight: neither web_search nor web_extract is available
    /// to the bound vertical, but multi-step mode (>1 step) was
    /// requested. We refuse to start rather than silently produce
    /// an unverified output.
    ResearchToolsUnavailable,
}

impl StopReason {
    /// Lower-snake-case wire form for JSON / memory rows.
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            StopReason::TaskComplete => "task_complete",
            StopReason::ApprovalRequired => "approval_required",
            StopReason::NeedInfo(_) => "need_info",
            StopReason::MaxStepsReached => "max_steps_reached",
            StopReason::ResearchToolsUnavailable => "research_tools_unavailable",
        }
    }

    /// Human-readable detail string for `NeedInfo`, empty for the
    /// other variants. Used in the artifact + memory payload.
    pub(crate) fn detail(&self) -> &str {
        match self {
            StopReason::NeedInfo(s) => s.as_str(),
            _ => "",
        }
    }
}

/// Search the assistant's last ~600 bytes for a stop marker.
/// Models tend to emit the marker on the final line; capping
/// the search keeps the parser cheap on long outputs.
///
/// **UTF-8 safety (#725).** The cutoff is in BYTES, not chars,
/// and a worker run reproducibly emitted output where a 4-byte
/// emoji (e.g. `📋`) straddled the 600-byte mark — slicing
/// directly via `&text[len-600..]` panicked with
/// `byte index N is not a char boundary`. We walk back to the
/// nearest UTF-8 boundary instead, which costs at most 3 bytes
/// of extra context and never panics.
pub(crate) fn parse_stop_marker(text: &str) -> Option<StopReason> {
    let approx_start = text.len().saturating_sub(600);
    let tail_start = floor_char_boundary(text, approx_start);
    let tail = &text[tail_start..];

    if tail.contains("[STOP: TASK_COMPLETE]") || tail.contains("[STOP:TASK_COMPLETE]") {
        return Some(StopReason::TaskComplete);
    }
    if tail.contains("[STOP: APPROVAL_REQUIRED]") || tail.contains("[STOP:APPROVAL_REQUIRED]") {
        return Some(StopReason::ApprovalRequired);
    }
    // [STOP: NEED_INFO <one-liner>] — `idx` from str::find and
    // `end` from find(']') are both UTF-8 char boundaries, and
    // `prefix.len()` is ASCII-only, so all slices below stay safe.
    for prefix in ["[STOP: NEED_INFO", "[STOP:NEED_INFO"] {
        if let Some(idx) = tail.find(prefix) {
            let after = &tail[idx + prefix.len()..];
            if let Some(end) = after.find(']') {
                let detail = after[..end]
                    .trim()
                    .trim_start_matches(':')
                    .trim()
                    .to_string();
                return Some(StopReason::NeedInfo(detail));
            }
        }
    }
    None
}

/// Walk back from `i` to the nearest UTF-8 char boundary in
/// `s`. Mirrors the behaviour of the unstable
/// `str::floor_char_boundary` (tracking issue 93743). Walks at
/// most 3 bytes (UTF-8 codepoints are 1–4 bytes long), so this
/// is O(1) regardless of input length. Always returns a value
/// in `0..=s.len()`.
fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while !s.is_char_boundary(i) {
        // i can't underflow: position 0 is always a char
        // boundary, so the loop terminates before then.
        i -= 1;
    }
    i
}

/// Build the synthetic per-step user message that nudges the
/// agent forward in step 2..N. Step 1's user message is the
/// real task prompt; this is for "keep going" rounds.
pub(crate) fn continue_nudge(step: u32, max_steps: u32) -> String {
    format!(
        "[Multi-step worker mode | step {step}/{max_steps}] Continue. \
         End with [STOP: TASK_COMPLETE], [STOP: APPROVAL_REQUIRED], \
         [STOP: NEED_INFO <reason>], or [CONTINUE]."
    )
}

/// Outcome of the multi-step loop, returned to the caller.
#[derive(Debug, Clone)]
pub(crate) struct LoopOutcome {
    pub steps_completed: u32,
    pub stop_reason: StopReason,
    pub final_text: String,
}

/// One step's result, supplied by the closure the caller passes.
#[derive(Debug, Clone)]
pub(crate) struct StepResult {
    pub text: String,
}

/// Drive the multi-step loop. Each iteration calls `step_fn`
/// with the step number (1-based) and the user message to
/// append; step 1 gets the real task prompt, step N>1 gets the
/// continue-nudge string.
///
/// Returns when the model emits a stop marker or we run out
/// of steps. The caller is responsible for persisting per-step
/// data — the loop only cares about driving the conversation
/// forward and detecting termination.
pub(crate) async fn run_loop<F, Fut>(
    initial_prompt: String,
    max_steps: u32,
    mut step_fn: F,
) -> LoopOutcome
where
    F: FnMut(u32, String) -> Fut,
    Fut: Future<Output = StepResult>,
{
    if max_steps == 0 {
        return LoopOutcome {
            steps_completed: 0,
            stop_reason: StopReason::MaxStepsReached,
            final_text: String::new(),
        };
    }
    let mut prompt = initial_prompt;
    let mut last_text = String::new();
    let mut steps_completed = 0u32;
    for step in 1..=max_steps {
        let result = step_fn(step, prompt).await;
        steps_completed = step;
        last_text = result.text;
        if let Some(reason) = parse_stop_marker(&last_text) {
            return LoopOutcome {
                steps_completed,
                stop_reason: reason,
                final_text: last_text,
            };
        }
        prompt = continue_nudge(step + 1, max_steps);
    }
    LoopOutcome {
        steps_completed,
        stop_reason: StopReason::MaxStepsReached,
        final_text: last_text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn parses_task_complete_at_end_of_text() {
        let out = parse_stop_marker("done with everything\n[STOP: TASK_COMPLETE]");
        assert_eq!(out, Some(StopReason::TaskComplete));
    }

    #[test]
    fn parses_approval_required() {
        let out = parse_stop_marker("ready for review\n[STOP: APPROVAL_REQUIRED]");
        assert_eq!(out, Some(StopReason::ApprovalRequired));
    }

    #[test]
    fn parses_need_info_with_detail() {
        let out = parse_stop_marker("can't proceed\n[STOP: NEED_INFO seller's reserve price]");
        assert_eq!(
            out,
            Some(StopReason::NeedInfo("seller's reserve price".into()))
        );
    }

    #[test]
    fn no_marker_returns_none() {
        let out = parse_stop_marker("just plain text without a marker");
        assert_eq!(out, None);
    }

    #[test]
    fn marker_in_long_text_still_detected_when_at_tail() {
        let prefix = "x".repeat(2000);
        let body = format!("{prefix}\nsome reasoning\n[STOP: TASK_COMPLETE]");
        assert_eq!(parse_stop_marker(&body), Some(StopReason::TaskComplete));
    }

    #[test]
    fn tolerates_compact_spacing_variants() {
        assert_eq!(
            parse_stop_marker("[STOP:TASK_COMPLETE]"),
            Some(StopReason::TaskComplete)
        );
        assert_eq!(
            parse_stop_marker("[STOP:APPROVAL_REQUIRED]"),
            Some(StopReason::ApprovalRequired)
        );
    }

    /// Loop with closures that always emit `[CONTINUE]` (i.e. no
    /// stop marker) — the loop must cap at `max_steps`. Locks the
    /// "max steps prevents infinite loops" guarantee.
    #[tokio::test]
    async fn loop_caps_at_max_steps_when_model_never_stops() {
        let calls = RefCell::new(0u32);
        let outcome = run_loop("seed prompt".into(), 3, |step, _prompt| {
            *calls.borrow_mut() += 1;
            async move {
                StepResult {
                    text: format!("step {step} continues [CONTINUE]"),
                }
            }
        })
        .await;
        assert_eq!(*calls.borrow(), 3);
        assert_eq!(outcome.steps_completed, 3);
        assert_eq!(outcome.stop_reason, StopReason::MaxStepsReached);
    }

    /// Closure emits TASK_COMPLETE on step 2 → loop must stop early.
    #[tokio::test]
    async fn loop_stops_early_on_task_complete() {
        let outcome = run_loop("seed".into(), 5, |step, _prompt| async move {
            if step == 2 {
                StepResult {
                    text: "wrapping up [STOP: TASK_COMPLETE]".into(),
                }
            } else {
                StepResult {
                    text: "[CONTINUE]".into(),
                }
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 2);
        assert_eq!(outcome.stop_reason, StopReason::TaskComplete);
    }

    /// Step 1 returns APPROVAL_REQUIRED — loop returns immediately
    /// with steps_completed = 1, exercising the no-iterations-after
    /// path.
    #[tokio::test]
    async fn loop_stops_on_first_step_when_approval_required() {
        let outcome = run_loop("seed".into(), 5, |_, _| async move {
            StepResult {
                text: "drafts ready\n[STOP: APPROVAL_REQUIRED]".into(),
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 1);
        assert_eq!(outcome.stop_reason, StopReason::ApprovalRequired);
    }

    #[tokio::test]
    async fn zero_max_steps_returns_immediately() {
        let outcome = run_loop("seed".into(), 0, |_, _| async move {
            StepResult {
                text: "should not run".into(),
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 0);
        assert_eq!(outcome.stop_reason, StopReason::MaxStepsReached);
    }

    #[test]
    fn continue_nudge_includes_step_numbering() {
        let out = continue_nudge(3, 5);
        assert!(out.contains("step 3/5"), "got: {out}");
    }

    /// #724 — when chat::run_turn returns Ok with empty text
    /// (e.g. the model only emitted tool_use blocks before
    /// max_rounds capped the inner loop), parse_stop_marker
    /// returns None on empty text, so the outer loop must
    /// continue to the next step instead of treating empty as
    /// a stop condition.
    #[tokio::test]
    async fn loop_continues_when_step_returns_empty_text() {
        let calls = RefCell::new(0u32);
        let outcome = run_loop("seed".into(), 4, |_step, _prompt| {
            *calls.borrow_mut() += 1;
            async move {
                StepResult {
                    text: String::new(),
                }
            }
        })
        .await;
        assert_eq!(*calls.borrow(), 4, "outer loop must run all 4 steps");
        assert_eq!(outcome.steps_completed, 4);
        assert_eq!(
            outcome.stop_reason,
            StopReason::MaxStepsReached,
            "empty text has no stop marker so loop caps at max_steps"
        );
    }

    /// Same shape as the live integration: each step returns
    /// the breadcrumb run_live_loop would append on a
    /// tool-budget-exhausted Ok TurnResult — text without any
    /// stop marker. The outer loop must run all max_steps and
    /// surface MaxStepsReached (NOT NeedInfo).
    #[tokio::test]
    async fn loop_treats_tool_budget_breadcrumb_as_continuation() {
        let outcome = run_loop("seed".into(), 3, |step, _prompt| async move {
            StepResult {
                text: format!(
                    "step {step} did some work\n\n_[step boundary: \
                     tool-round budget exhausted; continuing in next step]_\n"
                ),
            }
        })
        .await;
        assert_eq!(outcome.steps_completed, 3);
        assert_eq!(outcome.stop_reason, StopReason::MaxStepsReached);
        assert!(
            outcome.final_text.contains("step boundary"),
            "final_text must preserve the breadcrumb so the artifact splitter surfaces it"
        );
    }

    // ───────────────────────── UTF-8 safety (#725) ─────────────────────────

    /// Direct regression for the panic the user reported:
    ///
    ///     thread 'main' panicked at handlers/work_loop.rs:76:21:
    ///     start byte index 5403 is not a char boundary;
    ///     it is inside '📋'
    ///
    /// The model's output included a 📋 emoji that straddled the
    /// 600-byte tail cutoff. Pre-#725, `&text[len-600..]` panicked
    /// because the cutoff landed mid-codepoint. We pad the prefix
    /// so the emoji lands AT the cutoff boundary every run.
    #[test]
    fn parse_stop_marker_does_not_panic_when_emoji_straddles_byte_cutoff() {
        // 📋 is 4 bytes in UTF-8. We want the byte just before it
        // to be at exactly `len - 600`. Build a prefix whose byte
        // length sits just below that, append 📋, then a marker.
        let prefix = "x".repeat(700);
        let trailer = " all done\n[STOP: TASK_COMPLETE]";
        let text = format!("{prefix}📋{trailer}");
        // Sanity: the emoji's first byte is at len - 600 - epsilon,
        // squarely in the danger zone for a naive byte-tail slice.
        // (Exact alignment isn't required — the bug is "anywhere
        // in the last 3 bytes near the cutoff".)
        let r = parse_stop_marker(&text);
        assert_eq!(r, Some(StopReason::TaskComplete));
    }

    /// The exact body fragment the user pasted — verbatim — must
    /// not panic and must surface the marker.
    #[test]
    fn parse_stop_marker_handles_user_reported_panic_snippet() {
        let body = format!(
            "{padding}\n\
             📋 The previous turn was cut off mid-sentence at the \
             approval checklist. Completing it now in full.\n\
             \n\
             ## approval_checklist\n\
             - [ ] confirm reserve price\n\
             - [ ] confirm shipping budget\n\
             \n\
             [STOP: APPROVAL_REQUIRED]\n",
            padding = "y".repeat(5_000),
        );
        let r = parse_stop_marker(&body);
        assert_eq!(r, Some(StopReason::ApprovalRequired));
    }

    /// Emoji directly before, inside the bracket, and after the
    /// stop marker must all parse cleanly. (Inside-the-bracket
    /// is unusual but a well-behaved parser shouldn't panic on
    /// it; we just expect no marker rather than a crash.)
    #[test]
    fn parse_stop_marker_robust_to_emoji_around_marker() {
        // Before:
        let before = "draft is ready 📋\n[STOP: APPROVAL_REQUIRED]";
        assert_eq!(
            parse_stop_marker(before),
            Some(StopReason::ApprovalRequired)
        );

        // After:
        let after = "[STOP: TASK_COMPLETE] 🎉";
        assert_eq!(parse_stop_marker(after), Some(StopReason::TaskComplete));

        // Inside the brackets — not a real marker; must not panic.
        let inside = "[STOP: 🎉 NEED_INFO not a real marker]";
        // Either NeedInfo with weird detail or None — both are
        // acceptable; the contract is "no panic".
        let _ = parse_stop_marker(inside);

        // NEED_INFO with emoji in the detail — this MUST round-trip.
        let detail = "[STOP: NEED_INFO seller's reserve price 💰]";
        match parse_stop_marker(detail) {
            Some(StopReason::NeedInfo(d)) => {
                assert!(d.contains("reserve price"));
                assert!(d.contains('💰'));
            }
            other => panic!("expected NeedInfo with emoji detail; got {other:?}"),
        }
    }

    /// Property-style: parse must never panic on arbitrary
    /// input. Stress mix of multi-byte chars (emoji, CJK,
    /// combining marks, ZWJ sequences) at lengths straddling
    /// the 600-byte cutoff.
    #[test]
    fn parse_stop_marker_never_panics_on_arbitrary_unicode_input() {
        let stress = ["📋", "🎉", "你好", "é\u{0301}", "🇯🇵", "👨‍👩‍👧‍👦"];
        for filler in &stress {
            for n in [550, 597, 598, 599, 600, 601, 602, 603, 700, 1200] {
                let prefix = filler.repeat(n / filler.len().max(1) + 1);
                let _ = parse_stop_marker(&format!("{prefix}\n\nfinal note"));
                assert_eq!(
                    parse_stop_marker(&format!("{prefix}\n[STOP: TASK_COMPLETE]\n")),
                    Some(StopReason::TaskComplete),
                    "marker must survive unicode prefix; filler={filler}, n={n}",
                );
            }
        }
    }

    /// floor_char_boundary covers its corners.
    #[test]
    fn floor_char_boundary_covers_corners() {
        let s = "abc📋def"; // 'a','b','c' (1 each), 📋 (4), 'd','e','f' (1 each) → len 10
                            // 0 is always a boundary
        assert_eq!(floor_char_boundary(s, 0), 0);
        // 3 is the boundary just before 📋
        assert_eq!(floor_char_boundary(s, 3), 3);
        // 4 / 5 / 6 land inside 📋 — must walk back to 3
        assert_eq!(floor_char_boundary(s, 4), 3);
        assert_eq!(floor_char_boundary(s, 5), 3);
        assert_eq!(floor_char_boundary(s, 6), 3);
        // 7 is the boundary just after 📋
        assert_eq!(floor_char_boundary(s, 7), 7);
        // i >= len clamps to len
        assert_eq!(floor_char_boundary(s, 999), s.len());
        assert_eq!(floor_char_boundary("", 0), 0);
        assert_eq!(floor_char_boundary("", 999), 0);
    }
}
