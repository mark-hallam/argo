# Mission load-bearing wire — prompt cache hit-rate observability

**Owner**: claude. **Depends on**: provider response parsing in `argo-provider` (verify surface in first 5min); `argo-telemetry` metrics registry.
**Authored against**: origin/main `6b6b69a5` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-prompt-cache-hit-rate-observability`.

> SOURCE: MISSION_BACKLOG.md Section 8 #246 ("Prompt cache hit rate observability"). Anthropic responses include `cache_creation_input_tokens` and `cache_read_input_tokens` usage fields. We currently log them per request (verified in the Claude Code worker logs) but don't expose **fleet-wide hit-rate metrics**. Operators tuning prompt structure can't see whether their changes improved cache hit rate without re-parsing raw logs.

## Scope

1. **First 5 min**: locate where Anthropic API responses are parsed for token usage. Most likely in `argo-provider` per-provider response handler. Search: `rg -n "cache_creation_input_tokens\|cache_read_input_tokens" crates/argo-provider/`. If absent, search worker harness response handling in `argo-runtime` or `argo-cli`. **If neither location parses these fields**, file a BLOCKED note with the actual response-parsing location — DO NOT add new parsing without operator approval.
2. **Add metric counters** via existing `argo-telemetry::default_registry`:
   - `argo_provider_cache_creation_input_tokens_total{provider, model}` (counter)
   - `argo_provider_cache_read_input_tokens_total{provider, model}` (counter)
   - `argo_provider_uncached_input_tokens_total{provider, model}` (counter — total input tokens minus cache_creation minus cache_read)
3. **Compute hit rate** as a derived gauge: `argo_provider_cache_hit_rate{provider, model} = cache_read / (cache_read + cache_creation + uncached)`. Updated on every response.
4. CLI: `argo provider cache-stats [--provider <name>] [--model <name>]` prints the counters + hit rate. JSON output for `--json`.

Per-provider/model labels are essential — Anthropic cache semantics differ from OpenAI (different cache mechanisms), so single global hit-rate is misleading.

This is NOT in scope: changing cache invalidation logic; per-tenant cache stats (label cardinality concern); cache cost calculation (separate brief if needed); cross-process aggregation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-provider/src/<wherever-anthropic-response-is-parsed>.rs` — narrowly scoped: add 3 counter increments + 1 gauge update per response. Verify exact file in first 5min via `rg -l "input_tokens\|cache_creation" crates/argo-provider/`. NO change to response struct shape.
- `crates/argo-telemetry/src/metrics_prompt_cache.rs` (NEW, ≤ 150 LOC) — `pub fn register_prompt_cache_metrics(registry: &Registry)` + the 4 metric handles. Reuse existing metrics-registration patterns from sibling files.
- `crates/argo-telemetry/src/lib.rs` — narrowly scoped: ONE `pub mod metrics_prompt_cache;` line + re-export `register_prompt_cache_metrics`.
- `crates/argo-cli/src/handlers/provider_cache_stats.rs` (NEW, ≤ 120 LOC) — `argo provider cache-stats` subcommand handler.
- `crates/argo-cli/src/cli_provider_cmds.rs` (or wherever provider subcommands live — verify in first 5min) — narrowly scoped: ADD `cache-stats` arm.

## Files you must NOT touch

- `crates/argo-telemetry/src/tracing_sink.rs` — OTLP exporter is upstream; reuse only
- `crates/argo-provider/src/circuit.rs` — circuit breaker is upstream
- Any non-provider/non-telemetry/non-cli crate
- `crates/argo-store/migrations/**` — no schema change (metrics are in-process)

## Acceptance criteria

- [ ] 3 counters registered: `argo_provider_cache_{creation,read,uncached}_input_tokens_total`
- [ ] 1 gauge registered: `argo_provider_cache_hit_rate`
- [ ] All metrics labeled by provider + model
- [ ] Response-parsing path increments counters on every Anthropic response
- [ ] Hit rate updated on every response (cache_read / total)
- [ ] `argo provider cache-stats` CLI works; `--provider` + `--model` filter; `--json` output
- [ ] 4 unit tests: counter-increments-correctly; uncached-computed-from-total; hit-rate-zero-when-no-cache; hit-rate-handles-divide-by-zero
- [ ] 1 integration smoke: send 3 mock responses, assert metrics scrape correctly
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-provider -p argo-telemetry -p argo-cli` clean
- [ ] `cargo clippy -p argo-provider -p argo-telemetry -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-provider -p argo-telemetry -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-prompt-cache-hit-rate-observability`
- PR title: `provider: prompt cache hit-rate metrics (mission-prompt-cache-hit-rate-observability)`

## Notes for the implementer

- The "investigate first" step is load-bearing — if Anthropic response token usage isn't parsed today, this brief expands scope significantly. File a BLOCKED with the actual missing surface so the conductor can scope a precursor brief.
- Per-provider/model label cardinality is low (10-20 distinct combos in normal usage) — safe for Prometheus.
- The uncached counter is computed at increment time: `uncached = response.usage.input_tokens - response.usage.cache_creation_input_tokens - response.usage.cache_read_input_tokens`. Guard against negative values (shouldn't happen but defense-in-depth).
- Hit rate as a gauge (vs. derived at query time) avoids client-side math but loses cardinality on Prometheus rate() functions. Both expose the counters AND the gauge so operators can choose.
- Don't try to compute cost savings here. That's downstream and depends on per-model token pricing which changes; keep this brief about raw counters.
