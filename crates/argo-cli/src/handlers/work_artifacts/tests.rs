//! Unit tests for `SectionedOutput` (Sectioned-output contract +
//! the v3 multi-step assembly + evidence-blocked approval
//! checklist preamble). Extracted from `mod.rs` (#727) so the
//! orchestration file stays under the 500-LOC budget.
//!
//! Loaded via `#[cfg(test)] #[path = "tests.rs"] mod tests;`
//! at the bottom of `mod.rs`.

use super::*;

/// #722 — splitter pulls the four labelled deliverables out
/// of a multi-step worker's final assistant message.
#[test]
fn sectioned_output_splits_all_four_named_sections() {
    let body = "\
        preamble that should not be captured\n\n\
        ## research\nOne find with [src](https://e.example).\n\n\
        ## pricing\n| comp | $99 |\n\n\
        ## listings\nDraft: ProSkates 110mm\n\n\
        ## approval_checklist\n- [ ] confirm reserve price\n\n\
        [STOP: APPROVAL_REQUIRED]\n";
    let s = SectionedOutput::from_assistant_text(body);
    assert!(s.research.as_deref().unwrap_or("").contains("[src]"));
    assert!(s.pricing.as_deref().unwrap_or("").contains("comp"));
    assert!(s.listings.as_deref().unwrap_or("").contains("ProSkates"));
    assert!(s
        .approval_checklist
        .as_deref()
        .unwrap_or("")
        .contains("reserve price"));
}

#[test]
fn sectioned_output_handles_missing_sections_with_placeholder() {
    let body = "## research\nfindings here\n";
    let s = SectionedOutput::from_assistant_text(body);
    assert!(s.research.is_some());
    assert!(s.pricing.is_none());
    let pricing = s.pricing_or_placeholder();
    assert!(pricing.contains("not produced this run"));
    let listings = s.listings_or_placeholder();
    assert!(listings.contains("not produced this run"));
}

#[test]
fn approval_checklist_always_carries_stop_reason_in_preamble() {
    let s = SectionedOutput::from_assistant_text("## approval_checklist\n- [ ] human check\n");
    let out = s.approval_checklist_or_placeholder("approval_required", "", false, "");
    assert!(out.contains("# Approval checklist"));
    assert!(out.contains("stop_reason: `approval_required`"));
    assert!(out.contains("human check"));
    // Not blocked → no BLOCKED banner.
    assert!(!out.contains("BLOCKED"));
    assert!(!out.contains("not_ready_to_post"));
}

#[test]
fn approval_checklist_includes_need_info_detail() {
    let s = SectionedOutput::default();
    let out = s.approval_checklist_or_placeholder("need_info", "seller's reserve price", false, "");
    assert!(out.contains("stop_detail: seller's reserve price"));
}

/// #727 Issue 2 — when blocked=true, the file must lead with a
/// BLOCKED banner, name the reason, and include
/// `not_ready_to_post: true`.
#[test]
fn approval_checklist_emits_blocked_preamble_when_evidence_insufficient() {
    let s =
        SectionedOutput::from_assistant_text("## approval_checklist\n- [ ] verify reserve price\n");
    let out = s.approval_checklist_or_placeholder(
        "approval_required",
        "",
        true,
        "5 web_search call(s) returned zero results (0 failed); 3 web_extract call(s) yielded only 0 chars total (3 failed)",
    );
    assert!(out.contains("BLOCKED"));
    assert!(out.contains("NOT READY TO POST"));
    assert!(out.contains("not_ready_to_post: `true`"));
    assert!(out.contains("zero results"));
    // Model's checklist still surfaces under the banner.
    assert!(out.contains("verify reserve price"));
}

/// Multi-step assembly: pricing in step 1, approval in step 2 —
/// both must end up in the bundle (#727 Issue 1).
#[test]
fn from_assistant_messages_merges_sections_across_steps() {
    let step1 = "## research\nfindings\n\n## pricing\n| comp | $250 |\n";
    let step2 = "## approval_checklist\n- [ ] confirm reserve\n[STOP: APPROVAL_REQUIRED]";
    let s = SectionedOutput::from_assistant_messages([step1, step2]);
    assert!(s.research.as_deref().unwrap_or("").contains("findings"));
    assert!(
        s.pricing.as_deref().unwrap_or("").contains("$250"),
        "step 1 pricing must survive step 2 (which didn't re-emit it)"
    );
    assert!(s
        .approval_checklist
        .as_deref()
        .unwrap_or("")
        .contains("confirm reserve"));
    // listings was never produced anywhere → still missing.
    assert!(s.listings.is_none());
    assert_eq!(s.missing_sections(), vec!["listings"]);
}

