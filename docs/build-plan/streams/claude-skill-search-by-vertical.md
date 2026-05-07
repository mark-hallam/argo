# claude-skill-search-by-vertical — `argo skill search --vertical <id>`

**Owner**: claude. **Depends on**: existing `argo skill search` handler
(`crates/argo-cli/src/handlers/skill_search.rs`); skill→vertical relation
already in `argo_skill::store_vertical`.

## Why

Mission claim 4 says skills are a "library workers draw from." Right now the
search surface returns the global skill set with no way to slice by vertical
context. A sales worker shouldn't see medical skills when discovering what's
available, and a vertical owner should be able to ask "what skills exist for
my domain?" without grepping the manifest tree.

The `vertical_id` column already exists on the `skill` table (see migration
v9 — `idx_skill_vertical_id`). We just expose it on the CLI search.

## Scope

- Extend `argo skill search` with `--vertical <id>` filter.
- Behavior: if the skill row has a `vertical_id` matching the flag, include it.
- Plus an "indirect" mode (`--vertical <id> --include-used-by-sessions`)
  that joins through `session_meta`/`session_skills` to surface skills that
  workers in that vertical have actually used recently. Default: off.
- Existing flags (`--limit`, `--json`, `--markdown`) keep working in combination.
- Vertical-not-found returns clear non-zero exit with a list of valid vertical IDs.

## Approach

1. Read existing `skill_search.rs` handler.
2. Add `vertical: Option<String>` and `include_used_by_sessions: bool` to its
   clap struct.
3. Add `pub async fn search_by_vertical(...)` to a new file
   `crates/argo-store/src/skill_query.rs` that takes `(vertical_id, limit,
   indirect: bool)` and returns `Vec<SkillRow>`. Direct mode is one SELECT
   with a `WHERE vertical_id = ?`. Indirect mode joins
   `skill ↔ session_skills ↔ session_meta WHERE session.vertical_id = ?
   GROUP BY skill_id ORDER BY MAX(session_started_at) DESC LIMIT ?`.
4. Handler invokes the new helper when flag is set; otherwise falls through
   to existing behavior.

## Files you may touch

- BLANKET CLI ALLOWLIST applies.
- `crates/argo-cli/src/handlers/skill_search.rs` (extend).
- `crates/argo-cli/src/cli.rs` — narrowly: add flags to `SkillSearch` clap variant.
- `crates/argo-store/src/skill_query.rs` (NEW, ≤ 250 LOC).
- `crates/argo-store/src/lib.rs` — narrowly: 1 `pub mod skill_query;` line.
- `crates/argo-cli/tests/skill_search_vertical.rs` (NEW, ≤ 200 LOC).

## Files you must NOT touch

- Any migration file — `vertical_id` column already exists.
- `crates/argo-skill/src/store_vertical.rs` — orthogonal concern.
- Marketplace search (`crates/argo-skill/src/marketplace.rs`).

## Acceptance criteria

- [ ] `argo skill search --vertical <id>` returns only matching skills.
- [ ] `--vertical <id> --include-used-by-sessions` surfaces session-used skills.
- [ ] Combines with `--limit`, `--json`, `--markdown`.
- [ ] Unknown vertical → clear non-zero exit, valid IDs listed.
- [ ] 5+ tests: direct match, indirect via sessions, empty result, limit honored, JSON shape.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-search-by-vertical`
- PR title: `cli: argo skill search --vertical filter (claude-skill-search-by-vertical)`

## Notes

- Verify column names with `grep -rn "vertical_id" crates/argo-store/migrations/`
  before writing the SQL.
- If the indirect join performs poorly, add a covering index in a follow-up;
  do NOT add an index in this PR (keeps this brief schema-clean).
