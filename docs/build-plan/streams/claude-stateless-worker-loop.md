# Claude stateless worker loop

**Owner**: claude.
**Depends on**: none (uses existing `scripts/claim-with-retry.sh`).
**Authored against**: origin/main `5ea3f046` on `2026-05-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-stateless-worker-loop`.

## Scope

Add a bash wrapper `scripts/claude-worker-loop.sh` that drives a
stateless Claude Code worker loop: outer loop in bash holds the
swarm-protocol state machine (pull, claim, jitter), inner invocation
is `claude -p` in headless mode for one brief, then exits. Each
iteration spawns a fresh Claude context window — no compaction
accumulation, no degraded-session mode.

Workers running under this loop are exempt from the graceful-exit
protocol in `SWARM_PROTOCOL.md` (the "2–4 PRs is the healthy norm"
guidance). That protocol exists because long-running interactive
sessions accumulate context and degrade after ~4 multi-crate cycles.
The CLI loop spawns a fresh window per brief, so the failure mode it
addresses doesn't exist here.

**Why this is desirable.** The current model has a worker tab loop
inside a single Claude Code session. Empirical observation: ~4 PRs in,
auto-compaction summarizes earlier turns lossily, workers start to
miss their own recent PR numbers, and the auto-compaction-amnesia trap
(HARD RULE 3) becomes likely. Rotating to a fresh tab fixes it but
requires a human to spawn a new tab. This brief automates that
rotation.

**Not in scope.** A codex equivalent (codex CLI flags are out of scope
here; if a `codex -p`-style headless mode exists, a sibling brief can
add `scripts/codex-worker-loop.sh` later). Conductor automation — the
conductor stays in the desktop app under `/loop 5m` as today.
Replacing the existing interactive-tab worker model — that stays
available for cases where warm context is genuinely valuable.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/claude-worker-loop.sh` (NEW) — the wrapper. Bash outer
  loop, `claude -p` inner. Should run under 120 LOC.
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CLAUDE.md` — add a section
  documenting the loop launch command as an alternative to
  interactive desktop mode. Don't remove the desktop-mode start
  sequence; it stays available.
- `docs/build-plan/SWARM_PROTOCOL.md` — under "Graceful exit", add a
  note exempting CLI-loop workers (running under
  `claude-worker-loop.sh`) from the 2-4 PR norm.

## Files you must NOT touch

- `CLAUDE.md` — project-framing doc; this change lives in
  SWARM_PROTOCOL.md and the worker prompt.
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CODEX.md` — codex CLI is
  out of scope.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — conductor behavior is
  unchanged.
- Any Rust crate. This is a bash + docs brief.

## Acceptance criteria

- [ ] `scripts/claude-worker-loop.sh` exists, executable, under
      120 LOC.
- [ ] Script reads `BUILD_STATUS.md`, picks the first row matching
      `^\| .* \| (claude|codex) \| TODO \|`, calls
      `claim-with-retry.sh`, then invokes `claude -p` with a
      tightly-scoped one-brief prompt.
- [ ] The inner prompt instructs Claude: read SWARM_PROTOCOL.md and
      the brief, ship the PR, mark IN_REVIEW, exit. Explicitly says
      "do not loop, do not claim another brief".
- [ ] If `claude -p` exits non-zero, the script logs and continues
      to the next iteration without retrying the same brief.
- [ ] Honors `/tmp/argo-stop` sentinel (Stop Condition 5 in
      SWARM_PROTOCOL.md).
- [ ] Empty-queue back-off: sleep 120s when no claimable TODO is
      found (matches Stop Condition 1 cadence).
- [ ] Iteration jitter: `sleep $((RANDOM % 26 + 5))` between
      iterations.
- [ ] `bash -n scripts/claude-worker-loop.sh` passes (syntax check).
- [ ] `PROTOCOL_WORKER_PROMPT_CLAUDE.md` has a "CLI loop mode"
      section documenting the launch command.
- [ ] `SWARM_PROTOCOL.md` "Graceful exit" has a note exempting
      CLI-loop workers.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-stateless-worker-loop`
- PR title: `scripts: stateless Claude worker loop (claude-stateless-worker-loop)`

## Notes for the implementer

- Bash, `set -euo pipefail`, no clever tricks. The script will be
  read by humans debugging at 2am.
- The awk regex for the BUILD_STATUS.md table is the load-bearing
  parsing step. Owner column may be `claude` or `codex` (the prefix
  is a sizing hint, not a lock per HARD RULE 11). Match either.
- Header rows like
  `| **▼ PHASE A — stop bleeding credibility** | — | — | — | — |`
  must NOT match — owner column is `—`, not `claude|codex`. The
  literal-`claude|codex` constraint in the regex handles this.
- `claim-with-retry.sh` already exists and handles
  non-fast-forward races (see `claude-claim-with-retry` brief).
  Just call it; don't re-implement the retry.
- The inner `claude -p` prompt should be tight (~30 lines) — every
  iteration pays this as cold-context cost. Keep it under 3k tokens.
  Don't restate protocol rules; point at SWARM_PROTOCOL.md.
- The inner prompt should explicitly authorize Claude to mark a
  brief BLOCKED if it can't ship. That keeps the loop progressing
  past stale or wrong-headed briefs without operator intervention.
- The script does NOT call `gh pr merge`. Merging stays the
  conductor's job.
- The launch command in `PROTOCOL_WORKER_PROMPT_CLAUDE.md` should
  be a single line: `bash scripts/claude-worker-loop.sh`. Optional
  env vars (`REPO`, `FAMILY`) documented immediately below.
- The SWARM_PROTOCOL.md exemption note should be 2-3 sentences,
  pointing at the script and explaining *why* the exemption holds
  (stateless iteration → no context accumulation).
