# Stream H (follow-up) — Skill review CLI

**Owner**: codex. **Depends on**: H-4c (#805), H-4b (#803),
claude-cu-skill-learn-wire (#807) merged.

## Scope

The skill-learning loop now writes `SkillCandidate` rows to
the existing `pending_review` lifecycle. Operators need a
way to see + approve / reject those candidates from the CLI.

Subcommands:
- `argo skill review list [--limit N] [--vertical <id>]` →
  table or JSON of pending_review candidates
- `argo skill review show <skill_id>` → full SKILL.md body +
  trajectory + provenance + originating session
- `argo skill review approve <skill_id> [--note "..."]` →
  flips lifecycle from `pending_review` → `active` (existing
  `SkillStore::promote` does the work; this is the CLI front)
- `argo skill review reject <skill_id> [--note "..."]` →
  lifecycle → `rejected` (audit-trail-preserving — no delete)

Universal `--json` flag per CLI convention.

## Files you may touch
- `crates/argo-cli/src/cli_skill_cmds.rs` — narrowly: add
  `Review(...)` subcommand family
- `crates/argo-cli/src/handlers/skill_review.rs` — the brief
  H-auto-approval explicitly listed this as may-touch;
  EXTEND, do not replace if it already exists
- `crates/argo-cli/src/cli.rs` — narrowly: register the new
  subcommand arm
- `crates/argo-cli/src/main.rs` — narrowly: dispatch

## Files you must NOT touch
- `crates/argo-skill/**` — read-only consumer; the
  pending_review lifecycle + transition methods exist
- argo-orchestrator
- argo-channels

## Acceptance criteria
- [ ] All 4 subcommands wired + dispatch through clap
- [ ] `--json` flag on every list/show subcommand
- [ ] Filter `--vertical <id>` on list works correctly
- [ ] approve / reject persist with `note` recorded in audit
      row + skill row's `reviewed_at` / `reviewed_by` (if
      schema supports — codex checks)
- [ ] Integration test using `assert_cmd` + an in-memory
      store with seeded pending_review skills
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-skill-review-cli`
- PR title: `cli: argo skill review list/show/approve/reject (H-skill-review-cli)`

## Notes
- This is the OPERATOR-FACING half of the skill-learning
  loop. Without this, a worker can propose skills but nobody
  can act on the proposals.
- Mirror the table output shape of `argo worker actions list`.
