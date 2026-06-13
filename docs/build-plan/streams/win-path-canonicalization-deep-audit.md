# win-path-canonicalization-deep-audit

**Owner**: windows-fleet — **P2 — UNC / drive-letter / long-path audit (paths were the dominant Windows bug class).**
**Depends on**: none.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Why
Path handling was the #1 Windows bug source this session (capability glob,
fs_root join, etc.). Do a DEEP audit beyond the fixed cases: UNC paths
(\\server\share), drive letters (C: vs relative), the long-path \\?\
prefix (>260 chars), and case-insensitivity. Grep gateway/store/tool/identity
for path construction + comparison.

## Fix
Normalize path construction to `Path`/`PathBuf` (no literal separators);
canonicalize for comparison; ensure globs use `/` (globset). Fix any real
mishandling found; cfg-gate where genuinely platform-specific. Cross-platform —
conductor re-verifies unix.

## Verify
On Windows: UNC + drive-letter + long-path cases handled (add targeted tests);
conductor re-runs touched crates on unix green. Branch + IN_REVIEW.
