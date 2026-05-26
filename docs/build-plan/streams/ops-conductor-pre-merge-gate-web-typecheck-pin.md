# ops-conductor-pre-merge-gate-web-typecheck-pin

**Owner**: claude or codex
**Priority**: 🟢 P3 (process, fast-mode)
**LOC estimate**: ~30 LOC bash
**Lane**: ops-* (overnight `--skip-clippy` authorization applies)
**Authored against**: origin/main `78f162a5a` on 2026-05-26
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-conductor-pre-merge-gate-web-typecheck-pin`

## Source

Evaluator round 86 §S2 optional process candidate. 2 TypeScript
regressions on `web/` in under 12 hours (PR #2404 `use-provision.ts`
TS2741 + PR #2366 `ProvenanceBlock.tsx` TS2345) justify pinning
`pnpm tsc --noEmit` into the pre-merge gate whenever a PR touches
`web/`. Currently the gate runs Rust clippy + workspace clippy +
tests but never invokes the SPA TypeScript compiler — so a
TS regression sails through CI today.

Out of v1.0 demo critical path; defensible process hardening for
the SPA tier that's now demo-load-bearing.

## Problem

`scripts/conductor-pre-merge-gate.sh` runs 7 stages — Rust-only.
A PR that breaks `pnpm tsc --noEmit` can still admin-merge under
the web-only or evaluator-pre-verified lanes today because nothing
in the gate catches it. Evaluator round 85 and round 86 each caught
one such regression on main, but only because evaluator happened
to run `pnpm --dir web tsc --noEmit` independently — there is no
deterministic gate.

## Fix

Add a new stage (suggested: between LOC-budget check and clippy)
that runs `pnpm --dir web tsc --noEmit` IF the PR diff touches any
file under `web/`. Skip when `--skip-clippy` is passed AND the PR
is in the doc-only / test-only lane (no web file changes). Honour
the same `--skip-tests` escape hatch as gate 6 if it already exists.

Suggested location: a new helper
`scripts/pre-merge-web-typecheck.sh` that the main gate calls,
matching the existing
`scripts/pre-merge-test-compile.sh` pattern.

## Acceptance criteria

- New stage runs in < 60s warm (the SPA workspace is small).
- Stage SKIPS with a green log line when the PR touches no
  `web/**` files.
- Stage FAILS the gate loudly when `pnpm tsc --noEmit` exits
  non-zero, with the first 20 lines of stderr surfaced.
- Authorized for the same `--skip-clippy` lane as other ops-*
  gate work (overnight window applies).
- Smoke harness at `scripts/conductor-pre-merge-gate-web-typecheck-tests.sh`
  with 3 cases: (a) PR touches no `web/` → SKIP, (b) PR touches
  `web/` with clean tsc → PASS, (c) PR touches `web/` with broken
  tsc → FAIL.
- Gate help text (`--help`) lists the new stage.

## May touch

- `scripts/conductor-pre-merge-gate.sh` (add stage)
- `scripts/pre-merge-web-typecheck.sh` (new helper)
- `scripts/conductor-pre-merge-gate-web-typecheck-tests.sh` (new smoke)

## Out of scope

- Pinning `pnpm --dir web test` (vitest) — vitest is 1500+ tests
  and would slow the gate too much. Web-only merge authorization
  already requires "vitest green in body" so this is operator-eyes
  rather than gate-eyes.
- Pinning `pnpm --dir web lint` (eslint) — separate brief; lint
  warnings are not gate-fail-worthy by default.

## Parallel-surface check

- `rg 'pnpm.*tsc' scripts/` → no existing gate call site.
- `rg 'pre-merge-' scripts/` → 3 existing per-stage helpers
  (`pre-merge-test-compile.sh`, `pre-merge-test-carry-allowlist.txt`,
  etc.) so the helper-script pattern is established.