/// Latest-non-empty-wins rule: step 2's pricing overrides step
/// 1's pricing when both exist.
#[test]
fn from_assistant_messages_latest_section_wins_when_both_exist() {
    let step1 = "## pricing\nfirst draft $200\n";
    let step2 = "## pricing\nrefined $300 with comps\n";
    let s = SectionedOutput::from_assistant_messages([step1, step2]);
    let p = s.pricing.as_deref().unwrap_or("");
    assert!(p.contains("$300"));
    assert!(
        !p.contains("$200"),
        "step 2 must override step 1 for the same section"
    );
}

/// Missing-sections list is sorted by canonical order.
#[test]
fn missing_sections_lists_only_unproduced() {
    let s = SectionedOutput::from_assistant_text("## research\nstuff\n");
    let missing = s.missing_sections();
    assert_eq!(missing, vec!["pricing", "listings", "approval_checklist"]);

    let all = SectionedOutput::from_assistant_text(
        "## research\na\n## pricing\nb\n## listings\nc\n## approval_checklist\nd\n",
    );
    assert!(all.missing_sections().is_empty());
}

#[test]
fn sectioned_output_recognises_listings_singular_variant() {
    let body = "## listing\ndraft ad copy here\n";
    let s = SectionedOutput::from_assistant_text(body);
    assert!(
        s.listings.is_some(),
        "should match `## listing` (singular) too"
    );
}

#[test]
fn sectioned_output_treats_unrelated_text_as_empty_sections() {
    let body = "the worker said many words but never used a section header";
    let s = SectionedOutput::from_assistant_text(body);
    assert!(s.research.is_none());
    assert!(s.pricing.is_none());
    assert!(s.listings.is_none());
    assert!(s.approval_checklist.is_none());
}

// ─────────────────────── #727 integration ────────────────────────

/// End-to-end through `write_artifacts` with a synthetic session
/// that mirrors the user-reported failure shape:
/// - tool_result blocks all return zero search results / fetch errors
/// - assistant message claims approval-readiness
/// → bundle must land with `not_ready_to_post: true`,
///   `blocked_reason` populated, BLOCKED banner on
///   approval_checklist.md.
///
/// Also locks Issue 1: pricing emitted in step 1 must end up in
/// `pricing.md` even though the final assistant message only
/// produces the approval checklist.
#[tokio::test]
async fn write_artifacts_blocks_bundle_on_zero_evidence_research() {
    use argo_store::SessionStore;
    use argo_types::MessageRole;

    let store = std::sync::Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let (tenant, ws) = store
        .ensure_workspace("ws-727", &cwd.path().to_string_lossy())
        .await
        .unwrap();
    let sid = store
        .create_session(tenant, ws, "anthropic", "claude", Some("skate"), None)
        .await
        .unwrap();

    // user → "sell skates"
    store
        .append_message(
            sid,
            MessageRole::User,
            r#"[{"type":"text","text":"sell these skates"}]"#,
        )
        .await
        .unwrap();
    // step 1 assistant: produced research + pricing.
    let step1 = r###"[{"type":"text","text":"Step 1 results.\n\n## research\nFindings stub.\n\n## pricing\n| AU comp | $250 |\n"}]"###;
    let asst1 = store
        .append_message(sid, MessageRole::Assistant, step1)
        .await
        .unwrap();
    // tool_result: web_search returned ZERO hits.
    let zero = r###"[{"type":"tool_result","id":"t1","is_error":false,"content":[{"type":"text","text":"{\"backend\":\"duckduckgo_html\",\"query\":\"x\",\"result_count\":0,\"results\":[]}"}]}]"###;
    store
        .append_message(sid, MessageRole::Tool, zero)
        .await
        .unwrap();
    // tool_result: web_extract failed.
    let extract_err = r###"[{"type":"tool_result","id":"t2","is_error":true,"content":[{"type":"text","text":"web_extract failed: status 403 for https://x"}]}]"###;
    store
        .append_message(sid, MessageRole::Tool, extract_err)
        .await
        .unwrap();
    // step 2 assistant: only approval_checklist this time + STOP marker.
    let step2 = r###"[{"type":"text","text":"## approval_checklist\n- [ ] verify reserve price\n\n[STOP: APPROVAL_REQUIRED]\n"}]"###;
    store
        .append_message(sid, MessageRole::Assistant, step2)
        .await
        .unwrap();

    let inputs = ArtifactInputs {
        job_id: uuid::Uuid::now_v7(),
        session_id: sid,
        vertical_id: "gtm-engineer",
        provider: "anthropic",
        model: "claude",
        created_at: "2026-05-01T00:00:00Z",
        completed_at: "2026-05-01T00:01:00Z",
        assistant_id: asst1,
        assistant_text:
            "## approval_checklist\n- [ ] verify reserve price\n\n[STOP: APPROVAL_REQUIRED]\n",
        assistant_text_preview: "## approval_checklist...",
        steps_completed: 2,
        stop_reason: "approval_required",
        stop_detail: "",
    };
    let dir = write_artifacts(&store, cwd.path(), &inputs).await.unwrap();

    // Issue 1 acceptance: step-1 pricing survived even though
    // step-2 didn't re-emit it.
    let pricing = std::fs::read_to_string(dir.join("pricing.md")).unwrap();
    assert!(
        pricing.contains("$250"),
        "step-1 pricing must survive multi-step assembly; got:\n{pricing}"
    );

    // Issue 2 acceptance: approval_checklist.md leads with BLOCKED.
    let approval = std::fs::read_to_string(dir.join("approval_checklist.md")).unwrap();
    assert!(
        approval.contains("BLOCKED"),
        "BLOCKED banner missing; got:\n{approval}"
    );
    assert!(approval.contains("not_ready_to_post: `true`"));
    assert!(approval.contains("zero results") || approval.contains("403"));
    // Model's checklist content still appears under the banner.
    assert!(approval.contains("verify reserve price"));

    // result.json carries the v3 fields.
    let result_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("result.json")).unwrap()).unwrap();
    assert_eq!(result_json["not_ready_to_post"], true);
    assert!(result_json["blocked_reason"].as_str().unwrap_or("").len() > 10);
    assert_eq!(result_json["sections_present"]["pricing"], true);
    assert_eq!(result_json["sections_present"]["approval_checklist"], true);
    assert_eq!(result_json["sections_present"]["research"], true);
    assert_eq!(result_json["sections_present"]["listings"], false);
    let missing = result_json["missing_sections"].as_array().unwrap();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0], "listings");
    // Evidence summary surfaced for operator inspection.
    assert!(
        result_json["evidence"]["web_search_zero_result"]
            .as_u64()
            .unwrap()
            >= 1
    );
    assert!(
        result_json["evidence"]["web_extract_failed"]
            .as_u64()
            .unwrap()
            >= 1
    );
}

