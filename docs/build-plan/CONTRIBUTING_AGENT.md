# Agent coordination rules

How multiple AI agent families (Claude Code, Codex, future
agents) build this repo concurrently without stepping on each
other. Read this before starting any task.

`CLAUDE.md` is the live swarm-worker rule source for this repo,
including newer amendments for owner-as-sizing-hint,
`claim-with-retry.sh`, freshness checks, crate-scoped verification,
and worker rotation. If this companion file conflicts with
`CLAUDE.md`, follow `CLAUDE.md`.

---

## The protocol

1. **Sync main first.** Always start with `git checkout main &&
   git pull --ff-only origin main`.
2. **Read [`BUILD_STATUS.md`](BUILD_STATUS.md).** Find the first
   row whose `status` is `TODO` and whose predecessors are merged.
   The `owner` column is a sizing hint, not a hard lock.
   Codex-sized briefs are normally narrower and more mechanical;
   Claude-sized briefs may be broader.
3. **Claim the brief.** Edit `BUILD_STATUS.md` to set that
   row's `status` to `CLAIMED`. Commit + push to main with
   message `status: claim <brief-id>`. If push fails (someone
   else got there first), pull main and pick a different brief.
4. **Read the brief** at `streams/<brief-id>.md`. Follow it
   exactly.
5. **Build, test, lint** before opening the PR. Prefer the
   crate-scoped verification flow in `CLAUDE.md`, then run the final
   workspace clippy sanity check before pushing.
6. **Open the PR** with the brief's specified title + body.
7. **Mark in review.** Edit `BUILD_STATUS.md` to set the row's
   `status` to `IN_REVIEW`, add the PR number to the row,
   commit + push to main with message `status: <brief-id> in
   review (#<N>)`.
8. **Loop.** Pick the next `TODO` row and start over.

---

## File boundaries

Each brief specifies:

- **Files you may touch** — implementation files for this
  stream
- **Files you must NOT touch** — anything outside the stream's
  scope, including: other streams' implementation files, the
  `Executor` trait (touched only by Stream A), shared
  workspace `Cargo.toml` deps (any new dep must be added in
  the brief's "may touch" list explicitly)

If your task genuinely requires touching a "must not" file,
**stop and surface it in BUILD_STATUS.md** with a brief
description of what change is needed and why. Do not silently
expand scope.

---

## Branch + PR conventions

- Branch name format: `<owner>-<brief-id>` (e.g.
  `claude-A-executor-trait`, `codex-C-gmail`)
- PR title format: specified per-brief; usually
  `<area>: <one-line summary> (<brief-id>)`
- PR body must:
  - Reference the brief path
  - List acceptance criteria with checkboxes (paste from brief)
  - Note any deviations + reasons
- Commit messages: present-tense imperative, < 72 chars title,
  body wrapped at 72
- Co-Authored-By trailer for the agent that authored the work:
  - Claude: `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>`
  - Codex: `Co-Authored-By: Codex <noreply@openai.com>`

---

## Gates before opening any PR

Every brief lists this as the acceptance checklist; never skip:

- [ ] crate-scoped `cargo clippy -p <crate> --all-targets -- -D warnings`
      clean for every touched crate
- [ ] crate-scoped `cargo test -p <crate>` passes for every touched crate
- [ ] final `cargo clippy --workspace --all-targets -- -D warnings`
      sanity check is clean, or any failure is a documented
      pre-existing boundary issue under `CLAUDE.md` HARD RULE 2
- [ ] Every file you authored or modified is < 500 LOC
- [ ] Files-touched list matches the brief's "may touch"
- [ ] BUILD_STATUS.md updated with PR number

If clippy warns and you cannot resolve in 3 attempts, mark the
brief `BLOCKED` in BUILD_STATUS.md with a one-line reason and
pick a different brief.

---

## Merge policy

- **All PRs reviewed before merge.** Reviewer is the human
  operator OR a Claude reviewing-agent designated for that
  stream. The brief specifies who.
- **Squash-merge.** Merge commit body should preserve the PR
  title.
- **Delete branch on merge.**
- **Pull main immediately after your PR merges.**

---

## What to do when stuck

1. **Build fails or test fails:** debug for ≤30 min. If
   unresolved, mark brief `BLOCKED` in BUILD_STATUS.md, commit,
   continue with the next brief.
2. **Acceptance criteria conflict with each other:** mark brief
   `BLOCKED`, surface the conflict in BUILD_STATUS.md, ask the
   human operator to resolve.
3. **Mission disagreement:** if a brief contradicts
   [`docs/skycot/MISSION.md`](../skycot/MISSION.md), MISSION.md
   wins. Mark brief `BLOCKED`, note the disagreement.
4. **Git push fails because main moved:** `git pull --rebase
   origin main`, retry. Repeat once. If it fails twice, it
   means another agent is also pushing — back off 60 seconds,
   retry.

---

## Rules that override everything

1. **Never force-push to main.**
2. **Never modify files outside your brief's "may touch" list.**
3. **Never modify another agent's branch.**
4. **Never delete another agent's PR or branch.**
5. **Never override clippy warnings with `#[allow(...)]` unless
   the brief explicitly authorizes it.**
6. **Never skip the acceptance checklist.**

Violations are bugs. Each agent's PR review will check.

---

## Status values for BUILD_STATUS.md

- `TODO` — available to claim
- `CLAIMED` — an agent has started; do not pick up
- `IN_REVIEW` — PR open, awaiting review
- `MERGED` — done, on main
- `BLOCKED` — agent hit something it cannot resolve; needs
  human or different agent to unblock

Always check the latest BUILD_STATUS.md from origin/main, not a
stale local copy. Pull before claiming; pull before marking in
review.
