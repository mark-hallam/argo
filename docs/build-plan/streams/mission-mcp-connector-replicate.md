# Mission load-bearing wire — Replicate MCP connector

**Owner**: claude. **Depends on**: existing MCP connector scaffold (`crates/argo-mcp-connector-shared/` retry primitive; `crates/argo-mcp-connector-image-gen/` for the OpenAI-compatible image-gen analogue just-merged in #1326).
**Authored against**: origin/main `01aa2f5d` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-replicate`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #114 ("Add MCP connector Replicate (any-model HF endpoint)"). **Tier A (tool).** Replicate is the universal Hugging-Face-model-hosting endpoint — any model published to Replicate (image, audio, video, LLM, vision) can be called through one API shape. This connector wraps the Replicate REST API so SKYCOT verticals can route to ANY Replicate-hosted model without per-model integration work. Complements the just-merged `argo-mcp-image-gen` (#1326) which covered OpenAI-shaped image endpoints — Replicate covers the long-tail (SDXL/Flux variants, stable-audio, etc.).

## Scope

NEW crate `crates/argo-mcp-connector-replicate/` mirroring `argo-mcp-connector-clearbit` shape (Bearer-token auth, polling for async predictions):

```rust
// crates/argo-mcp-connector-replicate/src/lib.rs
pub struct ReplicateConnector { client: Client, api_token: String }
impl ReplicateConnector {
    pub fn new(api_token: String) -> Self;
    /// POST /v1/predictions with model + input; polls until completed/failed/canceled.
    pub async fn run_prediction(&self, model: &str, input: serde_json::Value, timeout: Duration) -> ArgoResult<Prediction>;
    /// POST /v1/predictions returning the prediction-id immediately (caller polls).
    pub async fn start_prediction(&self, model: &str, input: serde_json::Value) -> ArgoResult<PredictionId>;
    /// GET /v1/predictions/<id> — caller-paced polling.
    pub async fn get_prediction(&self, id: &PredictionId) -> ArgoResult<Prediction>;
    /// POST /v1/predictions/<id>/cancel
    pub async fn cancel_prediction(&self, id: &PredictionId) -> ArgoResult<()>;
    /// GET /v1/models/<owner>/<name> — capability + input-schema discovery.
    pub async fn get_model(&self, owner: &str, name: &str) -> ArgoResult<ModelInfo>;
    /// GET /v1/account — verify token + return billing/usage tier.
    pub async fn account(&self) -> ArgoResult<AccountInfo>;
}
pub struct Prediction {
    pub id: PredictionId,
    pub model: String,
    pub status: PredictionStatus,    // Starting | Processing | Succeeded | Failed | Canceled
    pub output: Option<serde_json::Value>,  // model-specific shape
    pub error: Option<String>,
    pub metrics: Option<PredictionMetrics>, // predict_time / total_time / input_token_count etc.
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}
pub struct ModelInfo {
    pub owner: String,
    pub name: String,
    pub description: String,
    pub default_example: Option<serde_json::Value>,
    pub latest_version_id: String,
    pub run_count: u64,
    pub cover_image_url: Option<String>,
    pub license_url: Option<String>,
}
pub struct AccountInfo {
    pub type_: String,             // "user" | "organization"
    pub username: String,
    pub name: String,
}
```

Polling strategy for `run_prediction`: exponential backoff (1s, 2s, 4s, 8s, 15s ceiling) until `status` is terminal (`Succeeded`/`Failed`/`Canceled`) or `timeout` elapses. On timeout, returns `ArgoError::Provider` with hint. Token cache for /account check (60s TTL) to avoid auth-churn on parallel calls.

This is NOT in scope: webhook-based completion (operators can use the polling path or wire webhooks in a follow-up brief once Replicate's webhook schema stabilizes); training-job APIs (Replicate's `/v1/trainings/*` is a separate brief); streaming-output for LLM models (Replicate's SSE endpoint is a separate brief); custom-model upload (`/v1/models` POST); collections/lists; cog-CLI integration.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-replicate/` (NEW crate, ≤ 1000 LOC total)
  - `Cargo.toml` (NEW, ≤ 25 LOC)
  - `src/lib.rs` (NEW, ≤ 100 LOC) — re-exports
  - `src/client.rs` (NEW, ≤ 360 LOC) — `ReplicateConnector` + REST (`https://api.replicate.com/v1/...`)
  - `src/types.rs` (NEW, ≤ 240 LOC) — DTOs + serde derives + `PredictionStatus` enum
  - `src/poll.rs` (NEW, ≤ 100 LOC) — backoff polling helper
  - `src/client_tests.rs` (NEW, sibling, ≤ 260 LOC) — 6 unit tests w/ wiremock: prediction-success-immediate, prediction-success-after-polling-2-ticks, prediction-failed-returns-typed-error, model-info-discovery, cancel-prediction, account-cache-hit-no-second-call
  - `tests/replicate_smoke.rs` (NEW, ≤ 150 LOC) — gated by `ARGO_REPLICATE_TEST_TOKEN`; run a cheap model (e.g. `stability-ai/sdxl` with low-step input or any free `meta/llama` 7B with 1-token output) + assert prediction completes with status=Succeeded within 60s
- `Cargo.toml` (workspace) — narrowly scoped: add `crates/argo-mcp-connector-replicate` to `members`

## Files you must NOT touch

- `crates/argo-mcp-connector-{shared,hubspot,clearbit,image-gen,telegram,mailgun,discord}/**` — sibling stable
- `crates/argo-store/migrations/**` — no schema
- Frontend / verticals

## Acceptance criteria

- [ ] NEW `crates/argo-mcp-connector-replicate/` crate compiles standalone
- [ ] `ReplicateConnector::new` constructs with api_token (no other params)
- [ ] Bearer auth: `Authorization: Token <api_token>` (NOT `Bearer <token>` — Replicate's docs are unusual; verify in their API docs)
- [ ] `run_prediction` polls with backoff and returns `Succeeded` Prediction within ceiling, OR returns typed timeout error
- [ ] `start_prediction` returns id immediately without polling (operator pacing)
- [ ] `get_prediction` is idempotent (callable many times)
- [ ] `cancel_prediction` returns 200 on success
- [ ] `get_model` returns ModelInfo for a known model (e.g. `stability-ai/sdxl`)
- [ ] `account` returns AccountInfo + cached for 60s — second call within 60s does NOT hit the API
- [ ] 404 on get_prediction returns typed `ArgoError::Provider`, NOT panic
- [ ] 401 on any call returns typed `ArgoError::ProviderAuth`, NOT silent failure
- [ ] 6 unit tests with wiremock + integration smoke gated by env var
- [ ] `cargo clippy -p argo-mcp-connector-replicate --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-replicate` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-replicate`
- PR title: `mcp-connector: Replicate REST + prediction-polling (mission-mcp-connector-replicate)`

## Notes for the implementer

- Replicate's auth header is `Authorization: Token <api_token>`, NOT `Bearer`. Their docs explicitly use the `Token` scheme. Get this wrong and every call returns 401.
- Prediction status state machine: `Starting → Processing → (Succeeded | Failed | Canceled)`. The polling loop should treat `Starting` and `Processing` as continue-polling; the three terminal states should exit the loop.
- Pricing visibility: `Prediction.metrics.predict_time` is the billing-relevant duration. Surface it so the calling vertical can attribute spend.
- The integration smoke should pick a CHEAP model — `meta/meta-llama-3-8b-instruct` with `max_new_tokens=1` costs fractional cents and returns in under 30s. Avoid Flux / SDXL in the smoke unless you accept ~$0.05/run cost — too expensive for CI.
- Cancel semantics: 200 means "cancellation accepted", not "cancellation completed". The prediction may still emit a Canceled-status row after some delay. Document this in module docs.
- Mirror `argo-mcp-connector-clearbit` for the polling-with-backoff helper — same exponential-backoff schedule (1/2/4/8/15s, ceiling 30s) is the established pattern.
- Output shape per-model varies wildly (image URLs / text / audio URLs / JSON arrays). Don't try to normalize — `Prediction.output` is intentionally `serde_json::Value`. The calling vertical knows what shape to expect from each model it uses.
- License surfacing: `ModelInfo.license_url` lets callers refuse to run models with restrictive licenses. Don't enforce in the connector; surface it for the calling vertical to decide.
