# Coverage threshold enforcement — Phase C follow-up to #942

**Owner**: claude. **Depends on**: #942 (mission-coverage-tooling, merged).

> SOURCE: production hardening Phase C. PR #942 wired
> `cargo-llvm-cov` into CI but deliberately set NO threshold —
> the pipeline reports coverage and never fails. This brief
> turns the report into a guardrail for the critical crates.

## Scope

Wire a coverage threshold check into CI that fails the build
when a critical crate's line coverage drops below the floor.

Critical crates (initial set):
- `argo-store` — schema + persistence; bugs corrupt user data
- `argo-skill` — skill execution path; bugs leak between tenants
- `argo-judge` — eval loop; bugs invalidate ranking signal

Initial floor: **70%** line coverage per crate. Tunable via
`scripts/check-coverage-threshold.sh` constants. The script:

1. Parses `target/llvm-cov/json/coverage.json` (already produced
   by the existing `cargo llvm-cov --json` step from #942).
2. For each crate in the critical list, extracts the per-file
   line coverage, aggregates by crate, and compares to the floor.
3. Prints a markdown table to stdout (rendered in the GH Actions
   summary tab via `$GITHUB_STEP_SUMMARY`).
4. Exits non-zero if any critical crate drops below its floor.

Non-critical crates report-only (existing behavior).

## Files you may touch

- `.github/workflows/ci.yml` — narrowly: append a step after the
  existing `cargo llvm-cov --json` step that runs
  `scripts/check-coverage-threshold.sh`. Pin the bash version.
- `scripts/check-coverage-threshold.sh` (NEW) — ~80 LOC bash;
  parses the llvm-cov JSON, applies the per-crate floor table,
  emits the markdown summary, exits non-zero on failure.
- `docs/skycot/RUNBOOK.md` — narrowly: append a "Coverage
  thresholds" subsection (~10 lines) explaining how to tune the
  floor and what to do when CI fails on coverage drop.

## Files you must NOT touch

- `Cargo.toml` workspace config — no new deps; `jq` is already
  available on the GitHub runner.
- Source code in `argo-store`/`argo-skill`/`argo-judge` —
  this brief is CI-only; if coverage is below floor today, the
  follow-up brief adds tests, not this one.

## Acceptance criteria

- [ ] `scripts/check-coverage-threshold.sh` exists, executable,
      passes shellcheck
- [ ] Script reads `target/llvm-cov/json/coverage.json` and
      extracts per-crate line coverage
- [ ] Critical crates list (argo-store, argo-skill, argo-judge)
      with 70% floor each — tunable at top of script
- [ ] Markdown summary written to `$GITHUB_STEP_SUMMARY`
- [ ] CI fails with clear error message when any critical crate
      drops below floor
- [ ] CI passes when all critical crates meet floor
- [ ] One smoke test in `scripts/check-coverage-threshold.sh`
      itself (a `--self-test` flag with a fixture JSON)
- [ ] RUNBOOK updated with tuning + failure runbook
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-coverage-threshold-enforcement`
- PR title: `ci: enforce coverage threshold on critical crates (claude-coverage-threshold-enforcement)`

## Notes for the implementer

- Do NOT bump the floor above whatever the current critical
  crates already achieve — the goal is "don't regress," not
  "raise the bar today." If a crate is currently at 65%, set
  the floor to 60% so this PR is green; bumping is a future
  brief.
- The llvm-cov JSON shape is documented at
  https://llvm.org/docs/CommandGuide/llvm-cov.html#export — use
  `jq` to extract, no need for a Rust helper.
- Skip flaky exclusions in this brief — if a file is excluded
  from coverage today, it stays excluded; this brief only adds
  the threshold gate.
