# mission-prompt-cache-end-to-end-smoke — assert provider cache-read counter rises through the affinity-enabled session path

**Owner**: claude. **Depends on**: mission-prompt-cache-affinity-wire-mints merged (#1543).
**Authored against**: origin/main `8d54a401` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prompt-cache-end-to-end-smoke`.

## Scope

Follow-up to #1543. That PR wired `PromptCacheAffinity` into `SessionStore::create_session_with_worker` and asserted at the store-boundary level (4-turn replay → same SessionId). The deferred operator-visible value is asserting that the **provider-side cache-read counter** climbs through the full path: store creates session → agent-runner runs turn → provider sends `cache_control` markers → cached read happens on turn 2+.

The counter is in `argo-telemetry::metrics_prompt_cache::snapshot()`. The only call site that records into it is `crates/argo-provider/src/anthropic/wire.rs:275`. An end-to-end smoke needs to drive a recorded HTTP fixture (wiremock) through the agent loop and assert the counter rises.

This brief ships **a single integration test** under `crates/argo-agent-runner/tests/` (or `crates/argo-provider/tests/`) that:

1. Sets up an in-memory `SessionStore` with `with_affinity(Arc<PromptCacheAffinity::enabled(270s)>)`.
2. Sets up a wiremock-backed `argo-provider` HTTP fixture returning a deterministic Anthropic response with `cache_creation_input_tokens` + `cache_read_input_tokens` headers/fields.
3. Drives 2 turns: turn 1 calls `create_session_with_worker(tenant, worker)` → drives `run_turn_with_events` → asserts `metrics_prompt_cache::snapshot()` shows `cache_creation_input_tokens > 0`, `cache_read_input_tokens == 0`.
4. Turn 2 reuses the SAME `(tenant, worker)` pair: asserts the SessionId returned matches turn 1, and after `run_turn_with_events` the counter shows `cache_read_input_tokens > 0` (i.e., the cache HIT).

**Disabled-affinity path**: a sister test asserts that without `with_affinity`, the SessionIds differ AND `cache_read_input_tokens` stays at zero across 2 turns (no warm cache).

**Not in scope**: changing the provider counter implementation, adding new provider features, wiring real Anthropic HTTP calls. This is a pure observability smoke.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `<NEW: crates/argo-agent-runner/tests/prompt_cache_end_to_end_smoke.rs>` — the 2 tests (~250 LOC). Use existing wiremock patterns from `crates/argo-agent-runner/tests/` (search `wiremock::MockServer` for the established setup).
- `crates/argo-agent-runner/Cargo.toml` — narrowly scoped: ONLY if a `[dev-dependencies]` line for `wiremock` or `argo-telemetry` is missing AND needed. Verify with `cargo build -p argo-agent-runner --tests` first; if it already builds, no Cargo.toml edit needed.

## Files you must NOT touch

- `crates/argo-provider/src/anthropic/wire.rs:275` — the counter call site is correct. Don't add counters elsewhere.
- `crates/argo-telemetry/**` — the snapshot/record API is the contract this test asserts against. Don't change it.
- `crates/argo-runtime/src/prompt_cache_affinity.rs` — primitive locked from #1539.
- `crates/argo-store/src/session_query.rs` — wiring locked from #1543.

## Acceptance criteria

- [ ] `crates/argo-agent-runner/tests/prompt_cache_end_to_end_smoke.rs` exists with 2 named tests:
  - `affinity_enabled_two_turn_replay_records_cache_read`
  - `affinity_disabled_two_turns_record_no_cache_read`
- [ ] Test 1 asserts: turn 1's SessionId == turn 2's SessionId (proves affinity works); turn 2's snapshot delta shows `cache_read_input_tokens > 0`.
- [ ] Test 2 asserts: turn 1's SessionId != turn 2's SessionId (proves disabled-affinity is pre-brief behavior); turn 2's snapshot delta shows `cache_read_input_tokens == 0`.
- [ ] Wiremock fixture is deterministic (same response shape for both turns, with `cache_creation_input_tokens` set on turn 1's response, `cache_read_input_tokens` set on turn 2's response — verify by mocking the Anthropic envelope shape from `argo-provider/src/anthropic/wire.rs`).
- [ ] `cargo clippy -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-agent-runner --test prompt_cache_end_to_end_smoke` passes (2/2).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-prompt-cache-end-to-end-smoke`
- PR title: `agent-runner: end-to-end smoke for prompt-cache affinity → provider counter (mission-prompt-cache-end-to-end-smoke)`

## Notes for the implementer

- The Anthropic response envelope shape: see `crates/argo-provider/src/anthropic/wire.rs:260+` for `AnthropicUsage` — the JSON fields `cache_creation_input_tokens` and `cache_read_input_tokens` are the ones to mock.
- Use `argo_telemetry::metrics_prompt_cache::snapshot()` to read counter deltas. The snapshot is process-global; use `Mutex` or test-isolation careful comparison (snapshot before turn 1 → snapshot after turn 2 → assert delta).
- The wiremock setup pattern is well-established in `crates/argo-provider/src/anthropic/wire_tests.rs` (or similar) — reuse rather than invent.
- If the existing test infrastructure makes it hard to drive a full `run_turn_with_events` from inside a test (e.g. tokio context, real DB needed), consider writing the smoke at a lower level: directly call `argo_provider::AnthropicTransport::send(...)` twice with the same SessionId, observe counter rise. Document the choice in the PR body.
- This brief is the user-visible value of #245. Once it lands, `argo provider cache-stats --json` (already shipped via #1300) will report non-zero hit rate in production deployments that wire the affinity registry.
