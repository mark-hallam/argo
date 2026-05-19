# Stream Audit-P2-3 — provider-backed JD-to-charter synthesizer

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-genesis-jd-llm-summarizer`.
**Source**: 2026-05-19 codex audit P2-5, validated by claude `VALIDATION.md` (CONFIRMED — `crates/argo-genesis/src/job_description.rs:175-176` is literally `fn synthesize_charter(raw: &str) -> String { raw.to_string() }`).

## Scope

The "drop in a real job description → working teammate" onboarding path (a key Skycot UX claim) currently passes the raw JD text directly through as the worker's charter, no synthesis. For demos with clean JDs this works; for real customer JDs (messy PDF extractions, 8-page boilerplate-heavy docs, JDs spanning multiple roles), the charter is junk and the worker behaves unpredictably.

`crates/argo-genesis/src/job_description.rs:31-33` documents this as an identity-fn stub awaiting `mission-genesis-llm-classifier`. This brief is that classifier: a provider-backed JD-to-charter call that produces a structured `WorkerCharter` (1-paragraph mission, 3-5 responsibilities, top-3 tools, escalation contact, vertical hint).

Approach: add a `JobDescriptionSynthesizer` trait with a `StubSynthesizer` (current identity behavior, kept for tests) and an `LlmSynthesizer` (calls the default provider with a structured-output prompt + schema). Config flag `genesis.jd_synthesizer = "stub" | "llm"` selects at boot. Eval seeds cover three case types: clean JD, messy PDF extract, multi-role JD.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/job_description.rs` — keep `synthesize_charter(raw)` as the public entry point but route through `Synthesizer` trait. Remove the identity-fn body; replace with `synthesizer.synthesize(raw).await?`. The `Synthesizer` field lives on whatever struct hosts genesis state (probably a `Genesis` orchestrator type — adapt to existing shape).
- `crates/argo-genesis/src/synthesizer.rs` (NEW) — `#[async_trait] pub trait Synthesizer: Send + Sync { async fn synthesize(&self, raw: &str) -> ArgoResult<WorkerCharter>; }`. Implement `StubSynthesizer` (returns identity-charter, used in tests + dev fallback) and `LlmSynthesizer` (wraps a `ProviderHandle`, builds the structured-output prompt, parses the JSON response into `WorkerCharter`).
- `crates/argo-genesis/src/charter.rs` (NEW or extend existing) — `pub struct WorkerCharter { mission: String, responsibilities: Vec<String>, tools_hint: Vec<String>, escalation_contact: Option<String>, vertical_hint: Option<String>, source_jd_hash: String }`. The hash is SHA-256 of the raw JD — used to dedupe + cache.
- `crates/argo-genesis/eval/jd_synthesis.jsonl` (NEW) — at least 6 fixtures: 2× clean JD, 2× messy PDF extract (long boilerplate + the actual role buried inside), 2× multi-role JD (ambiguous). Each fixture has the raw input + a golden charter (manually authored; can be approximate — eval uses similarity, not byte-equality).
- `crates/argo-config/src/sections.rs` (or a `sections_genesis.rs`) — add `[genesis] jd_synthesizer = "stub"` (default; back-compat). Setting to `"llm"` activates the provider-backed path.
- `crates/argo-genesis/src/synthesizer_tests.rs` (NEW) — three tests: (1) stub synthesizer round-trips raw → charter with identity mission; (2) LLM synthesizer with a `MockProvider` returning a scripted JSON response produces the expected `WorkerCharter`; (3) malformed provider response returns a clear `ArgoError::Provider` (not a panic).

## Files you must NOT touch

- Any provider crate (`crates/argo-provider*`) — re-use existing `ProviderHandle` machinery.
- The downstream consumer of `WorkerCharter` (whatever the rest of Genesis does with the charter today). That consumer keeps reading `mission` as a `String`; the additional fields are opt-in via accessor methods, no breaking change.
- `verticals/` content — vertical authoring is unrelated.
- Web frontend — the SPA's hire-flow stays the same; this is server-side synthesis.

## Acceptance criteria

- [ ] `Synthesizer` trait + two impls (`StubSynthesizer`, `LlmSynthesizer`).
- [ ] `[genesis] jd_synthesizer = "stub" | "llm"` config flag; default `"stub"` for back-compat.
- [ ] `LlmSynthesizer` uses the default provider and a structured-output prompt; parses response into `WorkerCharter` with field-level validation (mission non-empty, responsibilities count 1-7, etc.).
- [ ] At least 6 eval fixtures in `jd_synthesis.jsonl` covering clean / messy / multi-role cases.
- [ ] Three new unit tests in `synthesizer_tests.rs` pass.
- [ ] `argo vertical eval product-manager` (or whichever genesis-eval target exists) runs the eval suite and reports per-case scores.
- [ ] No regression in existing genesis tests (`cargo test -p argo-genesis` clean).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim in PR body.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-audit-genesis-jd-llm-summarizer`
- PR title: `genesis: provider-backed JD → charter synthesizer + eval seeds (mission-audit-genesis-jd-llm-summarizer)`

## Notes for the implementer

- The structured-output prompt should specify the `WorkerCharter` schema as JSON in the system prompt and request `"output_format": "json"`. Use the existing provider's structured-output mode (Anthropic + OpenAI both support this).
- For messy PDF input: the synthesizer can pre-process (strip excessive whitespace, dedupe paragraphs, truncate to first 8KB) — but this is best done in a `pre_process(raw)` helper inside the synthesizer, not in the trait shape.
- The `source_jd_hash` enables a future cache: same JD → cached charter. NOT in this brief; just emit the hash on the struct.
- Eval fixtures' golden charters: hand-author one charter per fixture that represents "what a good human PM would write." Eval similarity uses cosine of `mission` embedding + Jaccard of `responsibilities`. If eval infra doesn't exist yet, log golden mismatches without failing — implementer decides if blocking.
- If the provider response is malformed (not parseable JSON), the synthesizer falls back to the stub behavior with a `tracing::warn!` — fail-soft for onboarding UX, not fail-closed. Document this in the synthesizer rustdoc.

## Out of scope (follow-up briefs)

- A web-UI for the operator to edit the synthesized charter before confirming the hire — Track L Phase 2.
- Multi-language JD support — assume English-only for v1.
- PDF → text extraction (the synthesizer accepts pre-extracted text only; PDF parsing is the upload pipeline's job).
- Fine-tuning a small model on JD-to-charter pairs — premature; structured prompting on a frontier model is the baseline.
- Charter versioning when the operator re-uploads a JD — separate brief; for now last-write-wins.
