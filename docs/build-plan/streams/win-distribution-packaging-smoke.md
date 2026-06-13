# win-distribution-packaging-smoke

**Owner**: windows-fleet — **P2 — the single-binary distribution story on Windows.**
**Depends on**: none.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Do
Determine + verify how `argo` is distributed on Windows (single-binary
architecture is a core invariant). Is there a Windows packaging path (raw
argo.exe, an MSI, a zip)? Build the release argo.exe, confirm it's a
self-contained single binary that runs from a clean dir (no missing DLLs beyond
the expected MSVC runtime), and document the Windows install/run steps in
docs/skycot/windows/DISTRIBUTION.md. Flag any non-portable runtime dependency.

## Verify
A freshly-built argo.exe runs from a clean directory on Windows (argo --version);
the distribution doc records the real steps + any DLL/runtime caveat. Branch + IN_REVIEW.
