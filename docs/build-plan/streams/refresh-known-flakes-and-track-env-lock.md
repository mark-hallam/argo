# refresh-known-flakes-and-track-env-lock

Authored against: origin/main 8286d93b8247ff392fd0f118ea8c0e2c5b980b13 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh refresh-known-flakes-and-track-env-lock

## Goal

Refresh docs/build-plan/KNOWN_FLAKES.md so it reflects current reality: remove
the env-lock parallel-flake class (FIXED by #2781), correct the stale web-suite
stats, and update the "last refreshed" date.

## Why now (test-health audit 2026-06-10, F3)

KNOWN_FLAKES.md says "Last refreshed: 2026-05-22" (19 days stale). Its web-suite
stats ("1339 tests, 168 files") are ~half current reality (377 *.test.ts(x)
under web/). It never mentioned the ~19-test gateway env-lock parallel-flake
class — which #2781 (env-lock migration to shared_env_lock) has now ROOTED OUT.
The boundary-note protocol depends on this doc; stale → false exemptions +
wasted re-verification.

## Fix

1. Bump "Last refreshed" to today.
2. Update web-suite counts to current (`ls web/**/*.test.ts* | wc -l`).
3. Add + immediately mark RESOLVED the env-lock class (FIXED #2781) OR just
   document that gateway env/cwd tests now use the canonical shared_env_lock
   (no per-module ENV_LOCK), so that flake class is closed.
4. Re-scan for any genuinely-current flakes (rg '#\[ignore' with reasons) and
   reconcile the list.

## May touch

- docs/build-plan/KNOWN_FLAKES.md only

## Acceptance / verifier

1. Date is today; web counts match `ls`; env-lock class documented as closed.
2. No claim in the doc contradicts current main (spot-check 3 entries).

## Size / owner

Doc-only. Owner: claude (codex-eligible). 🟢 P3 — keeps the flake-exemption
protocol honest.
