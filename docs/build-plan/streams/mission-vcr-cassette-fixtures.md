# Mission load-bearing wire — VCR-cassette fixtures for silent-skip tests

**Owner**: claude. **Depends on**: existing silent-skip pattern (`rg -c 'if std::env::var\("ARGO_TEST_.*"\).is_err\(\)' crates/`); `mission-silent-skip-nightly-integration` (queued — once it lands, this brief is the follow-up that replaces silent-skip with deterministic CI coverage).
**Authored against**: origin/main `f1b6b0d1` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vcr-cassette-fixtures`.

> SOURCE: MISSION_BACKLOG.md Section 1 #12 ("Replace silent-skip pattern with VCR/cassette fixtures so live tests run deterministically in CI"). **Tier M (test-determinism).** Today ~65 integration tests are gated by env-var presence (`ARGO_TEST_*_KEY`) and silently skip when keys are absent. The nightly-runner brief (#11) makes them runnable in CI with real secrets; this brief is the DETERMINISTIC alternative — record real-API exchanges to JSON-cassette fixtures, replay them in CI for free + reliable coverage. Live tests still run nightly via the runner brief (#11) to catch real-API drift; the cassettes catch wire-format regressions in milliseconds.

## Scope

NEW `crates/argo-test-cassette/` crate (≤ 600 LOC across 4 files):

- `crates/argo-test-cassette/Cargo.toml` (NEW, ≤ 25 LOC) — pulls in `serde_json`, `hyper`, `bytes`. Dev-dep `wiremock` for the integration smoke.
- `crates/argo-test-cassette/src/lib.rs` (NEW, ≤ 60 LOC) — re-exports + crate-level rustdoc with usage example.
- `crates/argo-test-cassette/src/cassette.rs` (NEW, ≤ 200 LOC) — `Cassette` type: load/save JSON cassette files, sequential-match HTTP request → response pairs, RECORD-mode pass-through (forwards to real provider + appends to cassette), REPLAY-mode strict-match (no real network).
- `crates/argo-test-cassette/src/recorder.rs` (NEW, ≤ 200 LOC) — `CassetteRecorder` wraps `hyper::Client` (or `reqwest::Client`) and tees outgoing requests + incoming responses to the cassette. Mode driven by env var `ARGO_TEST_CASSETTE_MODE` ∈ `{record, replay, passthrough}` (default `replay`).
- `crates/argo-test-cassette/src/cassette_tests.rs` (NEW, ≤ 130 LOC, included via `#[path]`) — 6 unit tests: round-trip record→replay, sequential-match ordering enforcement, mismatch-on-replay error, env-driven mode selection, secret-redaction in cassette body (Authorization / api-key / Bearer-token redacted to `<REDACTED>` literal), header-allowlist filter.

Cassette JSON shape (documented in `lib.rs` example):

```json
{
  "version": "1",
  "exchanges": [
    {
      "request": {
        "method": "POST",
        "url": "https://api.anthropic.com/v1/messages",
        "headers": { "content-type": "application/json" },
        "body_sha256": "...",
        "body_json_redacted": { "...": "..." }
      },
      "response": {
        "status": 200,
        "headers": { "content-type": "application/json" },
        "body_json": { "...": "..." }
      },
      "recorded_at": "2026-05-12T03:00:00Z"
    }
  ]
}
```

NEW `docs/skycot/runbook/cassette-fixtures.md` (≤ 250 LOC):

