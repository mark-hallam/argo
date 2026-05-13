# claude-rebase-pr-1542 — recovery brief for chain-stalled #1542 (mission-mcp-pool-wire-dispatch)

**Owner**: claude. **Depends on**: #1540 MERGED (just landed via `claude-rebase-pr-1540`).
**Authored against**: origin/main `d86af369` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1542`.

## Scope

Recovery for PR #1542 (`claude-mission-mcp-pool-wire-dispatch`). The PR went DIRTY when its parent #1540 (mission-mcp-connection-pooling) merged. The downstream worker hasn't yet rebased against the new origin/main.

This is the **simplest recovery brief** in the queue: pure mechanical chain-rebase. No slot collision, no migration shenanigans — just rebase against current `main`. The original #1542 work is +1053/-53 across 14 files, all of which should rebase additively.

**Specific recovery**:

1. **Chain-rebase against current `main` (HEAD `d86af369`)** — pick up the #1540 + everything else that landed.
2. **Verify the dispatch-site changes still apply** — #1542 swaps `McpClient::connect_stdio` calls at 4 sites for `pool.acquire(...)`. The pool primitive now exists on main (via #1540); the wire-up swap should compile cleanly.
3. **Expected additive conflicts**:
   - `crates/argo-mcp/src/lib.rs` — re-export list (additive)
   - `crates/argo-mcp/src/pool.rs` — may have minor diffs if the worker also patched the pool primitive (unlikely; take origin's version if so)
   - Other typical additive sites (Cargo.lock, BUILD_STATUS.md)

**Not in scope**: changing the dispatch-site design; touching `mcp_health.rs` (cold-by-design per the original brief); revisiting the AgentDeps decision (worker correctly used `OnceLock` pattern instead).

## Files you may touch

- Whatever PR #1542 already touched (`gh pr diff 1542 --color never | head -200`).
- Resolve additive conflicts.

## Files you must NOT touch

- `crates/argo-mcp/src/pool.rs` — primitive locked from #1540; if your version diverged, take origin's.
- `crates/argo-cli/src/handlers/gateway/mcp_health.rs` — must stay cold per the brief.
- The 4 in-scope dispatch sites' DESIGN — only the swap logic.

## Acceptance criteria

- [ ] `gh pr checkout 1542 && git fetch origin main && git merge origin/main`.
- [ ] All conflicts resolved additively.
- [ ] `cargo clippy -p argo-cli -p argo-agent-runner -p argo-mcp --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-mcp` passes (11 pool tests + 3 new wire-dispatch smokes from #1542).
- [ ] `git push --force-with-lease origin claude-mission-mcp-pool-wire-dispatch`.
- [ ] PR #1542 mergeStateStatus returns to MERGEABLE.

## Branch + PR

- Branch: `claude-mission-mcp-pool-wire-dispatch` (existing PR branch).
- PR title: existing.

## Notes for the implementer

- ~10 min mechanical job. Lightest of the recovery briefs.
- `--force-with-lease` only.
- Once this lands, the mission-mcp-connection-pooling chain is fully shipped (primitive #1540 + wire-up #1542) and operators get warm MCP subprocesses across turns.
