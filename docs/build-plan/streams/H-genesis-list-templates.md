# Stream H (follow-up) — Expose Genesis archetypes via CLI

**Owner**: codex. **Depends on**: G-genesis (#790) merged.

## Scope

Genesis ships 12 archetypes (Sales SDR, Customer Support Agent,
Marketing Intern, …) but operators don't know what's available
without reading the source. Add:

- `argo genesis list-archetypes [--json]` → table of archetypes
  with name + 1-line description + sensitive-verb count + base
  action count
- `argo genesis show-archetype <name>` → full archetype detail:
  description, seed action vocabulary, default persona traits,
  recommended channels
- `argo genesis suggest --charter "..."` → runs the offline
  classifier WITHOUT writing any vertical; outputs the top-3
  archetype matches with confidence scores. Useful for
  operators previewing what Genesis would produce.

`--json` on every subcommand.

## Files you may touch
- `crates/argo-cli/src/cli_genesis_cmds.rs` — narrowly:
  extend with `ListArchetypes`, `ShowArchetype`, `Suggest`
  variants
- `crates/argo-cli/src/handlers/genesis.rs` — narrowly:
  extend with the 3 new dispatch functions; do NOT modify the
  existing `Create` handler
- `crates/argo-genesis/src/archetype.rs` — narrowly: ADD
  `pub fn list_archetypes() -> Vec<ArchetypeMeta>` and
  `pub fn show_archetype(name: &str) -> Option<ArchetypeMeta>`
  helper functions (the data is already there; expose it)

## Files you must NOT touch
- The 12-archetype data itself (existing constants stay
  as-is)
- The classifier logic
- The pipeline / writers
- The safety judge

## Acceptance criteria
- [ ] All 3 subcommands wired
- [ ] `list-archetypes` shows all 12 in a stable order (alpha
      by name) with description + counts
- [ ] `show-archetype <name>` returns 404-equivalent error
      (clear message, exit 1) when name is unknown
- [ ] `suggest --charter "..."` produces top-3 matches with
      confidence scores between 0.0 and 1.0; ties broken
      alphabetically
- [ ] `--json` flag on every subcommand
- [ ] Integration test calling each subcommand
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-genesis-list-templates`
- PR title: `cli: argo genesis list-archetypes + show-archetype + suggest (H-genesis-list-templates)`

## Notes
- This is operator-discovery polish. Without it, an
  operator setting up their first worker has to read the
  Genesis source code to know what archetypes exist.
- Mirror `argo skill list` for the table shape
