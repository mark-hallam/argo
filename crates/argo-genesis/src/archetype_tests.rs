//! `classify_llm` test matrix — 5 cases per the brief
//! (`mission-genesis-llm-classifier`). Loaded by `archetype.rs`
//! via `#[cfg(test)] #[path = "archetype_tests.rs"] mod llm_tests;`
//! to keep `archetype.rs` under the per-file 500-LOC budget.

use super::{classify_llm, Archetype, ProviderClient};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::sync::Mutex;

/// Tiny scriptable provider for unit tests. The next response (or
/// error) is popped on each `classify` call; running out of script
/// entries is a test bug, not a real failure mode.
struct MockProviderClient {
    script: Mutex<Vec<MockResponse>>,
}

enum MockResponse {
    Ok(String),
    Err(ArgoError),
}

impl MockProviderClient {
    fn ok(text: &str) -> Self {
        Self {
            script: Mutex::new(vec![MockResponse::Ok(text.to_string())]),
        }
    }

    fn err(error: ArgoError) -> Self {
        Self {
            script: Mutex::new(vec![MockResponse::Err(error)]),
        }
    }
}

#[async_trait]
impl ProviderClient for MockProviderClient {
    async fn classify(&self, _prompt: &str) -> ArgoResult<String> {
        let mut guard = self.script.lock().expect("mock script mutex poisoned");
        match guard.pop() {
            Some(MockResponse::Ok(s)) => Ok(s),
            Some(MockResponse::Err(e)) => Err(e),
            None => panic!("MockProviderClient: script exhausted"),
        }
    }
}

/// Charter that the keyword fallback maps to MarketingIntern. Used
/// in fall-through tests so we can assert "the LLM was bypassed and
/// keyword took over" without coupling to a specific failure cause.
const MARKETING_CHARTER: &str = "be my marketing intern, run weekly campaigns to cold leads";

#[tokio::test]
async fn llm_returns_valid_slug() {
    let llm = MockProviderClient::ok("ops_on_call");
    // The charter intentionally classifies to a DIFFERENT archetype
    // under the keyword path so passing means we honoured the LLM,
    // not that the fallback happened to coincide.
    let arch = classify_llm(MARKETING_CHARTER, &llm).await.unwrap();
    assert_eq!(arch, Archetype::OpsOnCall);
}

#[tokio::test]
async fn llm_returns_invalid_slug_falls_through_to_keyword() {
    let llm = MockProviderClient::ok("totally_made_up_archetype");
    let arch = classify_llm(MARKETING_CHARTER, &llm).await.unwrap();
    assert_eq!(
        arch,
        Archetype::MarketingIntern,
        "unknown slug must fall through to the keyword classifier",
    );
}

#[tokio::test]
async fn llm_network_error_falls_through_to_keyword() {
    let llm = MockProviderClient::err(ArgoError::Provider(
        "simulated network timeout".into(),
    ));
    let arch = classify_llm(MARKETING_CHARTER, &llm).await.unwrap();
    assert_eq!(
        arch,
        Archetype::MarketingIntern,
        "provider/network error must fall through to the keyword classifier",
    );
}

#[tokio::test]
async fn llm_empty_response_falls_through_to_keyword() {
    let llm = MockProviderClient::ok("");
    let arch = classify_llm(MARKETING_CHARTER, &llm).await.unwrap();
    assert_eq!(
        arch,
        Archetype::MarketingIntern,
        "empty model response must fall through to the keyword classifier",
    );
}

#[tokio::test]
async fn llm_mixed_case_slug_is_normalized() {
    // Tests both case-folding AND `-` → `_` swap that `from_slug`
    // performs, so e.g. `"Sales-SDR"` resolves to `Archetype::SalesSdr`.
    let llm = MockProviderClient::ok("  Sales-SDR  ");
    let arch = classify_llm(MARKETING_CHARTER, &llm).await.unwrap();
    assert_eq!(arch, Archetype::SalesSdr);
}
