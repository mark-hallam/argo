# win-process-registry-spawn-portability

**Owner**: windows-fleet — **P1 CROSS-PLATFORM — from #2911 (verify production, not just tests).**
**Depends on**: none.
**Authored against**: origin/main `83bce61f4` on `2026-06-13`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/TEST-FAILURES.md (#2911 first Windows test inventory).

## Problem
`argo-tool process_registry` tests spawn unix shell syntax (`sh -c 'for ...; do
... done'`, `true`) that Windows can't run → 2 tests fail. The worker's caveat:
CHECK whether `ProcessRegistry::spawn` hardcodes a unix shell (`sh -c`) in
PRODUCTION — if so the spawn primitive itself isn't portable (a real bug),
not just unix-y test commands.

## Fix
First determine: does production spawn hardcode `sh -c`? If YES → make the
spawn shell selection portable (cmd /C or powershell on Windows, sh -c on
unix) OR document process spawning as unix-only with a cfg-gated Windows path.
If the production spawn is already portable and only the TESTS use unix
commands → make the test commands portable (or cfg-gate them). Be explicit in
the PR which case it was.

## Verify
On Windows: process_registry tests pass (portable commands) AND, if production
was unix-hardcoded, a real Windows spawn works. Conductor re-verifies unix.
Branch + IN_REVIEW.
