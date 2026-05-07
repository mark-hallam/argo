# claude-skill-promote-cli-feedback-flag — `argo skill promote --with-feedback`

**Owner**: claude. **Depends on**: operator-feedback channel (#929) merged;
existing `argo skill promote` handler in `crates/argo-cli/src/handlers/skill_promote.rs`.

## Why

Mission claim 4: "Skills are shared and learned … with provenance + trust."
Promotion gates today only consider quality scores. Operator feedback (Up/Down
votes from the `operator_feedback` table introduced in #929) is the closest
human-in-the-loop signal we have on whether a skill is actually pulling its
weight. Promoting blind to that feedback wastes trust.

This brief surfaces operator feedback as an explicit, opt-in promotion gate
on the existing CLI command without changing default behavior — so existing
flows keep working and reviewers can ask "what does the feedback say?" with
one flag.

## Scope

- Add `--with-feedback` boolean flag to the `argo skill promote` clap struct.
- When set, the handler queries `operator_feedback` (filtered by skill_id)
  and computes a `(up, down, ratio)` triple over a configurable window
  (default last 30 days; `--feedback-window-days <N>` to override).
- Block promotion (clear non-zero exit) when ratio is below a threshold
  (`--min-feedback-ratio <0..1>`, default 0.6) AND total votes >= 5.
  Below 5 votes → "insufficient feedback" warning, do NOT block.
- JSON output includes the `feedback` block: `{ up, down, ratio, decision }`.
- Markdown output gains a "Operator feedback" section.
- Default behavior (no flag) is unchanged.

## Approach

1. Read existing `skill_promote.rs` handler — extend the clap-derived struct
   with `with_feedback`, `feedback_window_days`, `min_feedback_ratio`.
2. Add a small helper `fn fetch_feedback_summary(store, skill_id, window_days)
   -> ArgoResult<FeedbackSummary>` querying the `operator_feedback` table
   that already exists from migration 24. Lives in the handler file (no new
   crate-level surface).
3. Gate evaluation runs AFTER quality-score check; on block, return a
   structured `ArgoError::Schema(...)` so existing JSON-error flow handles it.
4. Update help-smoke test to pin the new flags.

## Files you may touch

- BLANKET CLI ALLOWLIST applies (cli.rs, main.rs, handlers/mod.rs, tests/, Cargo.toml).
- `crates/argo-cli/src/handlers/skill_promote.rs` (extend; keep ≤ 500 LOC).
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST; cli.rs is 49 LOC post-#968)
- `crates/argo-cli/src/cli_subcommands_skill.rs` — narrowly: SkillCmd lives here post-cli-split. Add the 3 flags to the `Promote` variant.
- `crates/argo-cli/src/handlers/skill_promote.rs` — narrowly: handler
- `crates/argo-cli/tests/skill_promote_feedback.rs` (NEW, ≤ 200 LOC).

## Files you must NOT touch

- `crates/argo-skill/src/feedback.rs` — read-only consumer; no schema change.
- Any migration file — feedback table already exists.
- Other skill handlers (skill_publish, skill_review, etc.).

## Acceptance criteria

- [ ] `--with-feedback` flag present and visible in `--help`.
- [ ] Promotion blocked when `up/(up+down) < min-feedback-ratio` AND `up+down >= 5`.
- [ ] Below 5 votes → warning, NOT block.
- [ ] JSON output includes `feedback` block when flag is set.
- [ ] Markdown output gains "Operator feedback" section.
- [ ] Default (no flag) behavior unchanged — covered by existing tests.
- [ ] 4+ unit tests: blocked, allowed, insufficient votes, default-unchanged.
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-promote-cli-feedback-flag`
- PR title: `cli: argo skill promote --with-feedback gate (claude-skill-promote-cli-feedback-flag)`

## Notes

- `operator_feedback` table columns are documented in migration
  `24__operator_feedback.sql`. Use the existing `crates/argo-store/src/feedback.rs`
  query helper if available; if not, inline a single SELECT — do not invent
  a new store module.
- Threshold defaults are deliberately conservative; ops can tune.
