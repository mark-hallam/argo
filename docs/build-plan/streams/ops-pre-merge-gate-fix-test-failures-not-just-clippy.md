# Stream ops-pre-merge-gate-fix-test-failures-not-just-clippy — add a non-skippable test gate to `conductor-pre-merge-gate.sh`

**Owner**: claude (bash/scripting; ~100-200 LOC).
**Depends on**: none.
**Authored against**: origin/main `be9206ef2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-pre-merge-gate-fix-test-failures-not-just-clippy`.

🟡 **P1 gate-hardening.** Source: round-41 evaluator finding §"Brief candidates surfaced this round" (a) + r45-r53 carry-forward (6th-consecutive round). The pre-merge gate currently has 6 gates (`scripts/conductor-pre-merge-gate.sh`); none of them run `cargo test`. Gate 6 (workspace clippy) is the only cross-crate sanity check, and it's `--skip-clippy`-able. Several PRs this week landed with red `cargo test` because the gate only caught clippy.

**Re-author note**: this brief's stream file was missing on disk despite the BLOCKED ledger row existing since tick-116. This re-author restores the brief with concrete acceptance criteria so the row can flip BLOCKED→TODO this tick.

## Scope

Add a new gate 7 (test) to `scripts/conductor-pre-merge-gate.sh` that:

1. Is **NOT skippable** by `--skip-clippy`. A new dedicated flag `--skip-tests` exists but is intended only for the no-Rust-touched case and emits a louder warning.
2. Runs `cargo test --workspace --no-run --release-deps-only` style — focus on **building** tests cleanly. Full execution of every test is too slow under load (the audit-flagged contention). The new gate's invariant: every test crate **compiles** against the current workspace; runtime green is a secondary verification the worker is expected to run per-crate.
3. Honors an **explicit-carry allowlist file** at `scripts/pre-merge-test-carry-allowlist.txt` listing test files known-red and explicitly carried (one filename per line, with comment lines starting `#`). Workers register a known red here when they ship anti-theater-honest disclosure footnotes; conductor reviews periodically.
4. Has a smoke harness `scripts/conductor-pre-merge-gate-tests.sh` that exercises the new gate against (a) a clean repo (passes), (b) a temporary broken test (fails), and (c) a broken test on the allowlist (passes with warning).

The implementation lives in `scripts/conductor-pre-merge-gate.sh` between gate 5 and gate 6 (so it runs before the slow clippy step — if tests fail to compile, no point running clippy).

Fast-mode candidate. Pure bash + ~10-20 LOC of test-compile invocation. The allowlist file is the substantive UX surface — get its format right.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`scripts/conductor-pre-merge-gate.sh`]

## Files you may touch

- `scripts/conductor-pre-merge-gate.sh` — add gate 7, renumber clippy to 7 of 7 (or keep clippy as 6 and tests as 7 — gate-numbering is cosmetic).
- `scripts/conductor-pre-merge-gate-tests.sh` (NEW) — smoke harness with 3 test cases.
- `scripts/pre-merge-test-carry-allowlist.txt` (NEW) — initially seeded with `turn_skills_tests::legacy_worker_treated_as_lowest_grade` (the round-50 red) + any others the worker confirms on origin/main.

## Tests

- Smoke harness above (~30 LOC bash test wrapper + 3 cases).
- Manual smoke against current main: `bash scripts/conductor-pre-merge-gate.sh` should pass with the allowlist seeded; without the allowlist entry it should FAIL.

## Definition of done

- New gate exists, runs before clippy, is NOT bypassed by `--skip-clippy`.
- Smoke harness passes (3/3).
- Allowlist file documented inline (top-of-file comments) explaining the carry workflow.
- PR body includes: (a) `bash scripts/conductor-pre-merge-gate.sh` run-output showing the new gate, (b) explanation of the allowlist format.

Notes: this is the load-bearing fix for the round-45 audit P0 ("gate-mask vector"). Keep the surface narrow — don't fold in cargo-cache eviction or other gate changes. Pair-merge with `ops-cargo-cache-eviction-policy-when-over-150gb` if both land same tick.

**Ledger BLOCKED → TODO flip**: this brief's row was BLOCKED waiting on stream re-author. This file IS the re-author. Conductor flipping row in the same commit.
