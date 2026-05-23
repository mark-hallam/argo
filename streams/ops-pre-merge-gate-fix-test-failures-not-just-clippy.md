# ops-pre-merge-gate-fix-test-failures-not-just-clippy

**Authored against:** `origin/main` 91ca7ace7e5b on 2026-05-23.
**Validate before claim:** `bash scripts/check-brief-freshness.sh ops-pre-merge-gate-fix-test-failures-not-just-clippy`.

## Why this brief exists

Evaluator round 41 (docs/skycot/evals/scheduled/2026-05-23/round-41-worker-skill-overlay-blob-text-join-mismatch-verify.md) independently verified — via cold `cargo test -p argo-skill --lib store_worker_overlay_tests::` on `origin/main` HEAD `bc78952a7` — that **2 of 5 pin tests are RED on main** (`list_for_worker_returns_attached_active_overlays`, `list_for_worker_filters_other_workers`).

These tests have been RED across a 6-PR carry-chain (#1855 / #1869 / #2110 / #2122 / #2143 / #2152) because every admin-merge in the chain ran `scripts/conductor-pre-merge-gate.sh --skip-clippy` to bypass the clippy gate. That flag silently disables gate 5, which calls `workspace-clippy-fast-fail.sh` — and **the conductor has been treating "skip clippy" as "skip cross-crate sanity check" without realising no other gate runs `cargo test`**. As long as the gate is silent on test failures, the conductor cannot tell a clippy-only DIRTY tree from a test-RED tree, and the swarm's "land scaffold now, wire later" failure mode (CLAUDE.md drift-prevention §1) extends to broken tests on main, which is strictly worse than broken lints.

Evaluator round 41 §"Brief candidates surfaced this round" (a):
> `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (P0, ~30-50 LOC scripts) — the pre-merge gate's `--skip-clippy` escape hatch is structurally masking RED tests. At minimum the gate should parse `cargo test` output for FAILED test names separately from clippy warnings and refuse `--skip-clippy` when any test is RED.

Goal: close the structural masking by adding a **separate test gate** that `--skip-clippy` cannot bypass, and that surfaces RED tests by name so the conductor sees them before admin-merge.

## Acceptance criteria

1. **New gate 6 in `scripts/conductor-pre-merge-gate.sh`**: a test-only gate that runs **fast** (default: workspace `cargo test --no-run --workspace --all-targets` to compile-check tests as a baseline, plus `cargo test -p argo-skill --lib` and `cargo test -p argo-store --lib` as the two crates with the highest historical RED-pin rate per the 2026-05-23 boundary-note chain). Cold runtime budget: ≤4 min. Warm: ≤45s.
2. **Gate 6 NOT skippable by `--skip-clippy`.** A new flag `--skip-tests` may be added with a louder warning (`warn` line plus a non-fatal `fail_count` increment that prints but does not exit non-zero) so the conductor can still merge a known-broken-test PR when the failure is **explicitly carried in the PR boundary note and the fix-brief is queued** — but the default must surface every test failure by name.
3. **Output format**: when gate 6 fails, list each failing test by `<crate>::<test_path>` on its own line so the conductor can grep the BUILD_STATUS.md ledger for a matching fix-brief. Example:
   ```
   [pre-merge-gate FAIL] 2 test(s) RED in -p argo-skill --lib:
     argo-skill::store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays
     argo-skill::store_worker_overlay_tests::list_for_worker_filters_other_workers
   ```
4. **Pre-existing RED tests on main are allowlisted with an explicit "carry" marker.** New file `scripts/conductor-pre-merge-gate-test-carries.txt` (1 test per line, with `# <brief-id> <date>` comment), seeded with the 2 round-41-verified `store_worker_overlay_tests` entries pointing at `fix-skill-worker-overlay-blob-text-join-mismatch`. Gate 6 reads this file, subtracts the carried tests from its FAIL list, and emits a `warn` line for each one. When a carry's fix-brief MERGES the brief-id reference becomes invalid — a sibling check (run weekly or on every tick that touches this file) should grep the ledger and warn on stale entries; out of scope for v1, optional for v2.
5. **Smoke harness** at `scripts/test/test-conductor-pre-merge-gate-test-gate.sh` (or equivalent): hermetic test that seeds a temp workspace with (a) a passing test crate, (b) a failing test crate, (c) a carried failing test crate; asserts gate 6 outputs the right combination of FAIL / warn lines and exits non-zero on (b) only. ~50-80 LOC bash.
6. **Documentation**: update the gate's header comment with the gate 6 rationale + a one-line entry in `docs/build-plan/PROTOCOL.md` under "pre-merge gates" listing all 6 gates.

## Parallel-surface declarations

- `scripts/conductor-pre-merge-gate.sh` already exists; extend it, do not parallel-author.
- `scripts/workspace-clippy-fast-fail.sh` is clippy-only by name and design — do NOT add tests to it (would change the contract).
- `scripts/check-findings-only-fix-by.sh` / `scripts/check-todo-aging.sh` are deadline trackers, not gates — do not duplicate.
- `scripts/conductor-pre-merge-gate-test-carries.txt` is new. There is no existing carry-tracking file; the closest sibling is the per-PR boundary-note pattern, which lives in PR bodies and is not machine-readable.

## May-touch

- `scripts/conductor-pre-merge-gate.sh` (extend with gate 6, new `--skip-tests` flag handling)
- `scripts/conductor-pre-merge-gate-test-carries.txt` (new, seeded)
- `scripts/test/test-conductor-pre-merge-gate-test-gate.sh` (new smoke harness)
- `docs/build-plan/PROTOCOL.md` (one-line gate-list entry under "pre-merge gates")

## Boundary

- Do NOT modify `workspace-clippy-fast-fail.sh`. The masking is in the conductor gate's `--skip-clippy` flag, not in the clippy script itself.
- Do NOT fix any of the carried RED tests in this brief — that's `fix-skill-worker-overlay-blob-text-join-mismatch`'s job. This brief is the gate, not the fix.
- Do NOT add a CI workflow trigger. GitHub Actions remain operator-disabled per HOST_SETUP runbook. The conductor-local gate is the only gate path until GHA returns.
- Do NOT introduce a `--force` / `--no-gate` flag — the explicit-carry pattern is the escape hatch.

## Sizing

~80-150 LOC bash + 50-80 LOC smoke harness + 2 docs lines. Fast-mode candidate.

**🔴 P0 OPS (gate hardening)** — the swarm has been silently merging broken tests for 6 PRs because the gate doesn't distinguish lints from test failures. This brief closes that masking before another 6-PR chain accumulates.
