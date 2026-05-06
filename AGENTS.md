# Agent instructions — read this first

This repo is built by multiple AI agent families running in
parallel: Claude Code (anthropic), Codex (openai), and any future
agents that join. Coordination is critical because we have a hard
deadline (see `docs/build-plan/BUILD_PLAN_2WK.md`).

## Read these in order

1. **[`docs/skycot/MISSION.md`](docs/skycot/MISSION.md)** — the
   six load-bearing mission claims. The "why."
2. **[`CLAUDE.md`](CLAUDE.md)** — project conventions (Rust
   workspace, 500-LOC budget, dev-safe-test workflow). The
   "how."
3. **[`docs/build-plan/BUILD_PLAN_2WK.md`](docs/build-plan/BUILD_PLAN_2WK.md)**
   — the 14-day plan. The "what."
4. **[`docs/build-plan/CONTRIBUTING_AGENT.md`](docs/build-plan/CONTRIBUTING_AGENT.md)**
   — coordination rules across agent families. The "how to not
   step on each other."
5. **[`docs/build-plan/BUILD_STATUS.md`](docs/build-plan/BUILD_STATUS.md)**
   — live ledger of which briefs are claimed, in review, or
   merged.

## Pick your next task

Open `docs/build-plan/BUILD_STATUS.md`. Find the first row whose
`owner` matches your agent family (claude / codex) and whose
`status` is `TODO`. Read the brief at
`docs/build-plan/streams/<brief-id>.md`. Follow it exactly.

If no `TODO` row matches your owner, you are done — exit cleanly.

## Three rules that override everything

1. **File boundaries are absolute.** Each brief lists files you
   may touch and files you must not touch. Never violate.
2. **Acceptance criteria are non-negotiable.** Every box in the
   brief's acceptance checklist must be ticked before opening
   the PR.
3. **`MISSION.md` wins ties.** If a brief and `MISSION.md`
   disagree, follow `MISSION.md` and surface the discrepancy in
   your PR description.
