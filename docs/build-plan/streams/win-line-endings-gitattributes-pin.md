# win-line-endings-gitattributes-pin

**Owner**: windows-fleet — **P2 — defensive CRLF pin (CRLF was suspected 3x; pin it shut).**
**Depends on**: none.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Why
CRLF was hypothesized as the cause of failures 3x this session (it never was —
each was a real bug or stale literal). Pin the repo so CRLF can NEVER silently
corrupt embedded SQL migrations, fixtures, or golden files on a Windows checkout.

## Fix
Add/extend `.gitattributes`: pin migration SQL, test fixtures, golden files,
and any byte-sensitive embedded asset to `text eol=lf`. Verify a fresh Windows
checkout gets LF on those files. Document the policy. This forecloses a whole
class of future Windows-only flakiness.

## Verify
`.gitattributes` pins the byte-sensitive files to LF; a Windows checkout
confirms LF on a sampled migration SQL file. Branch + IN_REVIEW.
