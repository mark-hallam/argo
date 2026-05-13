# Stream mission-prompt-cache-affinity-wire-mints — wire #1539 primitive into the 4 SessionId mint sites

**Owner**: claude. **Depends on**: mission-prompt-cache-session-affinity merged (#1539, `173e24dc`).
**Authored against**: origin/main `173e24dc` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prompt-cache-affinity-wire-mints`.

> SOURCE: deferred from #1539 PR-body "Dispatch-site swap deferred" section. The primitive is in `argo_runtime::prompt_cache_affinity::PromptCacheAffinity` but no call site uses it yet — the cache-hit-rate counter from #1300 will only climb after this lands.

## Scope

The MERGED #1539 brief shipped the `PromptCacheAffinity` registry but punted the integration because the original brief's may-touch list cited `planner.rs`, which doesn't mint `SessionId`s. The actual mint sites are:

| File | Line | Mint call | Context |
|---|---|---|---|
| `crates/argo-orchestrator/src/session.rs` | 102 | `id: SessionId::new()` | `Session::new` — the canonical per-turn mint site for worker sessions |
| `crates/argo-orchestrator/src/session.rs` | 184 | `let id = SessionId::new()` | helper used by tests + lifecycle reset |
| `crates/argo-gateway/src/router.rs` | 95 | `let id = SessionId::new()` | gateway POST `/sessions` |
| `crates/argo-cli/src/handlers/route.rs` | 19 | `let session_id = SessionId::new()` | CLI dispatch handler |

The integration is uniform: each site that has a `(TenantId, &str /* worker_id */)` in scope swaps `SessionId::new()` for `affinity.acquire_or_mint(tenant, worker_id)` where `affinity: &Arc<PromptCacheAffinity>` is plumbed via AgentDeps. Sites that genuinely don't have a `worker_id` in scope (e.g. an admin-tools session not bound to a worker) keep `SessionId::new()` and document why in a one-line comment.

Plumb the `Arc<PromptCacheAffinity>` through `AgentDeps` (per CLAUDE.md "AgentDeps is the only injection point" invariant). Build at startup from `RuntimeConfig::prompt_cache_affinity` (the `PromptCacheAffinityConfig` already shipped in #1539). Disabled-config path returns the same `disabled()` registry.

**Acceptance gate**: after the wire-up, run a 4-turn smoke for one tenant + worker and assert via `argo_telemetry::metrics_prompt_cache::snapshot()` that the cache-read counter for the system prompt rises (vs zero pre-brief).

**Not in scope**: the `record()` callback on turn-end (the primitive only needs `acquire_or_mint` for the cache hit; `record` is for cases where the chosen session was overridden mid-turn — defer to a follow-up). Cross-host affinity. Persistence of affinity state across process restart.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-orchestrator/src/session.rs` — narrowly scoped: 2 sites (line 102 + line 184). For each, accept `affinity: &Arc<PromptCacheAffinity>` and swap `SessionId::new()` for `affinity.acquire_or_mint(tenant, worker_id)`. ≤30 modified lines total.
- `crates/argo-gateway/src/router.rs` — narrowly scoped: 1 site (line 95). Same swap. ≤15 modified lines.
- `crates/argo-cli/src/handlers/route.rs` — narrowly scoped: 1 site (line 19). Same swap; if `worker_id` isn't in scope, plumb it from the caller. ≤15 modified lines.
- `crates/argo-orchestrator/src/deps.rs` (or wherever `AgentDeps` lives — `find crates -name 'deps.rs' -o -name 'agent_deps.rs'` to confirm) — narrowly scoped: add ONE `pub prompt_cache_affinity: Arc<PromptCacheAffinity>` field; construct in the `new()` from `RuntimeConfig::prompt_cache_affinity`. ≤20 modified lines.
- `<NEW: crates/argo-orchestrator/src/session_affinity_smoke.rs>` (or `crates/argo-orchestrator/tests/affinity_smoke.rs`) — integration smoke: 4-turn replay, assert single recorded SessionId + cache-read counter rises. ≤150 LOC.

## Files you must NOT touch

- `crates/argo-runtime/src/prompt_cache_affinity.rs` — the primitive is locked from #1539.
- `crates/argo-config/src/sections_context.rs` — config shape locked.
- Any provider crate — providers don't change; they just see the same session_id on the wire.
- Any test that's currently asserting `SessionId::new()` semantics — those tests should still pass because `affinity.disabled()` is a transparent passthrough.

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] All 4 mint sites in the table above use `affinity.acquire_or_mint(tenant, worker_id)` instead of `SessionId::new()`.
- [ ] `AgentDeps` carries `prompt_cache_affinity: Arc<PromptCacheAffinity>`; constructed at startup from `RuntimeConfig::prompt_cache_affinity`.
- [ ] Sites where `worker_id` is genuinely not in scope (e.g. anonymous admin-tools turns) keep `SessionId::new()` with a `// reason: ...` one-line comment.
- [ ] 4-turn replay smoke test passes: same `(tenant, worker_id)` over 4 consecutive turns within TTL → single distinct SessionId observed in `affinity.snapshot()` (or equivalent); cache-read tokens from `argo_telemetry::metrics_prompt_cache::snapshot()` increase across turns 2-4.
- [ ] Disabled-config path (`RuntimeConfig { prompt_cache_affinity: PromptCacheAffinityConfig { enabled: false, .. }, .. }`) recovers pre-brief behavior (fresh SessionId every turn).
- [ ] `cargo clippy -p argo-orchestrator -p argo-gateway -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-orchestrator` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-prompt-cache-affinity-wire-mints`
- PR title: `runtime: wire prompt-cache affinity into 4 SessionId mint sites (mission-prompt-cache-affinity-wire-mints)`

## Notes for the implementer

- The mint sites in `session.rs` are the load-bearing ones (worker sessions). The CLI and gateway sites are supplementary.
- `AgentDeps` plumbing: confirm the actual file path by `rg -l 'struct AgentDeps' crates/`. The brief assumes `argo-orchestrator/src/deps.rs` but the file may be `agent_deps.rs` or live in a different crate.
- The 4-turn smoke must run within the TTL window. Use `tokio::time::sleep(Duration::from_millis(50))` between turns, NOT actual seconds — the TTL is configurable per-test.
- If a mint site genuinely has no `worker_id` in scope (admin-tools, gateway-anonymous), it stays `SessionId::new()`. Document in the PR body which sites you treated this way and why.
- This brief is the operator-visible value of #245. Without this wire-up the #1539 primitive does nothing in production.
