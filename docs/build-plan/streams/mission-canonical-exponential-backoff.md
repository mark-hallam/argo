# mission-canonical-exponential-backoff — Canonical exponential backoff primitive across argo-provider

**Owner**: claude. **Depends on**: nothing (extends existing argo-mcp-connector backoff primitive).
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-canonical-exponential-backoff`.

## Scope

MISSION_BACKLOG item 270. The argo-mcp-connector crate has a backoff primitive (PR #1301). `argo-provider` (the LLM call path) has ad-hoc retries that don't use it. This brief consolidates: argo-provider adopts the same backoff primitive; tests prove identical behavior. Note: streaming-aware retry shipped via #1467 (mission-streaming-aware-retry MERGED), so this brief is the NON-streaming complement.

LOC: ~500.

## Files you may touch

- `crates/argo-provider/src/retry.rs` — TOUCH or NEW (verify which at claim time). Adopt argo-mcp-connector backoff primitive.
- `crates/argo-provider/src/retry_tests.rs` — TOUCH or NEW. Test parity with mcp-connector's tests.
- argo-mcp-connector backoff primitive — read-only (extend the trait if needed, but don't refactor the impl).

## Files you must NOT touch

- argo-mcp-connector internals (consume); streaming-retry path (already shipped via #1467).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] argo-provider retries use the canonical backoff primitive (no ad-hoc loops).
- [ ] Behavioral parity with argo-mcp-connector's retries proven by shared test fixtures.
- [ ] Both crates' tests still green.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-canonical-exponential-backoff`
- PR title: `Canonical exponential backoff primitive across argo-provider (mission-canonical-exponential-backoff)`

## Notes for the implementer

- Mission-claim alignment: claim 5 (real-world execution resilience — handles transient provider failures cleanly).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).
