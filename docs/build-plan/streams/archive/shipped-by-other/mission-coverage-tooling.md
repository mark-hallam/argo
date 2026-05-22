# Mission production blocker #5 — coverage tooling in CI

**Owner**: codex. **Depends on**: mission-reenable-ci-workflows merged.

> SOURCE: deep-research findings tick 198, Section 2, item #10.
> No coverage tool exists in any workflow or Cargo.toml. Coverage
> figures cannot be cited in any external claim because they
> aren't measured.

## Scope

Add `cargo-llvm-cov` to the CI pipeline. On every push to main
and every PR:

1. Compile + run the test suite under `cargo llvm-cov`.
2. Emit a coverage summary to PR comment.
3. Emit a coverage badge artifact (SVG) on `main` push, committed
   back to a `coverage` branch (or uploaded as a release artifact;
   simpler if the GH Actions self-hosted runner can write).

Coverage thresholds are NOT enforced in this brief. Just measure
and report. A future brief can ratchet thresholds once a baseline
is established.

## Files you may touch

- `.github/workflows/ci.yml` — narrowly: add a `coverage` job that
  runs after `test`, depends on it, uses `cargo-llvm-cov`
- `.github/workflows/coverage.yml` (OPTIONAL alternative; either
  layout is fine — one consolidated workflow or split by concern)
- `Cargo.toml` (workspace) — narrowly: nothing (cargo-llvm-cov is
  a CI tool, not a workspace dep). Verify no edits needed.
- `docs/skycot/RUNBOOK.md` — narrowly: append a "Coverage" subsection
  with the local invocation (`cargo llvm-cov --workspace --html`)

## Files you must NOT touch

- Any Rust source file (this brief is CI/docs only)
- ci.yml's existing fmt/clippy/test jobs (just add a new job)

## Acceptance criteria

- [ ] CI runs `cargo llvm-cov --workspace --lcov --output-path
      lcov.info` on every PR
- [ ] Coverage summary line printed to job log
      (`cargo llvm-cov report --summary-only`)
- [ ] Coverage badge SVG generated on main pushes (artifact upload
      OR commit to a `coverage` branch — implementer's choice;
      document)
- [ ] PR opened against this brief shows green CI run with the
      new coverage step
- [ ] No threshold enforcement in this brief (future ratcheting)
- [ ] RUNBOOK updated with local invocation

## Branch + PR

- Branch: `codex-mission-coverage-tooling`
- PR title: `ci: cargo-llvm-cov coverage measurement (mission-coverage-tooling)`

## Notes for the implementer

- `cargo install cargo-llvm-cov` adds ~5 minutes to a cold CI
  cache. If self-hosted runner: install once on the runner. If
  GH-hosted: use a setup-rust-toolchain action that includes
  `llvm-tools-preview` component + `cargo install --locked
  cargo-llvm-cov`.
- DO NOT enforce a coverage threshold yet. The codebase has 65
  silent-skip env-gated tests; until those are converted to
  cassettes, the measured coverage will understate reality.
- After this lands, the headline-reconciliation brief can quote
  a real coverage number rather than "uncited".
