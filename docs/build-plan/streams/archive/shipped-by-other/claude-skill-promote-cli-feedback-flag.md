# Stream B — feedback-confirmed learned skill promotion

**Owner**: codex.
**Depends on**: `claude-operator-feedback-channel` MERGED and `claude-dreams-skill-distillation` MERGED.
**Authored against**: origin/main `69b929c8` on `2026-05-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-skill-promote-cli-feedback-flag`.
**MVP lane**: supports mission claim 4's learned-skill approval loop.

## Scope

The original brief targeted a nonexistent `argo skill promote` command and
`crates/argo-cli/src/handlers/skill_promote.rs`. Current main's real learned
skill promotion surface is `argo skill review promote <memory_entry_id>`,
implemented in `crates/argo-cli/src/handlers/skill_review.rs` and
`skill_review_candidates.rs`.

Add an optional feedback confirmation flag to that real command:
`argo skill review promote <memory_entry_id> --feedback-action <action_id>`.
When the flag is present, the handler queries the existing
`operator_feedback` table for the latest vote on that action. An `up` vote
allows promotion and is echoed in text/JSON output; a `down` vote or missing
vote blocks promotion with a clear non-zero error. When the flag is omitted,
existing promotion behavior is unchanged.

## Files you may touch

- `crates/argo-cli/src/cli_skill_review_cmds.rs` — add `--feedback-action <action_id>` to `SkillReviewCmd::Promote`.
- `crates/argo-cli/src/handlers/skill_review.rs` — thread/display feedback confirmation and preserve default behavior.
- `crates/argo-cli/src/handlers/skill_review_candidates.rs` — narrowly: accept optional feedback metadata for audit payload/output if needed.
- `crates/argo-cli/src/handlers/skill_review_candidates_tests.rs` — extend candidate promotion tests.
- `crates/argo-cli/tests/skill_review_feedback_smoke.rs` (NEW) — smoke help/default/block/allow behavior.
- `crates/argo-store/src/feedback.rs` — narrowly: reuse `latest_vote_for_action`; add only a small helper if the existing function is insufficient.
- `crates/argo-store/migrations/24__operator_feedback.sql` — read-only via `include_str!` in tests only; do not alter SQL content.

## Files you must NOT touch

- `crates/argo-cli/src/handlers/skill_promote.rs` — this file does not exist on current main.
- `crates/argo-cli/src/cli_subcommands_skill.rs` — `SkillCmd::Review` already delegates to `cli_skill_review_cmds.rs`.
- Any migration file content — feedback storage already exists.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] `argo skill review promote --help` shows `--feedback-action <ACTION_ID>`.
- [ ] With no feedback flag, memory-backed promotion behavior is unchanged.
- [ ] With `--feedback-action <id>` and latest vote `up`, promotion succeeds.
- [ ] With latest vote `down`, promotion is blocked with a clear error and no skill row is created.
- [ ] With no feedback row for the supplied action id, promotion is blocked with a clear error and no skill row is created.
- [ ] JSON output includes a `feedback` object when `--feedback-action` is used.
- [ ] The `skill.candidate.promoted` audit payload records the feedback action id and vote when present.
- [ ] `cargo clippy -p argo-cli -p argo-store --all-targets -- -D warnings` passes.
- [ ] `cargo test -p argo-cli -p argo-store` passes.
- [ ] Every modified file stays under 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-claude-skill-promote-cli-feedback-flag`
- PR title: `skill: require positive operator feedback for learned promotion (claude-skill-promote-cli-feedback-flag)`

## Notes for the implementer

- Keep this as an opt-in safety gate. Do not make feedback mandatory for all promotions yet; operators still need to promote old candidates without historical feedback.
- Use `argo_store::feedback::latest_vote_for_action`; the table is action-scoped, not skill-scoped.
- If you add a helper in `argo-store`, keep it tiny and covered by unit tests. The CLI should not duplicate vote parsing.
