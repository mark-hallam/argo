# win-test-suite-run-and-failure-inventory

**Owner**: windows-fleet — **P2 — discovery.**
**Depends on**: win build/portability briefs largely MERGED.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Do
`cargo test --workspace` on Windows (once the workspace compiles). Inventory
every failure into docs/skycot/windows/TEST-FAILURES.md, categorized:
(a) real cross-platform bug, (b) test assumes unix (path sep / line ending /
/tmp / perms), (c) windows-env flake. Suggest a fix + owning follow-up brief
per row. Do NOT mass-fix here — inventory, then spawn targeted briefs.

## Verify
Report exists + categorized + cross-referenced. Branch + IN_REVIEW.
