# Codex pre-push clippy gate

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `886d9a6b` on `2026-05-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-pre-push-clippy-gate`.

## Scope

Add a blocking `cargo clippy --workspace -- -D warnings` gate that
codex worker tabs run before pushing a feature branch. Wire the gate
into the codex worker prompt and document it in `SWARM_PROTOCOL.md`
as a codex-only requirement.

**Why codex-only.** Documented failure pattern: codex tab on #905
(`codex-connector-authorize-cli`) introduced 3 clippy errors during
in-place merge-conflict resolution (compacted `if`-chains tripped
`collapsible_if`). The crate-scoped clippy ran clean *before* the
conflict resolve; the workspace check ran *after* the resolve as an
"optional final sanity check" (SWARM_PROTOCOL.md `Verification`) and
was skipped. Conductor caught the errors on review and requested
rework (ledger commit `42138c09`). Codex PRs currently average 70%
more conductor comments than claude PRs (0.29 vs 0.17 per PR); a
meaningful slice of that is "you have N clippy errors". Claude's
documented failure mode is the 500-LOC ceiling, not clippy
verification — gate is unnecessary there and would double per-PR
wall time on every claude push.

**Not in scope.** Adding the gate to claude tabs. Replacing the
existing crate-scoped clippy step (still runs first, fast feedback).
Adding `cargo test --workspace` as a gate (separate problem; tests
are slower and the documented gap is clippy-only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `scripts/codex-pre-push.sh` (NEW) — bash script implementing the
  gate. Captures `cargo clippy --workspace --all-targets -- -D
  warnings` output, fails the push if cargo exits non-zero OR if any
  `^error` lines surface in the captured output.
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CODEX.md` — add one
  start-sequence step that calls `bash scripts/codex-pre-push.sh`
  before the `git push -u origin <branch>` step.
- `docs/build-plan/SWARM_PROTOCOL.md` — under "Verification", add a
  short subsection "Codex pre-push gate" pointing at the script and
  explaining the rationale in 2–3 sentences.

## Files you must NOT touch

- `CLAUDE.md` — project-framing doc; gate is codex-only and lives in
  the codex adapter + SWARM_PROTOCOL.md.
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CLAUDE.md` — claude tabs
  do not run the gate.
- Any Rust crate. This is a bash + docs brief.
- `docs/build-plan/PROTOCOL_LOOP_PROMPT.md` — conductor behavior is
  unchanged.

## Acceptance criteria

- [ ] `scripts/codex-pre-push.sh` exists, executable (`chmod +x`),
      under 60 lines.
- [ ] Script runs `cargo clippy --workspace --all-targets -- -D
      warnings`, captures combined stdout+stderr, exits 1 if cargo
      exited non-zero OR if `grep -E '^error' "$OUT"` matches.
- [ ] Script prints a clear pass/fail line (`✅` or `🚫`) so the
      worker can read the result at a glance.
- [ ] `PROTOCOL_WORKER_PROMPT_CODEX.md` start-sequence references
      the script as a required step before `git push`.
- [ ] `SWARM_PROTOCOL.md` "Verification" section has a 2–3 sentence
      "Codex pre-push gate" subsection citing this brief id and
      pointing at the script.
- [ ] Smoke-test: run the script from a clean worktree on
      `origin/main` — should pass (or report only pre-existing
      workspace clippy state, not new errors). Note the result in
      the PR body.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `codex-pre-push-clippy-gate`
- PR title: `scripts: codex pre-push clippy gate (codex-pre-push-clippy-gate)`

## Notes for the implementer

- Keep the bash idiomatic and minimal. `set -euo pipefail`, `mktemp`
  with a trap for cleanup, `tee` so the worker still sees output
  live.
- Prefer `${PIPESTATUS[0]}` over `set -o pipefail` alone for the
  cargo exit-code check — explicit is better when the script's whole
  purpose is gating on cargo's result.
- The script should NOT modify any files (no auto-fix, no commit).
  It's a pure check.
- If the workspace clippy is already failing on `origin/main` for
  reasons unrelated to this brief, document that in the PR body
  under "Pre-existing state" but do not BLOCK the brief — the gate's
  job is to catch *new* errors a worker introduces, and that
  comparison is the worker's responsibility on its own branch.
- Wall-time budget for the gate: ~9 min on warm cache, ~28 min cold.
  This is the cost the gate trades for the documented #905-class
  rework cycle. Codex tabs already pay it via the "optional final
  sanity check" today; this brief just makes it blocking.
- The SWARM_PROTOCOL.md edit should NOT remove the existing
  `Verification` content — only add the subsection. The crate-scoped
  flow stays primary; the gate is the final pre-push step.