/// When research IS sufficient, the bundle must NOT be blocked
/// even if stop_reason is approval_required.
#[tokio::test]
async fn write_artifacts_does_not_block_bundle_when_evidence_is_sufficient() {
    use argo_store::SessionStore;
    use argo_types::MessageRole;

    let store = std::sync::Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let (tenant, ws) = store
        .ensure_workspace("ws-727b", &cwd.path().to_string_lossy())
        .await
        .unwrap();
    let sid = store
        .create_session(tenant, ws, "anthropic", "claude", Some("skate"), None)
        .await
        .unwrap();

    // tool_result: web_search returned 3 hits.
    let search = r###"[{"type":"tool_result","id":"t1","is_error":false,"content":[{"type":"text","text":"{\"backend\":\"duckduckgo_html\",\"query\":\"x\",\"result_count\":3,\"results\":[]}"}]}]"###;
    store
        .append_message(sid, MessageRole::Tool, search)
        .await
        .unwrap();
    // tool_result: web_extract returned 1500 chars.
    let extract = r###"[{"type":"tool_result","id":"t2","is_error":false,"content":[{"type":"text","text":"{\"url\":\"https://x\",\"title\":\"x\",\"text\":\"...\",\"char_count\":1500,\"truncated\":false}"}]}]"###;
    store
        .append_message(sid, MessageRole::Tool, extract)
        .await
        .unwrap();
    // assistant: produces approval_checklist.
    let asst_text = r###"[{"type":"text","text":"## approval_checklist\n- [ ] go!\n\n[STOP: APPROVAL_REQUIRED]\n"}]"###;
    let aid = store
        .append_message(sid, MessageRole::Assistant, asst_text)
        .await
        .unwrap();

    let inputs = ArtifactInputs {
        job_id: uuid::Uuid::now_v7(),
        session_id: sid,
        vertical_id: "gtm-engineer",
        provider: "anthropic",
        model: "claude",
        created_at: "2026-05-01T00:00:00Z",
        completed_at: "2026-05-01T00:01:00Z",
        assistant_id: aid,
        assistant_text: "## approval_checklist\n- [ ] go!\n\n[STOP: APPROVAL_REQUIRED]\n",
        assistant_text_preview: "## approval_checklist",
        steps_completed: 1,
        stop_reason: "approval_required",
        stop_detail: "",
    };
    let dir = write_artifacts(&store, cwd.path(), &inputs).await.unwrap();

    let approval = std::fs::read_to_string(dir.join("approval_checklist.md")).unwrap();
    assert!(!approval.contains("BLOCKED"));
    assert!(!approval.contains("not_ready_to_post"));
    let result_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("result.json")).unwrap()).unwrap();
    assert_eq!(result_json["not_ready_to_post"], false);
    assert_eq!(result_json["blocked_reason"], "");
}
