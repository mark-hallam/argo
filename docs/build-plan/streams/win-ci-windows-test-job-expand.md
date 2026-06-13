# win-ci-windows-test-job-expand

**Owner**: windows-fleet — **P3 — expand the disabled Windows CI to the full surface.**
**Depends on**: none.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Do
Expand `.github/workflows/windows-build.yml.disabled` (from #2918) to also:
run `cargo test --workspace` on windows-latest (now that all crates compile),
build the CU-executor crate, and run the non-input-gated unit tests (e.g. the
PNG-encode decode test). Keep it DISABLED (GHA billing blocked) + update the
DISABLED_WORKFLOWS_POLICY.md note. So the FULL Windows surface (not just build)
is CI-guarded the moment billing returns.

## Verify
Valid YAML; covers build + test + CU-executor; stays .disabled + policy-noted.
Branch + IN_REVIEW.
