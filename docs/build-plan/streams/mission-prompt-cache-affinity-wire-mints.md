# Stream mission-prompt-cache-affinity-wire-mints — wire #1539 primitive into the real mint site (`create_session_with_worker`)

**Owner**: claude. **Depends on**: mission-prompt-cache-session-affinity merged (#1539, `173e24dc`).
**Authored against**: origin/main `4ededf5e` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prompt-cache-affinity-wire-mints`.

> **REWRITTEN 2026-05-13** after the first version was BLOCKED by a claude worker on 2026-05-13T06:32Z with a forensic 7-point audit. The original brief cited 4 mint sites in argo-orchestrator/argo-gateway/argo-cli that were all wrong (different SessionId types, no worker_id in scope, inside test/bench functions, would break a §21.3 test). The worker's resolution path is followed verbatim below: target `SessionStore::create_session_with_worker` + `argo-agent-runner::turn`, widen may-touch to `argo-store` + `argo-agent-runner`, and use a smoke test inside `argo-store` (no argo-provider wiring needed — the affinity primitive is self-verifying without the provider counter).

## Scope

Today every call to `SessionStore::create_session_with_worker` in `crates/argo-store/src/session_query.rs:57` does `let id = SessionId::new();` — minting a fresh SessionId regardless of whether the same `(tenant_id, worker_id)` pair just created a session within the cache TTL. The shipped `PromptCacheAffinity` primitive (`crates/argo-runtime/src/prompt_cache_affinity.rs`, #1539) is unused.

Wire it through:

1. **`SessionStore::create_session_with_worker`** consults `affinity.acquire_or_mint(input.tenant_id, &worker.to_string())` BEFORE the `SessionId::new()` call — but only when `input.worker_id.is_some()` AND affinity is enabled. When affinity returns a recently-recorded id, the existing call site contract still holds: a new row gets INSERTed with that id (the row may already exist from a recent call; the existing `INSERT INTO session` uses no `OR REPLACE` so this currently errors on dup-PK — see "Notes" below for the fix).

2. **`argo-agent-runner::run_turn_with_events_inner`** (or wherever the session row is loaded near line 165 in `turn.rs`) calls `affinity.record(tenant_id, worker_id, session_id)` once `session.worker_id` is in scope. That keeps the registry fresh whenever any turn observes a worker-bound session.

3. **Affinity plumbing**: `SessionStore` gains an optional `prompt_cache_affinity: Option<Arc<PromptCacheAffinity>>` field set by a new builder method `with_affinity`. `agent-runner` reads it from `SessionStore` (or accepts it as a parameter to `run_turn_with_events_inner` — choose the cleaner of the two, document in PR body).

4. **Smoke test (in `argo-store`)**: call `create_session_with_worker` 4 times for the same `(tenant, worker)` pair within the TTL window → assert all 4 calls return the same SessionId. Disabled-affinity path asserts 4 distinct SessionIds. **No argo-provider wiring required** — the affinity primitive's correctness is observable at the `create_session_with_worker` boundary; the cache-read counter assertion lives in a separate `mission-prompt-cache-end-to-end-smoke` follow-up brief once we widen scope to provider.

**Not in scope**: gateway-side integration (the existing `SessionCache::resolve` in `argo-gateway/src/router.rs:95` is already stronger affinity and must NOT be touched — per the BLOCK audit point (3), replacing it would break §21.3 `same_sender_resumes_session_across_adapters`). Cross-host affinity. Persistence across process restart. Provider-counter assertion (deferred to follow-up).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/session_query.rs` — narrowly scoped: modify `create_session_with_worker` to consult affinity. Add a `with_affinity` builder on `SessionStore` (or wherever the existing builder lives — `find crates/argo-store -name 'lib.rs' -o -name 'session.rs'`). Roughly ≤40 modified lines.
- `<NEW or existing tests/sibling: crates/argo-store/src/session_query_affinity_tests.rs>` — 4-turn replay smoke + disabled-config path + per-tenant isolation smoke. ≤200 LOC.
- `crates/argo-store/src/lib.rs` — narrowly scoped: re-export `PromptCacheAffinity` (re-export from argo-runtime, NOT a re-implementation) if needed for ergonomics. ≤3 modified lines.
- `crates/argo-store/Cargo.toml` — narrowly scoped: add ONE dep `argo-runtime = { path = "../argo-runtime", version = "0.0.1" }`. **Verify no dependency cycle** by running `cargo metadata 2>&1 | grep cycle` before pushing — if argo-runtime depends on argo-store, BLOCK and report.
- `crates/argo-agent-runner/src/turn.rs` — narrowly scoped: at the site where `session.worker_id` is extracted (search `worker_id` inside `run_turn_with_events_inner`), call `affinity.record(tenant, worker, session_id)`. ≤15 modified lines.
- `crates/argo-agent-runner/Cargo.toml` — narrowly scoped: ensure `argo-runtime` is a dep (already is — verify). No edit if present.

## Files you must NOT touch

- `crates/argo-runtime/src/prompt_cache_affinity.rs` — primitive locked from #1539.
- `crates/argo-config/src/sections_context.rs` — config shape locked.
- `crates/argo-orchestrator/src/session.rs` — `Session::new(tenant, title)` takes title, NOT worker_id (per BLOCK audit point 1). Out of scope.
- `crates/argo-gateway/src/router.rs` — `SessionCache::resolve` already provides STRONGER affinity than the TTL primitive (per BLOCK audit point 3). Replacing it would break §21.3 `same_sender_resumes_session_across_adapters`. Out of scope.
- `crates/argo-cli/src/handlers/route.rs` — the cited site is inside `bench()` (per BLOCK audit point 4). Out of scope.
- `crates/argo-core/src/agent.rs` — `AgentDeps` here is dead code; production agent runtime is `argo_agent_runner::run_turn` which doesn't consume it (per BLOCK audit point 5). Out of scope.
- `crates/argo-provider/**` — provider counter assertion is deferred to a follow-up brief.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `SessionStore::create_session_with_worker` consults `affinity.acquire_or_mint(tenant, worker)` when `worker_id.is_some()` AND `with_affinity` was set; otherwise mints fresh via `SessionId::new()` (the pre-brief behavior).
- [ ] `SessionStore::with_affinity(Arc<PromptCacheAffinity>) -> Self` builder method added.
- [ ] `argo-agent-runner::run_turn_with_events_inner` records the observed `(tenant, worker, session_id)` triple into affinity at the worker-id-in-scope site.
- [ ] 4-turn replay smoke test: same `(tenant, worker)` over 4 consecutive `create_session_with_worker` calls within TTL → all 4 return the same SessionId. Disabled-config path returns 4 distinct SessionIds.
- [ ] Per-tenant isolation smoke: `(tenant_a, worker_x)` and `(tenant_b, worker_x)` get distinct SessionIds even at identical wall-clocks.
- [ ] No dependency cycle (`cargo metadata` clean).
- [ ] Existing `create_session_with_worker` callers (other tests, `create_session` legacy fn) keep working unchanged because affinity defaults to `None` if not wired.
- [ ] `cargo clippy -p argo-store -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-store` passes.
- [ ] `cargo test -p argo-agent-runner` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-prompt-cache-affinity-wire-mints`
- PR title: `store: wire prompt-cache affinity into create_session_with_worker (mission-prompt-cache-affinity-wire-mints)`

## Notes for the implementer

- **Dup-PK risk**: `INSERT INTO session` currently has no `OR REPLACE`. If affinity returns an existing id and the row already exists, the INSERT fails. Two acceptable resolutions: (a) `INSERT ... ON CONFLICT(id) DO NOTHING` (returns the existing row), (b) the `create_session_with_worker` short-circuits to a SELECT lookup when affinity returns a recently-recorded id. Pick (a) — it's the smaller diff and SQLite supports it natively. Document the choice in the PR body.
- **Dependency direction**: confirm `argo-runtime` does NOT depend on `argo-store` before adding `argo-store → argo-runtime`. If a cycle exists, the primitive may need to move to `argo-types` instead — BLOCK and ask the conductor to coordinate.
- **`with_affinity` placement**: if `SessionStore` is already constructed via a builder pattern, add the new method next to existing builders. If `SessionStore::new` is the only constructor, add `with_affinity` as a chained method (`SessionStore::new(...).with_affinity(...)`).
- **Session-cache vs prompt-cache**: do not confuse them. The Anthropic prompt-cache is provider-side. This brief only wires the session-id affinity that makes the provider cache hit-able. Provider-side counter assertion is a follow-up.
- The brief is `mission-`-prefixed because the original was, and the BLOCK note refers to the same id. Keep the slug stable.
