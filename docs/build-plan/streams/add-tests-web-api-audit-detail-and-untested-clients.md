# add-tests-web-api-audit-detail-and-untested-clients

Authored against: origin/main 55814da6ae602dc4859ee115f95b953bfc504949 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh add-tests-web-api-audit-detail-and-untested-clients

## Goal

Add vitest coverage to the highest-value untested web API-client modules,
starting with audit-detail.ts (backs the hash-chained-audit drill-in — one of
the 4 marketing differentiators) and the next 4 biggest untested clients.

## Why now (test-health audit 2026-06-10, F5)

73 modules in web/lib/api/, only 44 have a .test.ts → 29 untested (verified
2026-06-10). The biggest/most-visible untested ones: audit-detail.ts (~362
LOC, the audit drill-in client — ZERO tests today), worker-settings.ts (~433),
verticals.ts (~344), worker-activity.ts (~337), use-realtime-voice.ts (~284).

## Fix

Add a focused .test.ts per module (start with audit-detail.ts), mirroring the
existing client-test pattern (e.g. web/lib/api/skills.test.ts): assert (a) the
demo-live gate (real session hits the gateway, anonymous-dev uses fixtures),
(b) the happy-path shape, (c) error/404 handling, (d) for audit-detail, that
the hash-chain fields surface and a tampered/missing row is handled. Do NOT
change the client code unless a test reveals a real bug (then fix + note it).

## May touch

- web/lib/api/audit-detail.test.ts (NEW) + the other top-4 .test.ts files
- the client modules ONLY if a test surfaces a real bug

## Acceptance / verifier

1. `pnpm --dir web vitest run` green incl. the new specs.
2. audit-detail.test.ts asserts the demo-live gate + hash-chain field shape.
3. Untested-client count drops (73/44 → 73/49 for the top-5).

## Size / owner

~5 × 40-100 LOC test files. Owner: claude. 🟡 P2 — closes the worst coverage
cold spot (a differentiator surface with zero tests).
