# win-ci-windows-build-job-authored

**Owner**: windows-fleet — **P3 — future-guard.**
**Depends on**: none.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Goal
Author a GitHub Actions workflow `.github/workflows/windows-build.yml` (kept
DISABLED per the operator GHA-billing policy — add it to the DISABLED list)
that runs `cargo build --workspace` + `cargo test` on `windows-latest`, so
Windows portability is guarded the moment CI billing is restored. Mirror the
existing disabled-workflow conventions (grep .github/workflows + the
DISABLED_WORKFLOWS_POLICY.md).

## Verify
Workflow file is valid YAML, references windows-latest, and is listed in the
disabled-workflows policy with rationale. Branch + IN_REVIEW.
