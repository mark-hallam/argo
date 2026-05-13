# claude-rebase-pr-1540 — recovery brief for stalled chain-rebase on #1540 (mission-mcp-connection-pooling)

**Owner**: claude. **Depends on**: nothing (recovery refactor).
**Authored against**: origin/main `abc8c2ff` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1540`.

## Scope

Recovery brief for PR #1540 (`claude-mission-mcp-connection-pooling`). PR #1540 has been DIRTY for 2h+ since 2026-05-13T06:24Z. The original worker hasn't re-pushed despite a fix-recipe being posted on the PR at 2026-05-13T06:00Z explaining the additive `RuntimeConfig` conflict (sibling field add — take both sides) introduced by the merge of #1539.

The chain dependency matters because PR #1542 (`mission-mcp-pool-wire-dispatch`) waits behind this. The 2h+ stall isn't formally 24h+ but the operator authorized recovery in the briefing because chain-stalls compound.

**Not in scope**: changing the McpClientPool design, adding new pool features, redesigning the LRU/idle-TTL semantics. This is a mechanical chain-rebase.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- Whatever PR #1540 already touched (`gh pr diff 1540 --color never | head -200`).
- **Specifically expected conflicts** (from the posted fix-recipe):
  - `crates/argo-config/src/sections_context.rs` — take both sides; keep both `prompt_cache_affinity` AND `mcp_pool` fields on `RuntimeConfig`.
  - `crates/argo-config/src/lib.rs` — take both sides; keep `PromptCacheAffinityConfig` AND `McpPoolConfig` in the re-export list.

## Files you must NOT touch

- The `McpClientPool` implementation in `crates/argo-mcp/src/pool.rs` — locked from #1540's original design.
- `crates/argo-runtime/src/prompt_cache_affinity.rs` — locked from #1539.
- Anything outside PR #1540's existing may-touch set.

## Acceptance criteria

- [ ] `gh pr checkout 1540 && git fetch origin main && git merge origin/main`.
- [ ] Two conflicts resolved additively (take both sides on `RuntimeConfig` field + re-export).
- [ ] No new commits beyond the merge commit — substantive changes already shipped.
- [ ] `cargo clippy -p argo-mcp -p argo-config --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-mcp` passes (the 11 pool tests stay green).
- [ ] `git push --force-with-lease origin claude-mission-mcp-connection-pooling`.
- [ ] PR #1540 mergeStateStatus returns to MERGEABLE.
- [ ] PR #1542 (chained downstream) re-evaluates — likely also needs a chain-rebase after #1540 lands, which is a separate problem; this brief doesn't touch it.

## Branch + PR

- Branch: `claude-mission-mcp-connection-pooling` (existing PR branch).
- PR title: existing.

## Notes for the implementer

- ~10 min mechanical job. The fix-recipe is already on PR #1540 as a comment.
- If the conflicts have grown beyond the 2-file additive set named in the recipe, BLOCK with a note. Don't expand scope unilaterally.
- `--force-with-lease` only.
- Operator authorized recovery 2026-05-13 in conductor briefing.
