//! Research-evidence detector for marketplace-selling jobs (#727 Issue 2).
//!
//! Worker v3 with real `web_search` / `web_extract` was reaching
//! end-of-loop with the model claiming "no live retail prices
//! confirmed" yet still producing approval-ready listings, high
//! confidence ratings, and invented product features. This
//! module gives the artifact writer a programmatic signal — it
//! walks the session's `tool_result` messages and counts how
//! much real research actually landed. When the evidence is
//! thin, `write_artifacts` blocks the bundle with a visible
//! `not_ready_to_post: true` flag and a BLOCKED preamble on
//! `approval_checklist.md`. We don't override the model's
//! `stop_reason` — operators still see what the model claimed —
//! but the artifact bundle makes the gap loud.
//!
//! ## Heuristics (deliberately conservative)
//!
//! For the gate to PASS (i.e. evidence is sufficient):
//!   * at least one `web_search` call returned ≥ 1 hit, AND
//!   * `web_extract` cumulatively returned ≥ 200 chars of
//!     readable body across all calls.
//!
//! That bar is low — one search hit + one fetched article is
//! enough to count as "we did SOMETHING". Below that, we
//! conclude the worker was guessing.
//!
//! Tool-result bodies are recognised by their JSON shape (the
//! actual web tools serialise `{"backend":..,"result_count":..}`
//! / `{"url":..,"char_count":..}` payloads). We pattern-match
//! on those keys rather than threading tool name through the
//! session, which keeps this module standalone.

use serde_json::Value;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub(super) struct EvidenceSummary {
    pub web_search_calls: u32,
    /// Calls that returned with `result_count: 0`.
    pub web_search_zero_result: u32,
    /// Calls that errored (transport/HTTP/backend failure).
    pub web_search_failed: u32,
    /// Sum of `result_count` across all successful searches.
    pub total_search_hits: u32,
    pub web_extract_calls: u32,
    pub web_extract_failed: u32,
    /// Sum of `char_count` across all successful extracts.
    pub web_extract_total_chars: u32,
}

impl EvidenceSummary {
    /// Sufficient research = at least one search hit AND at
    /// least 200 chars of fetched content. The thresholds are
    /// intentionally low — they catch the "everything failed
    /// and the model guessed" case without rejecting workers
    /// who only needed one source.
    pub(super) fn is_sufficient(&self) -> bool {
        self.total_search_hits >= 1 && self.web_extract_total_chars >= 200
    }

    /// Reason string surfaced into the BLOCKED preamble of
    /// `approval_checklist.md` and `result.json`. Empty when
    /// research was sufficient.
    pub(super) fn insufficiency_reason(&self) -> String {
        if self.is_sufficient() {
            return String::new();
        }
        let mut bits = Vec::new();
        if self.web_search_calls == 0 {
            bits.push("no web_search calls were made".to_string());
        } else if self.total_search_hits == 0 {
            bits.push(format!(
                "{} web_search call(s) returned zero results ({} failed)",
                self.web_search_zero_result, self.web_search_failed,
            ));
        }
        if self.web_extract_total_chars < 200 {
            if self.web_extract_calls == 0 {
                bits.push("no web_extract calls were made".to_string());
            } else {
                bits.push(format!(
                    "{} web_extract call(s) yielded only {} chars total ({} failed)",
                    self.web_extract_calls, self.web_extract_total_chars, self.web_extract_failed,
                ));
            }
        }
        bits.join("; ")
    }
}

/// Walk every persisted `tool_result` block and tally what
/// landed. Operates on `(content_json, role)` pairs so the
/// caller doesn't need to depend on a specific MessageRow type.
pub(super) fn summarize_evidence<'a, I>(rows: I) -> EvidenceSummary
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    let mut s = EvidenceSummary::default();
    for (role, content_json) in rows {
        // Tool results land in a "tool" role row in the v1+
        // session shape; we also walk assistant rows because the
        // OpenAI/Anthropic transports embed tool_result blocks
        // in either role depending on adapter.
        if role != "tool" && role != "assistant" {
            continue;
        }
        let arr: Vec<Value> = match serde_json::from_str(content_json) {
            Ok(Value::Array(a)) => a,
            _ => continue,
        };
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_result") {
                continue;
            }
            let is_error = block
                .get("is_error")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let text = block
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|a| {
                    a.iter()
                        .find_map(|c| c.get("text").and_then(|t| t.as_str()))
                })
                .unwrap_or("");
            classify(&mut s, text, is_error);
        }
    }
    s
}

