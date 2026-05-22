# V1.4 — Recovery cache CLI

**Owner**: codex. **Depends on**: claude-error-recovery-suggestions
(#901) merged.

## Scope

#901 added `argo recover <session-id>` — LLM-assisted post-mortem
that caches suggestions per `(session_id, chain_hash)` in the new
`recovery_cache` table (migration 28). Missing: operator-facing
CLI to inspect, replay, or invalidate that cache.

Add:
- `argo recover cache list [--worker <id>] [--since <duration>]
  [--limit N] [--json]` — recent recovery sessions newest-first
  with their cache hit/miss state and timestamp.
- `argo recover cache show <session-id> [--json]` — print the
  cached suggestions (the same JSON `argo recover` would emit on
  cache hit) without re-calling the LLM.
- `argo recover cache clear <session-id> [--all] [--older-than
  <duration>] [--json]` — invalidate one or many cache entries.
  `--all` requires a typed confirmation. `--older-than 7d` evicts
  entries older than the given window.

Pure CRUD over the existing `recovery_cache` table. No new tables,
no new dependencies, no LLM calls.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/recover_cache_list.rs` (NEW)
- `crates/argo-cli/src/handlers/recover_cache_show.rs` (NEW)
- `crates/argo-cli/src/handlers/recover_cache_clear.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_recover_cmds.rs` (NEW or extend
  existing recover subcommand enum if cleaner)
- `crates/argo-store/src/recovery_cache.rs` — narrowly: add
  list/show/clear helpers if the table CRUD isn't already
  there (read-side store helpers only; #901 wrote the table
  but may not have full CRUD exposed)
- `crates/argo-store/src/lib.rs` — narrowly: re-export new helpers
- `crates/argo-cli/tests/recover_cache_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo recover cache list --limit 10` returns 10 newest entries
- [ ] `argo recover cache show <id>` prints cached suggestions JSON
      without invoking the LLM (verifiable: works with
      `ANTHROPIC_API_KEY` unset)
- [ ] `argo recover cache clear <id>` removes a single entry
- [ ] `argo recover cache clear --all` requires typed
      "yes-i-mean-it" confirmation
- [ ] `argo recover cache clear --older-than 7d` evicts old entries
      (verified via integration test that seeds entries with
      back-dated timestamps then asserts only old ones are gone)
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 5 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-recover-cache-cli`
- PR title: `recover: argo recover cache list/show/clear (codex-recover-cache-cli)`
