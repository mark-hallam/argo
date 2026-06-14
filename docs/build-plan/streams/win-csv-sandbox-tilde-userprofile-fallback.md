# win-csv-sandbox-tilde-userprofile-fallback

**Owner**: windows-fleet — **P2 CROSS-PLATFORM — CSV exports land in a literal `~` dir on native Windows.**
**Depends on**: none.
**Authored against**: origin/main `b7135a81d` on `2026-06-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-csv-sandbox-tilde-userprofile-fallback`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/PORTABILITY-AUDIT-2-2026-06-15.md (finding 4 of 7).

## Problem
`crates/argo-executor-document-csv/src/sandbox.rs:28` defines
`DEFAULT_OUTPUT_ROOT = "~/.argo/exports/csv/"`, the runtime default when a CSV
manifest omits `output_root` (`parse_request`,
worker_actions_executor_document_csv.rs:128). It is expanded by the LOCAL
`expand_tilde` (sandbox.rs:121) which reads only `HOME` — NO `USERPROFILE`
fallback. #2904 added HOME→USERPROFILE to every other tilde resolver but missed
this copy. On native Windows (HOME unset) `expand_tilde` returns the path
verbatim → `Sandbox::open` creates a literal `~` directory under the process CWD
instead of the user profile.

## Fix
Mirror the #2904 pattern: route `expand_tilde` through a `home_dir()` that falls
back HOME→USERPROFILE (empty ignored) via a pure env-injected `home_dir_from` +
unit tests. Unix byte-identical (HOME set ⇒ same path).

## Verify
On Windows: `cargo test -p argo-executor-document-csv --lib` green (incl. the 4
new `home_dir_from` cases) + clippy clean on sandbox.rs. (NB a PRE-EXISTING
Windows-only clippy warning exists in `document_csv_tests.rs:5` — `DEFAULT_TENANT_SEGMENT`
is imported but its only use, line 253, is inside a `#[cfg(unix)]` block, so it
reads unused on Windows; NOT this change — diff is sandbox.rs only.) Conductor
re-verifies unix. Branch + IN_REVIEW.