fn classify(s: &mut EvidenceSummary, text: &str, is_error: bool) {
    // Error-text shapes (from the Argo-side error_text helper).
    if is_error || text.starts_with("web_search") || text.starts_with("web_extract") {
        if text.contains("web_search") {
            s.web_search_calls += 1;
            s.web_search_failed += 1;
            return;
        }
        if text.contains("web_extract") {
            s.web_extract_calls += 1;
            s.web_extract_failed += 1;
            return;
        }
    }
    // Successful web_search payloads carry both `"backend"` and
    // `"result_count"`. Pattern-match the JSON shape rather than
    // doing a full parse to stay cheap.
    if text.contains(r#""backend""#) && text.contains(r#""result_count""#) {
        s.web_search_calls += 1;
        let n = pluck_u32(text, r#""result_count":"#);
        if n == 0 {
            s.web_search_zero_result += 1;
        } else {
            s.total_search_hits = s.total_search_hits.saturating_add(n);
        }
        return;
    }
    // Successful web_extract payloads carry `"url"` AND `"char_count"`.
    if text.contains(r#""char_count""#) && text.contains(r#""url""#) {
        s.web_extract_calls += 1;
        let n = pluck_u32(text, r#""char_count":"#);
        if n == 0 {
            s.web_extract_failed += 1;
        } else {
            s.web_extract_total_chars = s.web_extract_total_chars.saturating_add(n);
        }
    }
}

/// Cheap "find the first integer after `key`" helper. Avoids a
/// full JSON parse for what's a one-key probe.
fn pluck_u32(text: &str, key: &str) -> u32 {
    let Some(idx) = text.find(key) else {
        return 0;
    };
    let after = &text[idx + key.len()..];
    let digits: String = after
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool_msg(text: &str, is_error: bool) -> String {
        let payload = serde_json::json!([{
            "type": "tool_result",
            "id": "t-1",
            "is_error": is_error,
            "content": [{"type": "text", "text": text}],
        }]);
        payload.to_string()
    }

    #[test]
    fn zero_results_search_marks_evidence_insufficient() {
        let zero_search = tool_msg(
            r#"{"backend":"duckduckgo_html","query":"x","result_count":0,"results":[]}"#,
            false,
        );
        let rows: Vec<(&str, &str)> = vec![
            ("user", r#"[{"type":"text","text":"go research"}]"#),
            ("tool", zero_search.as_str()),
        ];
        let s = summarize_evidence(rows);
        assert_eq!(s.web_search_calls, 1);
        assert_eq!(s.web_search_zero_result, 1);
        assert_eq!(s.total_search_hits, 0);
        assert!(!s.is_sufficient());
        assert!(s.insufficiency_reason().contains("zero results"));
    }

    #[test]
    fn successful_search_plus_extract_passes() {
        let search = tool_msg(
            r#"{"backend":"duckduckgo_html","query":"skates","result_count":3,"results":[]}"#,
            false,
        );
        let extract = tool_msg(
            r#"{"url":"https://example.com","title":"X","text":"...","char_count":1234,"truncated":false}"#,
            false,
        );
        let rows: Vec<(&str, &str)> = vec![("tool", search.as_str()), ("tool", extract.as_str())];
        let s = summarize_evidence(rows);
        assert_eq!(s.web_search_calls, 1);
        assert_eq!(s.total_search_hits, 3);
        assert_eq!(s.web_extract_calls, 1);
        assert_eq!(s.web_extract_total_chars, 1234);
        assert!(s.is_sufficient());
        assert!(s.insufficiency_reason().is_empty());
    }

    #[test]
    fn extract_failures_count_as_failed() {
        let extract_err = tool_msg(
            "web_extract failed: status 403 for https://example.com/",
            true,
        );
        let rows: Vec<(&str, &str)> = vec![("tool", extract_err.as_str())];
        let s = summarize_evidence(rows);
        assert_eq!(s.web_extract_calls, 1);
        assert_eq!(s.web_extract_failed, 1);
        assert_eq!(s.web_extract_total_chars, 0);
        assert!(!s.is_sufficient());
    }

    #[test]
    fn search_failures_count_as_failed() {
        let search_err = tool_msg(
            "web_search (duckduckgo_html) failed: DDG HTML status 429",
            true,
        );
        let rows: Vec<(&str, &str)> = vec![("tool", search_err.as_str())];
        let s = summarize_evidence(rows);
        assert_eq!(s.web_search_calls, 1);
        assert_eq!(s.web_search_failed, 1);
        assert!(!s.is_sufficient());
    }

    #[test]
    fn sufficient_requires_both_search_hits_and_extract_chars() {
        // Only search hits, no extract → insufficient.
        let search = tool_msg(
            r#"{"backend":"duckduckgo_html","query":"x","result_count":5,"results":[]}"#,
            false,
        );
        let s = summarize_evidence(vec![("tool", search.as_str())]);
        assert!(!s.is_sufficient(), "search alone must not satisfy the gate");

        // Only extract, no search → insufficient.
        let extract = tool_msg(
            r#"{"url":"https://x","title":"x","text":"x","char_count":500,"truncated":false}"#,
            false,
        );
        let s = summarize_evidence(vec![("tool", extract.as_str())]);
        assert!(
            !s.is_sufficient(),
            "extract alone must not satisfy the gate"
        );
    }

    /// User-reported transcript shape: searches return zero,
    /// extracts hit 403 / 404 / network errors, model still
    /// emits approval-ready listings. Evidence detector must
    /// classify this as INSUFFICIENT and produce a reason
    /// string that names what failed.
    #[test]
    fn observed_skate_run_zero_results_plus_extract_errors_is_blocked() {
        let mut rows: Vec<String> = Vec::new();
        for _ in 0..5 {
            rows.push(tool_msg(
                r#"{"backend":"duckduckgo_html","query":"q","result_count":0,"results":[]}"#,
                false,
            ));
        }
        for _ in 0..3 {
            rows.push(tool_msg(
                "web_extract failed: status 403 for https://example.com/",
                true,
            ));
        }
        let refs: Vec<(&str, &str)> = rows.iter().map(|s| ("tool", s.as_str())).collect();
        let s = summarize_evidence(refs);
        assert!(!s.is_sufficient());
        let reason = s.insufficiency_reason();
        assert!(reason.contains("zero results"), "got: {reason}");
        assert!(
            reason.contains("403") || reason.contains("failed"),
            "got: {reason}"
        );
    }

    #[test]
    fn pluck_u32_extracts_first_integer_after_key() {
        assert_eq!(
            pluck_u32(r#"{"result_count":42,"x":1}"#, r#""result_count":"#),
            42
        );
        assert_eq!(pluck_u32(r#"{"result_count": 0}"#, r#""result_count":"#), 0);
        assert_eq!(pluck_u32(r#"{}"#, r#""result_count":"#), 0);
    }
}
