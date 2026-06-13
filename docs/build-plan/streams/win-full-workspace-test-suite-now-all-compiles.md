# win-full-workspace-test-suite-now-all-compiles

**Owner**: windows-fleet — **P1 — definitive Windows test pass (newly unblocked).**
**Depends on**: none.
**Authored against**: origin/main `c24ae0d84` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform fixes → conductor re-verifies unix. Push branch, IN_REVIEW; conductor reviews+merges.

## Why now
The #2911/#2916 inventories EXCLUDED argo-cli (kryptering didn't link) and the
CU-executor crate. Both now compile on Windows (#2905, #2906). So a FULL
`cargo test --workspace` on Windows is possible for the FIRST time — argo-cli
(the biggest crate, never Windows-tested) especially.

## Do
`cargo test --workspace --no-fail-fast` on Windows. Refresh
docs/skycot/windows/TEST-FAILURES.md to the definitive all-crates picture.
Categorize remaining fails: (a) real cross-platform bug, (b) test-assumes-unix,
(c) flake. Fix the small/clear (a)+(b) cases in THIS PR where safe (cross-
platform fixes only, conductor re-verifies unix); spawn targeted briefs for
anything large or ambiguous. NO THEATER — honest inventory.

## Verify
The refreshed inventory covers ALL crates (no exclusions); any fixes keep unix
green (conductor re-runs touched crates on macOS). Branch + IN_REVIEW.
