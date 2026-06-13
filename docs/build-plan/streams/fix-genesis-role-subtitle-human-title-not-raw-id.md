# fix-genesis-role-subtitle-human-title-not-raw-id

**Owner**: claude or codex — **P2 FIRST-IMPRESSION (re-test F13 remainder).**
**Depends on**: none.
**Authored against**: origin/main `2319a7a69` on `2026-06-13`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-genesis-role-subtitle-human-title-not-raw-id

Source: docs/skycot/audit/2026-06-13-e2e-platform-shakedown.md ("Re-test 2026-06-13" section — operator-contracted shakedown re-test).

## Problem
A Genesis-hired teammate's role subtitle renders as the raw
"Genesis Rex 019ebea4d72d1b81" (provenance string + plan id) on BOTH the worker
header and the team-card badge, instead of a human role title (e.g.
"Collections Coordinator"). Avatar initials were fixed; this is the remaining
F13 gap. It's the first thing a user sees after the hire — the magic dips.

## Fix
Grep where the worker subtitle/role-title is derived for genesis workers
(provenance_chain → display). Genesis already synthesises a charter + role;
surface the human role TITLE there, not the "Genesis <name> <plan_id>"
provenance marker. Fall back to a sensible derived title if none captured.
Keep the provenance available where it belongs (audit/provenance view), just
not as the user-facing role label.

## Verify
Test: a genesis hire's worker header + team card show a human role title, never
the raw "Genesis <name> <id>" string. vitest on WorkerHero + team card.
