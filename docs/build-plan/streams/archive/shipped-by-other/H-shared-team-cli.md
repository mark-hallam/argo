# Stream H (follow-up) — `argo memory shared-team` CLI

**Owner**: codex. **Depends on**: F-memory-shared-team (#819)
merged.

## Scope

The shared-team memory namespace exists (PR #819) but operators
have no CLI to inspect it. Add subcommands so a human can see
what workers in their tenant have been writing to the shared
namespace — useful for debugging "why is the team agreeing on
the wrong thing?" scenarios.

Subcommands:
- `argo memory shared-team list --tenant <id> [--limit N]` →
  table or JSON of recent shared-team entries (newest first)
  with author_worker_id + content + written_at
- `argo memory shared-team write --tenant <id> --author <worker_id> --content "..."` →
  writes a new entry (operator-side, useful for seeding
  team context manually)
- `argo memory shared-team grep --tenant <id> --pattern <regex>` →
  searches across entries (case-insensitive default)

`--json` on every subcommand.

## Files you may touch
- `crates/argo-cli/src/cli_memory_cmd.rs` — narrowly: extend
  with new `SharedTeam(...)` subcommand variants
- `crates/argo-cli/src/handlers/memory_shared_team.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE line

## Files you must NOT touch
- `crates/argo-memory/**` — read-only consumer; the
  shared_team API exists per #819
- Any other handler

## Acceptance criteria
- [ ] All 3 subcommands wired
- [ ] `--tenant <id>` required on every subcommand
- [ ] `list --limit N` returns N rows newest first
- [ ] `grep --pattern <regex>` filters correctly; invalid
      regex returns 400 with clear message
- [ ] `--json` flag on every subcommand
- [ ] Integration test using `assert_cmd` + in-memory store
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-shared-team-cli`
- PR title: `cli: argo memory shared-team list/write/grep (H-shared-team-cli)`

## Notes
- Mirror the table shape of `argo memory list` if it exists
- For `grep`, use the `regex` workspace dep (already
  available)
