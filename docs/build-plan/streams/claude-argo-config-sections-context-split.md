# claude-argo-config-sections-context-split — split 510-LOC `sections_context.rs` into per-config siblings

**Owner**: claude. **Depends on**: nothing (pure refactor).
**Authored against**: origin/main `58fa6e29` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-argo-config-sections-context-split`.

## Scope

Pure refactor. `crates/argo-config/src/sections_context.rs` is at **510 LOC** — already OVER the 500-LOC hard ceiling (cluster-5 audit finding). Currently holds 7 config structs in one file: `ContextCompressConfig`, `ActionThrottleConfig`, `ActionThrottlePolicyConfig`, `SandboxPrewarmConfig`, `McpPoolConfig`, `PromptCacheAffinityConfig`, `WorkerResourceConfig`, plus the `RuntimeConfig` aggregator.

This blocks `mission-multi-vertical-concurrency-lock` (worker hit ceiling-refusal today) and ANY brief that needs to add a new field to `RuntimeConfig`.

Per the existing `sections_<topic>.rs` sibling pattern in argo-config (e.g. `sections_security.rs`, `sections_telemetry.rs`), extract each non-aggregator config into its own sibling file. `RuntimeConfig` stays in `sections_context.rs` as the aggregator + re-export façade.

## Approach

For each of these 6 configs, move to its own `sections_<topic>.rs` sibling:
- `ContextCompressConfig` → `sections_context_compress.rs` (~110 LOC)
- `ActionThrottleConfig` + `ActionThrottlePolicyConfig` + `DEFAULT_ACTION_THROTTLE_AUDIT_SECS` → `sections_action_throttle.rs` (~80 LOC)
- `SandboxPrewarmConfig` → `sections_sandbox_prewarm.rs` (~60 LOC)
- `McpPoolConfig` + `DEFAULT_MCP_POOL_*` consts → `sections_mcp_pool.rs` (~60 LOC)
- `PromptCacheAffinityConfig` → `sections_prompt_cache.rs` (~50 LOC)
- `WorkerResourceConfig` + `DEFAULT_WORKER_RESOURCE_*` consts → `sections_worker_resource.rs` (~70 LOC)

`sections_context.rs` keeps `RuntimeConfig` + its `Default` impl + the parent `[runtime]` doc + `pub use` re-exports of the 6 children. Target: ~150 LOC.

`crates/argo-config/src/lib.rs` re-export block stays unchanged (consumers `use argo_config::McpPoolConfig` etc. continue to work).

## Files you may touch

- `crates/argo-config/src/sections_context.rs` — slim from 510 → ~150 LOC.
- 6 NEW `sections_*.rs` siblings as above.
- `crates/argo-config/src/lib.rs` — narrowly: ONE block of `pub mod sections_<topic>;` declarations + the existing `pub use` re-exports stay. ≤10 modified lines.

## Files you must NOT touch

- `RuntimeConfig` shape (fields, defaults, derive macros) — locked.
- Each config struct's shape — moves verbatim.
- Any consumer outside `crates/argo-config/` — re-export-preserving means zero breakage.

## Acceptance

- [ ] `sections_context.rs` ≤ 200 LOC (target ~150).
- [ ] 6 sibling files each ≤ 150 LOC.
- [ ] All consumers (`use argo_config::McpPoolConfig` etc.) still compile.
- [ ] Workspace clippy clean.
- [ ] Zero behavior change.

## Branch
- `claude-argo-config-sections-context-split`

## Notes
- `-split` suffix → ceiling check auto-skips per `scripts/check-pre-claim-ceiling.sh:45`.
- Mechanical. Follow the existing `sections_security.rs` / `sections_telemetry.rs` pattern.
- Unblocks `mission-multi-vertical-concurrency-lock` + future briefs that add RuntimeConfig fields.
