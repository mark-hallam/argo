# win-doc-executor-chrome-path-windows-discovery

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — PDF export can't find Chrome on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `b7135a81d` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-doc-executor-chrome-path-windows-discovery`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2-2026-06-15.md (finding 7 of 7).

## Problem
`crates/argo-executor-document/src/pdf.rs:68` `resolve_chrome_path()` returns the
`ARGO_DOC_CHROME` override or a HARDCODED macOS path (`DEFAULT_CHROME_PATH =
/Applications/Google Chrome.app/...`). No Windows arm / discovery. On Windows the
macOS path never `.exists()`, so `invoke_chrome` (pdf.rs:131) returns
`ChromeMissing` even when Chrome IS installed at the standard Windows location →
PDF generation silently non-functional unless the operator knows to set
`ARGO_DOC_CHROME`. (Reachable via `executor.rs:223` Pdf dispatch.)

## Fix
cfg-gate `DEFAULT_CHROME_PATH` to `not(windows)` (unix unchanged). Add a
`#[cfg(windows)]` `windows_chrome_path()` that probes the standard install
locations — `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LOCALAPPDATA%` for
`Google\Chrome\Application\chrome.exe`, then Edge `msedge.exe` as a Chromium
fallback — first existing wins, else the first candidate (so `ChromeMissing`
names a real path). `resolve_chrome_path` keeps `ARGO_DOC_CHROME` as the
highest-priority source on all platforms; the `ChromeMissing` hint now includes
Windows guidance. Unix byte-identical (macOS + linux still use `DEFAULT_CHROME_PATH`).

## Verify
On Windows: `cargo test -p argo-executor-document --lib` green (incl. the new
`windows_chrome_path_returns_exe_candidate` + the existing env-override test) +
clippy clean. Conductor re-verifies unix (not(windows) arm unchanged). Branch + IN_REVIEW.
