# ghostwriter-real-llm-routing (FINAL SPRINT) — real provider draft, deterministic fallback

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ghostwriter-real-llm-routing`.

## Why (evidence)
`POST /api/v1/skills/draft-from-prompt` is hard-bound to a **deterministic offline
client**:
`crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs:61`
(`let llm: OfflineGhostwriterClient = OfflineGhostwriterClient;`), and the module
header (line 9) says "v1 wires the deterministic `OfflineGhostwriterClient`". So
Ghostwriter draft output is canned, not LLM-generated.

The **real provider path is already used by sibling routes** via
`argo_provider::Transport` (genesis/pitch/judge call `complete_once →
argo_provider::Transport`): see `routes_genesis_author.rs:6`,
`routes_genesis_stream_llm.rs:9`, `routes_orchestration_team_llm.rs:15`,
`argo-judge/src/lib.rs:12`. Swap Ghostwriter onto that same transport, truth-bounded,
**falling back to the deterministic client on provider error** (NO_THEATER: no
silent canned output when a real key is present and working).

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'OfflineGhostwriterClient|draft_from_prompt_response' crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs` — confirm the swap point.
- `rg -n 'complete_once|argo_provider::Transport' crates/argo-cli/src/handlers/gateway/routes_genesis_author.rs crates/argo-cli/src/handlers/gateway/routes_genesis_stream_llm.rs` — copy the EXACT transport-acquisition + env-gate pattern these use; do NOT invent a new provider-wiring helper.
- `rg -n 'pub trait .*Ghostwriter|GhostwriterClient' crates/` — confirm the client trait so the real path and the offline fallback share ONE interface (no parallel trait).
- `rg -n 'ARGO_.*LLM|provider.*configured|transport.*available' crates/argo-cli/src/handlers/gateway/routes_genesis_author.rs` — reuse the same "is a real provider configured?" gate so behavior matches genesis/pitch.

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs` (replace the
  unconditional `OfflineGhostwriterClient` with: try the real `argo_provider::Transport`
  draft via `complete_once` exactly as genesis/pitch do; on `Err`, fall back to
  `OfflineGhostwriterClient` and tag the response `source` so the caller can tell real
  from fallback). Keep < 500 LOC; the draft helper fn < 80 LOC — extract a sibling
  `routes_ghostwriter_llm.rs` if needed.
- new (only if needed) `crates/argo-cli/src/handlers/gateway/routes_ghostwriter_llm.rs`.
- the existing test module in `routes_ghostwriter.rs` (extend it).

## Acceptance (truth-bounded + testable)
- A test with a **mock/stub Transport** (the same test seam genesis/judge use) proves
  the real path is taken when a transport is present: the returned draft is the mock
  transport's output, NOT the `OfflineGhostwriterClient` canned text, and the response
  `source` reads `provider` (or equivalent).
- A test with the transport forced to error proves the **deterministic fallback**
  engages (200, offline draft, `source: offline`) — graceful degrade, no 500.
- The existing offline-path tests (e.g. the malformed-body test at line 278) stay
  green.
- `cargo test -p argo-cli` green.

## On ship
Flip the FEATURE_MATRIX Ghostwriter row (offline-only → real-provider w/ deterministic
fallback) with the commit SHA + route in the SAME commit, then
`python3 scripts/render-feature-matrix.py`.

## Effort
S (Rust). ~120–200 LOC incl. test. Additive; no schema change.
