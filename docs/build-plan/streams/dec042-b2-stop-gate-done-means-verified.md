# dec042-b2-stop-gate-done-means-verified

**Owner**: claude.
**Authored against**: origin/main `421a5e725` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh dec042-b2-stop-gate-done-means-verified`.

> DEC-042 Stage 1, upgrade **B-2** (the second build-loop hardening step, after
> B-1 which MERGED as #2753). See
> [`docs/skycot/decisions/DEC-042-continuous-self-improvement-loop.md`](../../skycot/decisions/DEC-042-continuous-self-improvement-loop.md)
> §3 B-2 + §5 Stage 1. **Claim only after B-1 is on main** (it is — the conductor
> build worktree `scripts/conductor-build-worktree.sh` is what lets the gate run
> without contending with workers). This is the top Stage-1 row now.

## Why (VERIFIED 2026-06-09, read-only)
"Done" is **trust-based**, not mechanically enforced — this is the NO-THEATER
(DEC-029) hole compiled into the worker loop:

- The worker is a headless `claude -p` session. Its prompt
  (`scripts/claude-worker-loop.sh:397-409`) *instructs* it to "Run verification
  per SWARM_PROTOCOL.md — crate-scoped clippy + `bash scripts/workspace-clippy-fast-fail.sh`"
  THEN `gh pr create` THEN flip the ledger row to `IN_REVIEW`. Nothing **enforces**
  that order — a session can skip the gate and open a PR anyway.
- The only post-session backstop, the "ship-check"
  (`claude-worker-loop.sh:772-799`), checks whether a **PR was opened** (row still
  `CLAIMED` → release as zombie). It does **NOT** check that the gate passed. So a
  PR can reach `IN_REVIEW` with a failing `cargo test` / clippy / LOC-budget /
  missing ledger-in-band update, and the conductor only finds out at merge-gate
  time (or, worse, a `--skip-clippy` lane merges it).
- There is no `.claude/hooks/` dir and no `Stop` hook in `.claude/settings*.json`
  (verified: both absent as of `421a5e725`).

DEC-042 B-2 verifier (the bar this brief must hit): *"a worker that leaves a
failing test cannot exit 'done'; the hook blocks it."* I.e. make `IN_REVIEW`
**unreachable** unless the gate passed — bind "done" to "verified".

## Grep-before-write
- `rg -n 'gh pr create|IN_REVIEW|workspace-clippy-fast-fail|pre-merge|ship-check|set_status' scripts/claude-worker-loop.sh` — the ship path (≈397-409 prompt, ≈772-799 ship-check) where enforcement must bind.
- `ls scripts/ | rg -i 'gate|clippy|pre-merge|loc-budget|render-feature-matrix'` — the existing checkable gates to REUSE (do NOT author a parallel gate): `conductor-pre-merge-gate.sh`, `pre-merge-test-compile.sh`, `workspace-clippy-fast-fail.sh`, `check-loc-budget*.sh`, the ledger-in-band guard. B-2 composes these, it does not reinvent them.
- `rg -n 'ARGO_CONDUCTOR_BUILD_WORKTREE|conductor-build-worktree' scripts/conductor-pre-merge-gate.sh scripts/conductor-build-worktree.sh` — B-1's worktree helper the gate runs in (so B-2's gate doesn't peg the workers' RAM).
- `rg -n 'hooks|Stop' .claude/settings*.json` — confirm no existing Stop hook to collide with (none as of `421a5e725`).
- Confirm the Claude Code `Stop`-hook contract for **headless `-p`** sessions before relying on hard-block semantics (a `Stop` hook returning a block decision must actually prevent the `-p` session from terminating as done — verify empirically; if it is advisory-only in `-p` mode, use the bash-wrapper backstop in Fix #2 as the load-bearing enforcement).

## Fix (default-OFF flag `ARGO_STOP_GATE`; ship the enforcement that actually blocks)
First-principles: enforcement must live where it **cannot be skipped by the thing
being graded** (the worker session). Two layers, both behind `ARGO_STOP_GATE=1`
(default OFF per DEC-042; flip after a blast-radius check):

1. **A reusable gate-verifier `scripts/stop-gate-verify.sh`** (NEW, < 200 LOC).
   Given a brief id + its worktree, runs the existing checks **in the B-1
   conductor build worktree** (`ARGO_CONDUCTOR_BUILD_WORKTREE` path) so it does
   not contend with workers: (a) `cargo test` for the touched crate(s) +
   `workspace-clippy-fast-fail.sh`, (b) LOC budget, (c) **ledger-in-band**: the
   diff that touched a shipped capability also touched its `FEATURE_MATRIX.md`
   row (+ regenerated `feature-matrix.html`). Exit 0 = gate passed; non-zero +
   a one-line reason = blocked. This is the single checkable verifier "done"
   binds to.
2. **Bash-wrapper enforcement (the load-bearing backstop)** in
   `claude-worker-loop.sh`: after the headless session returns but BEFORE the
   ship-check accepts `IN_REVIEW`, if `ARGO_STOP_GATE=1`, run
   `stop-gate-verify.sh`. On failure: do NOT leave the row at `IN_REVIEW` —
   revert it to a `NEEDS-FIX`/`TODO` state (or hold `CLAIMED` for one retry) and
   post the failure reason as a PR comment, so a failing-gate change can never
   *present* as done. Additive + default-OFF: with the flag unset, behavior is
   byte-identical to today.
3. **The `Stop` hook (the faithful-to-DEC layer, if it hard-blocks in `-p`)**:
   add `.claude/hooks/stop-gate.sh` wired via a worker-scoped `Stop` hook that
   calls `stop-gate-verify.sh` and returns a block decision on failure, so the
   session itself refuses to finish "done". Gate it behind the same flag. If
   headless `-p` treats `Stop` as advisory (verify per grep-before-write), keep
   it as defence-in-depth and rely on Fix #2 as the enforcer.

Do NOT change which row a worker claims, the row schema, or any behavior when
`ARGO_STOP_GATE` is unset.

## Acceptance (truth-bounded — the meta-verifier IS the deliverable)
1. **`scripts/test-stop-gate.sh`** (NEW, < 150 LOC) — the DEC-042 B-2 verifier,
   self-contained: stand up a throwaway brief whose worktree has a **deliberately
   failing** `cargo test` (or a LOC-budget violation, simulated cheaply without a
   full compile where possible — e.g. a stub crate or a forced non-zero from a
   fake gate shim), run the enforcement path with `ARGO_STOP_GATE=1`, and assert
   the row does **NOT** end at `IN_REVIEW` (it is held / reverted) and a reason
   was recorded. Then a passing case → reaches `IN_REVIEW`. Run 2×, deterministic.
2. **No-behavior-change proof**: with `ARGO_STOP_GATE` unset, the ship-check +
   ledger flip are byte-identical to `421a5e725` (diff the code paths / run the
   loop's ship path with the flag off → unchanged).
3. `bash -n` + `shellcheck -x` clean on all new scripts; each < 500 LOC;
   `claude-worker-loop.sh` does not regress its structure (extract into a sourced
   helper if the wire-in would bloat a block).
4. The gate-verifier reuses the existing gate scripts (no parallel gate); the
   conductor build worktree is used so the verify does not peg worker RAM.

## On ship (same commit)
- `FEATURE_MATRIX.md` row for "B-2 Stop-gate (done = gate-verified, `ARGO_STOP_GATE`
  default-OFF)" + regenerate `docs/skycot/feature-matrix.html` via
  `python3 scripts/render-feature-matrix.py`, same commit.
- Note it advances DEC-042 Stage 1 (B-1 → **B-2 done** → B-3 next: automated
  rule-capture). The `pct_done_claims_pass_cold_reverify` north-star metric
  (`scripts/north-star-metrics.py`, currently `null`) becomes instrumentable once
  this lands — name that as the B-2→metrics follow-up.

## Effort
M (one reusable verifier script composing existing gates + a guarded wire-in +
a self-contained meta-verifier; bash; default-OFF; no Rust runtime change).
