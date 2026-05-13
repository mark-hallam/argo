# mission-llm-call-dedup-cache-v2 — in-process LLM dedup wired at the real transport-builder site

**Owner**: claude. **Depends on**: existing `Transport` trait at `argo-provider/src/transport.rs`; existing transport-builder at `argo-agent-runner/src/transport.rs::build_transport`.
**Authored against**: origin/main `6676b391` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-llm-call-dedup-cache-v2`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized Cluster D #4).** V1 targeted gateway boot for the wire-up; gateway boot does NOT construct provider transports. The real construction is `argo-agent-runner/src/transport.rs::build_transport`. V2 widens may-touch to include it.

> Note on complementarity with the Anthropic prompt-cache: #1539 + #1543 ship session-id affinity for the **provider-side** prompt cache (cache_control hints reduce per-call cost). This brief is **in-process** dedup (eliminates the call entirely when an identical request already returned within the TTL window). Different layers; complementary.

## Scope

Add `DedupCache<P: Transport>` wrapping the existing `Transport` trait. Per-key fingerprint = `sha256(model + json(messages) + json(params))`; TTL default 60s; in-process `Arc<RwLock<HashMap>>`; per-turn benefit (within a single agent loop, auto_approval + dispatch + safety_judge often issue identical completion requests). Opt-in via `ARGO_PROVIDER_DEDUP_TTL_SECS` env var.

Wire at the real construction site: `build_transport` in `argo-agent-runner/src/transport.rs` reads the env var; when set, wraps the inner transport with `DedupCache::new(inner, ttl)`.

## Files you may touch

- `<NEW: crates/argo-provider/src/dedup_cache.rs>` (≤ 250 LOC) — `DedupCache<P>` + `CachedEntry` + `Transport` impl + 5-min GC tokio interval.
- `<NEW: crates/argo-provider/src/dedup_cache_tests.rs>` (≤ 200 LOC) — 7 unit tests: hit-within-ttl, miss-outside-ttl, gc-evicts-stale, streaming-bypasses-cache, per-tenant-isolation (key includes tenant), 2 counter-correctness (hit + miss).
- `crates/argo-provider/src/lib.rs` — narrowly: `pub mod dedup_cache;` + re-export `DedupCache`.
- `crates/argo-agent-runner/src/transport.rs` — narrowly: ≤20 modified lines. Read env var; conditionally wrap.
- `crates/argo-provider/Cargo.toml` — `sha2` dep if not already present.
- `crates/argo-agent-runner/tests/dedup_cache_smoke.rs` (NEW, ≤150 LOC) — integration: same request twice → 1 underlying transport call.

## Files you must NOT touch

- `crates/argo-provider/src/transport.rs` (the trait file) — trait is locked.
- The `Transport` trait itself — `DedupCache` implements it.
- Streaming endpoints — streaming calls bypass the cache entirely.

## Acceptance criteria

- [ ] `DedupCache::new(inner, ttl)` builds wrapper.
- [ ] Same request hash within TTL → 1 underlying call (verified via mock-transport call counter).
- [ ] Outside TTL → fresh underlying call.
- [ ] Streaming requests bypass the cache.
- [ ] GC interval drops stale entries every 5min.
- [ ] Env var `ARGO_PROVIDER_DEDUP_TTL_SECS` wires through `build_transport`.
- [ ] Per-tenant isolation: cache key includes tenant_id (if available in the Transport call context).
- [ ] `cargo clippy -p argo-provider -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] Final workspace clippy clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-llm-call-dedup-cache-v2`
- PR title: `provider: in-process DedupCache wrapper + env-var wire-up (mission-llm-call-dedup-cache-v2)`

## Notes for the implementer

- 60s TTL is conservative; tune via env. Don't make it configurable per-tenant in v1 — that's a v1.x follow-up.
- The GC interval must NOT block. Use `tokio::time::interval` + `tokio::spawn`.
- Per-tenant isolation: if the `Transport::call` signature doesn't carry a tenant id explicitly, derive it from the message context (search for `tenant_id` in the request struct). If genuinely not available, document the limitation in PR body.
- Anthropic prompt-cache hits are still useful when DedupCache misses; the two layers compose cleanly.
