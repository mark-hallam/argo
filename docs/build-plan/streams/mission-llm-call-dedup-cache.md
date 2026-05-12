# Mission load-bearing wire — in-process LLM call deduplication cache

**Owner**: claude. **Depends on**: `argo-provider::transport` SHIPPED (verified at `crates/argo-provider/src/transport.rs`).
**Authored against**: origin/main `f83b9836` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-llm-call-dedup-cache`.

> SOURCE: MISSION_BACKLOG.md Section 9 #253 ("LLM call deduplication"). Within a single agent turn, multiple internal call sites (auto_approval policy check + agent dispatch + safety judge) may issue **identical** completion requests (same model, same messages, same params). Today each fires independently — burning input tokens 2-3× over for the same content. Adding an in-process dedup cache with a 60s TTL collapses the redundant calls without changing any caller's interface.

## Scope

Add a `DedupCache` wrapper around the existing `Transport` trait (defined at `crates/argo-provider/src/transport.rs:147`; mirror of the existing `BreakerTransport<T: Transport>` pattern at line 216). NOTE: earlier draft of this brief incorrectly referenced a `Provider` trait — the actual workspace trait is `Transport`; do a careful s/Provider/Transport/ at claim time on this brief.

1. Hashes the request key: `sha256(model + json(messages) + json(params))`.
2. Looks up the key in an in-process `Arc<RwLock<HashMap<[u8;32], (Instant, Response)>>>`.
3. On hit + entry age < TTL (default 60s): returns the cached `Response` clone, increments `cache_hit` counter.
4. On miss: forwards to inner provider, stores result with timestamp, returns it. Increments `cache_miss` counter.
5. Periodic GC (every 5 min via tokio interval) drops entries older than TTL.

```rust
pub struct DedupCache<P: Transport> {
    inner: Arc<P>,
    cache: Arc<RwLock<HashMap<[u8; 32], CachedEntry>>>,
    ttl: Duration,
}

struct CachedEntry {
    response: Response,
    at: Instant,
}

impl<P: Transport> DedupCache<P> {
    pub fn new(inner: P, ttl: Duration) -> Self;
}

impl<P: Transport> Transport for DedupCache<P> { /* delegates */ }
```

Wired via NEW `Transport::with_dedup(ttl)` builder helper. Opt-in: not on by default. Operators enable via `ARGO_PROVIDER_DEDUP_TTL_SECS=60` (env-var name stays `PROVIDER` because the **crate** is `argo-provider` even though the **trait** is `Transport`).

This is NOT in scope: cross-process cache (in-process only — workers in different processes have separate caches); persistent cache (TTL is process-lifetime bounded); hashing tool_use blocks (only flat messages cache cleanly); LRU eviction (TTL-only); streaming responses (skip the cache layer for streaming calls).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-provider/src/dedup.rs` (NEW, ≤ 300 LOC) — `DedupCache`, `CachedEntry`, `Provider` trait delegation, GC loop spawn.
- `crates/argo-provider/src/dedup_tests.rs` (NEW, sibling) — 8 unit tests: hit-returns-cached; miss-forwards; TTL-expiry; identical-params-same-key; differing-params-different-keys; streaming-skipped; gc-drops-old; concurrent-readers-no-block.
- `crates/argo-provider/src/lib.rs` — narrowly scoped: ONE `pub mod dedup;` line + re-export `DedupCache`.
- `crates/argo-provider/src/transport.rs` — narrowly scoped: ADD `with_dedup(ttl)` trait extension method. NO change to existing `Provider` trait signature.
- `crates/argo-telemetry/src/metrics_provider_dedup.rs` (NEW, ≤ 100 LOC) — `cache_hit_total`/`cache_miss_total`/`cache_hit_rate` counters/gauges. Verify metrics integration pattern matches the rest of `argo-telemetry`.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: read `ARGO_PROVIDER_DEDUP_TTL_SECS` env at boot, wrap the constructed provider with `DedupCache` when set, leave default-construction path identical when unset.

## Files you must NOT touch

- `crates/argo-provider/src/circuit.rs` — circuit breaker is upstream; orthogonal
- The `Provider` trait surface itself — wrap, don't modify
- Any non-provider/non-telemetry/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (in-process cache)

## Acceptance criteria

- [ ] `DedupCache<P>` wraps any `Provider` impl, key = sha256(model+messages+params)
- [ ] TTL default 60s, configurable via `ARGO_PROVIDER_DEDUP_TTL_SECS` env
- [ ] Identical calls within TTL return cached `Response`
- [ ] Calls past TTL refetch
- [ ] Streaming calls skip the cache entirely (return inner directly)
- [ ] GC loop runs every 5min, drops expired entries
- [ ] Telemetry counters: `argo_provider_cache_hit_total{model}`, `argo_provider_cache_miss_total{model}`
- [ ] `cache_hit_rate` gauge updated per request
- [ ] Wired from gateway boot when env var set; default-path unchanged when unset
- [ ] 8 unit tests + 1 integration smoke
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-provider -p argo-telemetry -p argo-cli` clean
- [ ] `cargo clippy -p argo-provider -p argo-telemetry -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-provider -p argo-telemetry -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-llm-call-dedup-cache`
- PR title: `provider: in-process LLM call dedup cache (mission-llm-call-dedup-cache)`

## Notes for the implementer

- The 60s TTL is the brief's default — tuned to "single agent turn" duration. Operators with longer-lived sessions can extend via env var.
- Hash collision risk: sha256 collision is negligible. Don't add a fallback path; if two calls hash to the same key, they ARE identical by definition.
- Streaming calls aren't cached because the response is consumed once and can't be replayed cheaply. Real streaming dedup needs SSE replay buffers — separate brief.
- The `with_dedup` builder is opt-in by design. Wrapping every provider unconditionally would surprise operators who rely on "every call is fresh" semantics for ephemeral content.
- GC every 5min is sufficient because TTL is 60s — at most you accumulate 5min × MAX(calls/sec) entries between sweeps. Operators can tune `ARGO_PROVIDER_DEDUP_GC_SECS` if they want tighter.
