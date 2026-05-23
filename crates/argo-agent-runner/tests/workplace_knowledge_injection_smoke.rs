//! Integration smoke test for `mission-knowledge-context-injection`.
//!
//! Seeds a workplace-knowledge doc, composes the turn-start system
//! message via the public diagnostic helper, and asserts the literal
//! bytes the LLM would see contain the doc body. This is the
//! "not theater" wire test — design doc §7 Pillar 1 acceptance #2.

use argo_agent_runner::compose_system_prompt_for_diagnostics;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId};
use argo_workplace_knowledge::doc::{self, KnowledgeCategory};
use std::sync::Arc;

#[tokio::test]
async fn workplace_doc_body_appears_byte_for_byte_in_system_message() {
    let store = Arc::new(
        SessionStore::open(":memory:")
            .await
            .expect("open in-memory store"),
    );
    store.migrate().await.expect("apply migrations");
    let tenant = TenantId::new();

    let body = "## Voice\nNever say cheap.";
    doc::create(
        store.pool(),
        &tenant,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        body,
        "ops@example.com",
    )
    .await
    .expect("seed workplace doc");

    let prompt = compose_system_prompt_for_diagnostics(
        &store,
        SessionId(uuid::Uuid::nil()),
        None,
        tenant,
        None,
    )
    .await
    .expect("compose system message")
    .expect("expected a non-empty system message");

    assert!(
        prompt.contains("Never say cheap."),
        "expected literal doc body in system message; got:\n{prompt}"
    );
    assert!(
        prompt.contains("[WORKPLACE KNOWLEDGE — read-only, do not contradict]"),
        "expected workplace-knowledge banner; got:\n{prompt}"
    );
    assert!(
        prompt.contains("[END WORKPLACE KNOWLEDGE]"),
        "expected workplace-knowledge EOF marker; got:\n{prompt}"
    );
}

#[tokio::test]
async fn empty_tenant_omits_workplace_block_entirely() {
    let store = Arc::new(
        SessionStore::open(":memory:")
            .await
            .expect("open in-memory store"),
    );
    store.migrate().await.expect("apply migrations");
    let tenant = TenantId::new();

    let prompt = compose_system_prompt_for_diagnostics(
        &store,
        SessionId(uuid::Uuid::nil()),
        None,
        tenant,
        None,
    )
    .await
    .expect("compose system message");

    let body = prompt.unwrap_or_default();
    assert!(
        !body.contains("[WORKPLACE KNOWLEDGE"),
        "expected NO workplace block for empty tenant; got:\n{body}"
    );
    assert!(
        !body.contains("[END WORKPLACE KNOWLEDGE]"),
        "expected NO workplace EOF marker for empty tenant; got:\n{body}"
    );
}

#[tokio::test]
async fn multiple_docs_appear_in_alphabetical_order() {
    let store = Arc::new(
        SessionStore::open(":memory:")
            .await
            .expect("open in-memory store"),
    );
    store.migrate().await.expect("apply migrations");
    let tenant = TenantId::new();

    doc::create(
        store.pool(),
        &tenant,
        "refund-policy",
        "Refund policy",
        KnowledgeCategory::BusinessRules,
        "BODY-REFUND",
        "ops@example.com",
    )
    .await
    .expect("seed business-rules doc");
    doc::create(
        store.pool(),
        &tenant,
        "voice",
        "Voice",
        KnowledgeCategory::Brand,
        "BODY-VOICE",
        "ops@example.com",
    )
    .await
    .expect("seed brand doc");

    let prompt = compose_system_prompt_for_diagnostics(
        &store,
        SessionId(uuid::Uuid::nil()),
        None,
        tenant,
        None,
    )
    .await
    .expect("compose system message")
    .expect("expected a non-empty system message");

    let voice_at = prompt.find("BODY-VOICE").expect("voice body present");
    let refund_at = prompt.find("BODY-REFUND").expect("refund body present");
    assert!(
        voice_at < refund_at,
        "expected brand (Voice) before business-rules (Refund); got voice@{voice_at} refund@{refund_at}\n{prompt}"
    );
}
