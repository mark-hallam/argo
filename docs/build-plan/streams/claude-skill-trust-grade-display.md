# claude-skill-trust-grade-display — surface `TrustGrade` in CLI output

**Owner**: claude. **Depends on**: existing `TrustGrade` enum
(`crates/argo-skill/src/trust.rs`); existing `argo skill list` and
`argo skill view`/`show` handlers.

## Why

`TrustGrade` (Builtin / PublishedVerified / Tenant / PublishedCommunity /
LearnedUnreviewed) is the single most important attribute a reviewer needs
to see when triaging skills, and it's currently invisible in the CLI list
and show outputs. That's a UX gap on the very signal mission claim 4 says
must be visible.

## Scope

- `argo skill list` markdown table gains a `Trust` column.
- `argo skill list --json` output objects gain a `trust_grade` field
  (string form: `published_verified`, `learned_unreviewed`, etc.).
- `argo skill show <id>` (or `view`, whichever is the canonical handler)
  surfaces `Trust grade: <value>` on its own line in markdown form, plus
  field in JSON.
- Sortable: `argo skill list --sort trust` orders by descending trust grade.

## Approach

1. Read existing `skill_list.rs` and `skill_view.rs` (or `skill_show.rs`)
   handlers — confirm canonical path. Note: codebase has `skill_view.rs`,
   not `skill_show.rs`.
2. Resolve trust grade per row using `TrustGrade::from_source(...)` (pattern
   already established in `version.rs`). If a stored grade column exists on
   the skill row, prefer that; otherwise compute from source.
3. Add a helper `fn format_trust(g: TrustGrade) -> &'static str` returning
   the snake_case form (matches `parse_trust_grade` round-trip).
4. Update markdown table renderer to include the column. Add column to JSON
   serialization. Plumb `--sort trust` through the existing sort dispatch.

## Files you may touch

- BLANKET CLI ALLOWLIST applies.
- `crates/argo-cli/src/handlers/skill_list.rs` (extend).
- `crates/argo-cli/src/handlers/skill_view.rs` (extend).
- `crates/argo-cli/src/cli.rs` — narrowly: add `--sort trust` value if not present.
- `crates/argo-cli/tests/skill_trust_display.rs` (NEW, ≤ 200 LOC).

## Files you must NOT touch

- `crates/argo-skill/src/trust.rs` — read-only consumer; do NOT add new variants.
- Migration files — purely a display change.
- Other handlers (skill_publish, skill_promote, etc.).

## Acceptance criteria

- [ ] `argo skill list` markdown shows a `Trust` column.
- [ ] `argo skill list --json` includes `trust_grade` field (snake_case).
- [ ] `argo skill view <id>` markdown shows `Trust grade: <value>`.
- [ ] `argo skill list --sort trust` orders descending (most-trusted first).
- [ ] 5+ tests: markdown column, JSON field, view markdown, sort, unknown grade fallback.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-trust-grade-display`
- PR title: `cli: surface TrustGrade in skill list/view (claude-skill-trust-grade-display)`

## Notes

- Trust ordering is the natural enum order (Builtin = highest, LearnedUnreviewed
  = lowest), already encoded by `Ord` on the enum — verify by reading
  `marketplace.rs` which already relies on it (`row.trust_grade <= TrustGrade::PublishedCommunity`).
- Snake_case form must match `parse_trust_grade` — check that file before writing the formatter.
