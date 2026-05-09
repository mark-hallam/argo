//! Stop-marker parsing for the multi-step worker loop (#722).
//!
//! Split out of `work_loop.rs` to keep that file under the
//! 500-LOC HARD RULE 6 ceiling. Markers are documented on
//! [`super::work_loop::StopReason`]; anything else is lenient
//! `[CONTINUE]` so newer models that drift on terse format
//! demands don't cause the worker to abort early.

use super::work_loop::StopReason;

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

#[cfg(test)]
mod tests {
    use super::*;

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
