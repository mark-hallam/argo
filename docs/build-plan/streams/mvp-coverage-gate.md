# mvp-coverage-gate — cargo-llvm-cov in CI with ≥85% baseline on core crates

**Owner**: claude. **Depends on**: `mvp-ci-reenable` (CI must be live). **Tier 2 MVP.**

**Authored against**: origin/main 7fdbbdb8 on 2026-05-10.

## Why

V1.0 needs an enforced coverage floor on the load-bearing crates so regressions in core paths are caught at PR time. Spec target: ≥85% line coverage on `argo-core`, `argo-store`, `argo-capability`, `argo-routing`, `argo-judge`.

## Approach (concrete)

1. **Tool**: use `cargo-llvm-cov` (already mature, single-binary, JSON output suitable for CI thresholds).
2. **CI step**: add a coverage job to `.github/workflows/ci.yml` (post-`mvp-ci-reenable`):
   ```yaml
   - name: install cargo-llvm-cov
     run: cargo install cargo-llvm-cov --locked
   - name: coverage
     run: cargo llvm-cov --workspace --json --output-path coverage.json
   - name: enforce per-crate floor
     run: bash scripts/check-coverage-floor.sh coverage.json
   ```
3. **Floor script**: NEW `scripts/check-coverage-floor.sh` (≤ 80 LOC) — parses `coverage.json`, fails if any of {`argo-core`, `argo-store`, `argo-capability`, `argo-routing`, `argo-judge`} drop below 85%.
4. **Baseline measurement**: PR body must include the actual baseline numbers measured locally (so reviewers see if any crate is currently below the floor and needs catch-up tests). If a crate is below 85%, EITHER bring it up via tests in the same PR OR document it with a `# floor-exempt-until: <date>` and a follow-up brief.
5. **Local hook**: add a one-line Makefile target / shell helper for `bash scripts/coverage.sh` that runs the same check locally.

## Files you may touch

- `.github/workflows/ci.yml` — narrowly: add coverage job
- `scripts/check-coverage-floor.sh` (NEW, ≤ 80 LOC)
- `scripts/coverage.sh` (NEW, ≤ 30 LOC) — local convenience wrapper

## Files you must NOT touch

- Any source code (the floor either passes or doesn't; this brief does not write tests)
- A separate brief should be queued if any crate needs new tests to clear 85%

## Acceptance criteria

- [ ] `cargo llvm-cov --workspace --json` runs cleanly locally
- [ ] Floor script correctly fails when a crate is below 85%
- [ ] CI job runs on PR + merge
- [ ] PR body has the baseline measurement table

## Branch + PR

- Branch: `mvp-coverage-gate`
- PR title: `ci: cargo-llvm-cov coverage gate (≥85% on core crates) (mvp-coverage-gate)`