- What VCR-style cassettes are + why we use them
- How to record a cassette: `ARGO_TEST_CASSETTE_MODE=record ARGO_TEST_ANTHROPIC_KEY=... cargo test --test <name>`
- How replay-mode works in CI: `ARGO_TEST_CASSETTE_MODE=replay cargo test --test <name>` (default — no env var needed)
- How to refresh a cassette when the upstream API contract changes
- Secret-redaction guarantees: which headers + body fields are scrubbed; what is NOT scrubbed (operator reviews recorded JSON before commit)
- Sequential-match ordering: cassettes assume same-order request playback; non-determinism in test order breaks replay (test author's responsibility to keep order stable)
- Failure modes: mismatch (request shape differs from recording), short-cassette (test makes more requests than recorded), under-replay (test makes fewer)

NO silent-skip TEST FILE MIGRATION in this brief — that's a per-test follow-up. This brief lands the cassette infrastructure + one DEMO integration test using a cassette to prove the recipe (the demo can be in `argo-test-cassette` itself or `crates/argo-mcp-connector-mailgun` since Mailgun has a small live surface). Document the migration recipe in the runbook so per-test follow-ups can fan out.

This is NOT in scope: per-test migration of all 65 silent-skip tests (separate follow-up; each test owner converts theirs); SSE-streaming response cassettes (the v1 cassette is request-response only; streaming is a v2 extension noted in the runbook); WebSocket cassettes (out of scope; web-sockets are separate from HTTP integration); secret-replacement-by-pattern (the v1 redaction is header-allowlist + JSON-key-blocklist; broader matching is a v2); cassette-version migrations (v1 only for this brief; format evolution is a future brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-test-cassette/Cargo.toml` (NEW, ≤ 25 LOC)
- `crates/argo-test-cassette/src/lib.rs` (NEW, ≤ 60 LOC)
- `crates/argo-test-cassette/src/cassette.rs` (NEW, ≤ 200 LOC)
- `crates/argo-test-cassette/src/recorder.rs` (NEW, ≤ 200 LOC)
- `crates/argo-test-cassette/src/cassette_tests.rs` (NEW, ≤ 130 LOC)
- `crates/argo-test-cassette/fixtures/sample.cassette.json` (NEW, ≤ 50 LOC) — the demo cassette
- `Cargo.toml` (workspace EXTEND only — add `crates/argo-test-cassette` to `members`)
- `docs/skycot/runbook/cassette-fixtures.md` (NEW, ≤ 250 LOC)

## Files you must NOT touch

- `crates/argo-store/migrations/**`
- Existing `crates/argo-mcp-connector-*/tests/` files (per-test migration is a separate follow-up)
- Existing silent-skip test files in any crate (this brief proves the recipe; per-test migration follows)
- `Cargo.lock` (re-generated by cargo, but no direct edits)

## Acceptance criteria

- [ ] `cargo build -p argo-test-cassette` succeeds
- [ ] `cargo test -p argo-test-cassette` passes all 6 unit tests
- [ ] One demo cassette file at `crates/argo-test-cassette/fixtures/sample.cassette.json` round-trips through record→replay
- [ ] `ARGO_TEST_CASSETTE_MODE=record cargo test -p argo-test-cassette --test <name>` re-writes the cassette (with redacted secrets)
- [ ] `ARGO_TEST_CASSETTE_MODE=replay cargo test -p argo-test-cassette --test <name>` runs without real network (verified via wiremock or by `ARGO_TEST_DENY_NETWORK=1` env in CI)
- [ ] Secrets are redacted in the recorded cassette: Authorization header → `<REDACTED>`, body keys matching `api_key|secret|password|token|bearer` → `<REDACTED>` literal
- [ ] Runbook at `docs/skycot/runbook/cassette-fixtures.md` covers record/replay/refresh/redact/order/failure-modes
- [ ] All new files < 500 LOC; crate total < 600 LOC
- [ ] NO Rust changes outside the new crate + workspace `members` add
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-vcr-cassette-fixtures`
- PR title: `test-infra: VCR-style cassette fixtures (record/replay HTTP for deterministic CI coverage) (mission-vcr-cassette-fixtures)`

## Notes for the implementer

- This brief lands the INFRASTRUCTURE — the per-test migration of all 65 silent-skip tests is a follow-up brief (or fan-out per test owner). Keep the scope tight here: the crate + one demo + the runbook.
- Mode selection via `ARGO_TEST_CASSETTE_MODE` env: default `replay` so CI runs in replay mode by default (deterministic), `record` is opt-in for cassette-refresh, `passthrough` is for ad-hoc debug (real network, no recording).
- Sequential-match ordering is the v1 simplification: the cassette is a JSON array; the recorder pushes; the replayer pops in order. If a test makes requests in a different order than the recording, replay fails. Document this as a v1 limitation.
- Secret redaction is critical — cassettes get COMMITTED to git. The redaction must scrub Authorization / api-key / Bearer / X-API-Key / X-Auth-Token headers AND body JSON keys matching `(?i)(api_key|secret|password|token|bearer)`. Cassette tests assert this.
- The `body_sha256` field in the request shape is a cheap way to verify replay-match without exposing body content if the body itself contains a secret that the redaction missed (defense-in-depth).
- The runbook should document the migration recipe for an existing silent-skip test:
  1. Add `let cassette = Cassette::load("fixtures/<name>.cassette.json")?;` at test top
  2. Replace `reqwest::Client::new()` with `cassette.client()`
  3. Drop the `if std::env::var("ARGO_TEST_*").is_err() { return; }` guard
  4. Run `ARGO_TEST_CASSETTE_MODE=record cargo test --test <name>` once with real creds
  5. Commit the cassette + the simplified test
- DO NOT add HTTP/2 multiplexing or HTTP/3 support to v1. Stick to HTTP/1.1 request-response. Streaming + WS are explicitly out of scope.
- `wiremock` is the dev-dep for the in-memory smoke; don't reach for a runtime cassette dep — keep this in-house so we control format evolution.
- If the implementer wants to use the `vcr` or `cassette` crate from crates.io instead of authoring from scratch, that's a sensible deviation IF: (a) the chosen crate has the secret-redaction primitive, (b) the cassette format is JSON-readable, (c) the dep tree is minimal. Justify in the PR body.
