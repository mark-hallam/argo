# V2.0 — Operator feedback channel (thumbs up/down per action)

**Owner**: claude. **Depends on**: claude-cu-skill-learn-wire
(#807), claude-skill-success-metric (#813),
claude-realtime-audit-stream (#831) merged.

## Scope

Today the skill-learning loop infers "good" trajectories
heuristically (success_metric: confidence + trajectory_length
+ duration + final_state_score). But the operator has
ground-truth signal that's currently uncaptured: was this
action ACTUALLY good?

Add operator-feedback channel: per-action thumbs up/down
plus optional comment. Feeds the skill-learning loop as a
strong-positive / strong-negative training signal.

- New table `operator_feedback(action_id, vote ENUM(up,down),
  comment TEXT, given_at)`
- New CLI: `argo feedback up <action-id> [--comment]`,
  `argo feedback down <action-id> [--comment]`,
  `argo feedback list [--worker <id>] [--vote up|down]`
- TUI dashboard (#claude-tui-realtime-dashboard) gets `+`
  and `-` keys to rate the highlighted action
- Web dashboard (#claude-web-dashboard-mvp) gets thumbs
  buttons inline on each action row
- Skill-learning loop: actions with `vote=up` get bonus
  weight in skill-candidate scoring; `vote=down` actions
  blacklisted from skill-candidate emission

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- EXHAUSTIVE-MATCH amendment applies
- `crates/argo-store/migrations/<next>__operator_feedback.sql`
  (NEW)
- `crates/argo-store/src/feedback.rs` (NEW)
- `crates/argo-cli/src/handlers/feedback_*.rs` (3 NEW —
  up/down/list)
- `crates/argo-cli/src/cli_feedback_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Feedback` subcommand
- `crates/argo-skill/src/candidate.rs` — narrowly: weight
  candidate scoring by feedback votes
- `crates/argo-cli/tests/feedback_smoke.rs` (NEW)

## Acceptance criteria
- [ ] up/down votes persist with action_id
- [ ] Comment optional
- [ ] Skill candidate scoring weights feedback votes
- [ ] `argo feedback list` filters by worker + vote
- [ ] `--json` envelope universal
- [ ] 8 unit tests + 1 integration test (full vote → skill
      score change cycle)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `claude-operator-feedback-channel`
- PR title: `feedback: operator thumbs up/down feeds skill learning (claude-operator-feedback-channel)`
