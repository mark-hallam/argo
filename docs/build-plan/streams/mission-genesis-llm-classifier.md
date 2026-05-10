# Mission load-bearing wire — replace hardcoded archetype classifier with LLM-driven

**Owner**: claude. **Depends on**: `Archetype` enum stable (verified at `crates/argo-genesis/src/archetype.rs`); existing offline classifier is the fallback.
**Authored against**: origin/main `e17857b0` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-llm-classifier`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 3 item #58 ("Replace 12-archetype hardcoded keyword classifier with model-driven classifier"). Today's classifier in `archetype.rs` is offline + keyword-based (per the doc-comment header: "v1 ships an offline keyword classifier — fast, deterministic, testable without an API key"). Mission Claim 3 ("platform builds workers / Genesis") promises real classification for charter→archetype matching; the keyword classifier produces brittle results outside the original 12 categories.

## Scope

Add an LLM-driven classifier alongside the existing offline keyword classifier — DO NOT remove the keyword path. The two coexist:

- **Offline (existing)** — `Archetype::classify_keywords(charter)` — fast, deterministic, no API key needed. Stays as fallback for tests + offline development.
- **LLM-driven (NEW)** — `pub async fn classify_llm(charter: &str, llm: &dyn ProviderClient) -> ArgoResult<Archetype>` — calls the workspace's existing provider client (e.g. via `argo-provider`) with a structured prompt, parses a single-archetype-slug response, falls back to keyword classifier on any error.

The Genesis pipeline (`pipeline.rs`) gets a config flag (`use_llm_classifier: bool`, default `false` for v1 conservatism). When enabled and a provider is wired, the pipeline calls `classify_llm` first; on failure, falls back to keyword.

This is NOT in scope: removing the keyword classifier; supporting non-archetype output (that's #65, separate brief); per-tenant model selection; A/B testing classifier variants.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-genesis/src/archetype.rs` — narrowly scoped: ADD `pub async fn classify_llm(charter: &str, llm: &dyn ProviderClient) -> ArgoResult<Archetype>` method. Existing `classify_keywords` (or whatever the existing fn is named — verify in first 5min) stays intact.
- `crates/argo-genesis/src/archetype_tests.rs` (or sibling) — narrowly scoped: append 5 NEW unit tests using a `MockProviderClient` from a test fixture: llm-returns-valid-slug; llm-returns-invalid-slug-falls-through; llm-network-error-falls-through; llm-empty-response-falls-through; llm-mixed-case-slug-normalized.
- `crates/argo-genesis/src/pipeline.rs` — narrowly scoped: extend `GenesisPipeline` config with `use_llm_classifier: bool` + a single dispatch arm `if cfg.use_llm_classifier && let Some(client) = &llm { match classify_llm(...).await { Ok(a) => a, Err(_) => classify_keywords(charter) } } else { classify_keywords(charter) }`.
- `crates/argo-genesis/Cargo.toml` — narrowly scoped: confirm `argo-provider` is a dependency (or add via narrow trait abstraction in `archetype.rs` so the surface is `&dyn ProviderClient`, not pulling all of argo-provider into argo-genesis).

## Files you must NOT touch

- `crates/argo-provider/**` — provider client is upstream; reuse only
- `crates/argo-genesis/src/templates/**` — archetype templates are downstream
- Any non-genesis crate
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `Archetype::classify_llm(charter, llm)` exists and is async
- [ ] On any error from the LLM call (network, timeout, parse failure, empty response), function returns the keyword classifier's result — NOT propagating the error
- [ ] LLM prompt is structured (system + user message asking for one of the 12 archetype slugs); the parser accepts mixed case and normalizes to `Archetype::from_slug`
- [ ] When LLM returns an unknown slug, falls back to keyword classifier (no panic, no error)
- [ ] Pipeline config `use_llm_classifier` defaults to `false`
- [ ] When `use_llm_classifier=true` AND provider is wired: pipeline uses LLM; falls back to keyword on error
- [ ] When `use_llm_classifier=false` OR no provider: pipeline uses keyword classifier (today's behavior preserved)
- [ ] 5 new unit tests cover the matrix
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-genesis` clean
- [ ] `cargo clippy -p argo-genesis --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-genesis` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-genesis-llm-classifier`
- PR title: `genesis: LLM-driven archetype classifier with keyword fallback (mission-genesis-llm-classifier)`

## Notes for the implementer

- Define a narrow `pub trait ProviderClient: Send + Sync { async fn classify(&self, prompt: &str) -> ArgoResult<String>; }` in `archetype.rs` so this brief doesn't pull all of argo-provider into argo-genesis. The real provider impl can live in argo-provider as a separate brief; for now, tests use a `MockProviderClient`.
- The LLM prompt should be deterministic: system says "respond with exactly one slug from this list: ..."; user is the charter. Temperature should be 0.
- Slug normalization: `Archetype::from_slug(s.trim().to_lowercase().replace('-', '_'))` — make this robust to model output quirks.
- The fallback-on-error is load-bearing: NEVER let an LLM hiccup break Genesis. The keyword classifier is always available.
