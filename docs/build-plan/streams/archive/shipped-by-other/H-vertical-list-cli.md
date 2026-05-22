# Stream H (follow-up) — `argo vertical list` + `argo vertical show`

**Owner**: codex. **Depends on**: claude-marketplace-vertical-publish
(#821) merged.

## Scope

Operators now have `argo vertical publish/pull` (per #821). They
need to discover what verticals are installed locally + see
their structure without `find verticals/` + cat. Add:

- `argo vertical list [--tenant <id>] [--source workspace|library|both]` →
  table of installed verticals (id, version, persona, # actions,
  source, last_modified)
- `argo vertical show <id>` → full details: vertical.toml
  contents, action list, persona description, eval seed count,
  skills bundled
- `argo vertical describe <id>` → 1-paragraph human summary
  (same data as show, narrative form for screen readers / LLMs)

`--json` on every subcommand.

## Files you may touch
- `crates/argo-cli/src/cli_vertical_cmds.rs` — narrowly:
  extend with new `List`, `Show`, `Describe` variants
- `crates/argo-cli/src/handlers/vertical.rs` — extend (do NOT
  modify the existing handler functions; ADD new ones)
- `crates/argo-cli/src/handlers/vertical_list.rs` (NEW)
- `crates/argo-cli/src/handlers/vertical_show.rs` (NEW)

## Files you must NOT touch
- `crates/argo-vertical/**` — read-only consumer
- The publish/pull handlers from #821

## Acceptance criteria
- [ ] All 3 subcommands wired
- [ ] `list --source workspace` only lists verticals/<id>/
      directories
- [ ] `list --source library` (when implemented; v1.x) — for
      now, output a clear "no library configured" message
- [ ] `show <id>` includes: vertical id + version + persona +
      action count + per-action risk + per-action approval
      requirement
- [ ] `describe <id>` produces 100-300 chars of narrative
- [ ] `--json` flag on every subcommand
- [ ] Integration test using `assert_cmd` + a fixture
      vertical
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-vertical-list-cli`
- PR title: `cli: argo vertical list/show/describe for installed verticals (H-vertical-list-cli)`

## Notes
- The `last_modified` field can be the directory's mtime
  (filesystem) — no manifest field needed
- Look at existing `argo skill list` for the table shape
