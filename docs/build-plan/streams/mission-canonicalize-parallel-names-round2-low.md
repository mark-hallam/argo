# mission-canonicalize-parallel-names-round2-low — Round-2 LOW-severity duplicate types (10 types)

**Owner**: claude. **Depends on**: claude-canonicalize-parallel-names-round2-high MERGED (recommended — handles the cascading HIGH-sev types first).
**Authored against**: origin/main `bfe849f4` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-canonicalize-parallel-names-round2-low`.

## Scope

Phase D audit (`docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` §2) flagged 19 NEW parallel-name surfaces beyond Phase A's 11. The 7 HIGH-severity ones go via `claude-canonicalize-parallel-names-round2-high` (CLAUDE-PREFERRED). This brief covers the **10 LOW-severity** ones — same-name types across 2 crates with low semantic-reuse risk.

**Types to triage** (all 2-way duplicates unless noted):
1. `TickReport` — argo-orchestrator/duty_scheduler.rs:117 + argo-cron/scheduler.rs:36 (likely different concepts: orchestrator's vs cron's)
2. `SuggestedAction` — argo-genesis/eval_seed.rs:40 + argo-genesis/action_suggest.rs:7 (same crate, different files — likely same concept, just split)
3. `SendMessageRequest` — argo-acp/server.rs:13 + argo-bluebubbles-client/lib.rs:64 (likely different — argo-acp removed via #1657? Verify)
4. `SandboxPrewarmConfig` — argo-config/sections_sandbox_prewarm.rs:33 + argo-executor-computer-use/sandbox_prewarm.rs:70 (config struct duplicate)
5. `RoutingContext` — argo-orchestrator/routing.rs:187 + argo-routing/router.rs:11 (likely different concepts)
6. `RetryPolicy` — argo-vertical/manifest_actions.rs:170 + argo-mcp-connector-shared/retry.rs:36
7. `QuietHours` — argo-channels/quiet_hours.rs:32 + argo-identity/policy.rs:143 (likely same concept; canonicalize to argo-types)
8. `PoolKey` — argo-mcp/pool.rs:80 + argo-executor-computer-use/pool.rs:26 (likely different per-pool keys)
9. `PoolConfig` — argo-store/pool_config.rs:43 + argo-mcp/pool.rs:58 (likely different — store vs MCP pool config)
10. `Edge` — argo-workflow/dag.rs:88 + argo-skill/ir/types.rs:61 (likely different — DAG edge vs skill IR edge)
11. `Compliance{Soc2Args,ExportArgs,DeleteArgs}` — argo-compliance/{soc2,export,delete}.rs + argo-cli/cli*.rs (intentional CLI duplicates? Verify)

Per-type triage strategy (same as round-1 medium-duplicates brief): read both definitions, determine same-concept vs different-concept, rename or consolidate.

LOC: ~800.

## Files you may touch

For each type cited above + its call sites surfaced by `rg '<TypeName>'`. Tests + Cargo.toml dep edges as needed.

## Files you must NOT touch

- The 7 HIGH-severity round-2 types (handled by separate brief).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] For each of the 11 surface clusters: ONE canonical home OR clearly-named distinct types.
- [ ] `rg 'pub (struct|enum) (TickReport|SuggestedAction|...etc)\b' crates/` returns minimum-necessary matches.
- [ ] PR description documents per-type decision.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-mission-canonicalize-parallel-names-round2-low`
- PR title: `refactor: canonicalize 10 LOW-severity round-2 duplicate types (mission-canonicalize-parallel-names-round2-low)`

## Notes for the implementer

- Mission-claim alignment: production hardening (type clarity).
- Post-v1 polish; not ship-blocking.
- Many of these are likely "different concepts, just confusingly same-named". Rename rather than force-merge.
- Drift-prevention: brief removes parallel surfaces; adds zero new ones.
