# win-store-cascade-and-migration-crlf-investigate

**Owner**: windows-fleet — **P2 INVESTIGATE — from #2911.**
**Depends on**: none.
**Authored against**: origin/main `83bce61f4` on `2026-06-13`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify on Windows; the FIX is cross-platform so the conductor ALSO re-verifies on unix before merge. Push branch, IN_REVIEW; conductor reviews+merges.
Source: docs/skycot/windows/TEST-FAILURES.md (#2911 first Windows test inventory).

## Problem (two suspected-real, need disambiguation)
1. `argo-store` `archive_cascade_deletes_only_matching_tenant_child_rows`: after
   cascade delete, 1 row remains vs expected 0 on Windows. SQLite cascade
   shouldn't differ by OS → either a test-isolation/order flake or a real
   `PRAGMA foreign_keys` setup difference. Re-run in isolation to disambiguate.
2. `argo-identity` `migration_v46_applies_to_v45_db`: column/row count 94 vs 92
   (2 extra) on Windows. Suspect embedded migration SQL checked out with CRLF
   (`
`) changing parsing/counting — could also be a real migration bug.

## Fix
(1) Re-run the cascade test in isolation (`--test-threads=1`); if it passes →
flake, add to KNOWN_FLAKES; if it fails → root-cause the foreign_keys pragma /
cascade setup. (2) Check whether the migration SQL files have CRLF on Windows
(`.gitattributes` text=auto?); if CRLF changes the parse, add a `.gitattributes`
rule pinning migration SQL to LF, OR make the parser CRLF-tolerant. Report which
it was.

## Verify
On Windows: both tests pass (or the cascade is documented as a flake with the
isolation evidence). Conductor re-verifies unix. Branch + IN_REVIEW.


## UPDATE 2026-06-14 (conductor) — migration half RESOLVED
The `migration_v46_applies_to_v45_db` count (94 vs 92) was NOT CRLF — it was a
STALE LITERAL assert (the test hardcoded schema tip 92; this session's migrations
bumped it to 94). It failed identically on macOS (no CRLF). Root-fixed on main
(`ea88d0e89`): the test now references `argo_store::EXPECTED_SCHEMA_VERSION`.
**Only the argo-store archive-cascade item (1 row remains vs 0) remains** — that's
the sole remaining scope of this brief: re-run in isolation (--test-threads=1) to
decide flake vs real PRAGMA foreign_keys/cascade bug.
