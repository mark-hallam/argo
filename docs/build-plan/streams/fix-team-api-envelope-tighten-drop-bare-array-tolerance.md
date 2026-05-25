# fix-team-api-envelope-tighten-drop-bare-array-tolerance

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-api-envelope-tighten-drop-bare-array-tolerance`.
**Source**: e2e-ux-audit (scene-1-and-2-claim-1-and-2) findings F14 + F15 — `docs/skycot/ux-audits/2026-05-26-0820/scene-1-and-2-claim-1-and-2.md`.

🟢 **P3 — `/api/v1/team` SPA fetcher tolerates `Worker[] | { workers: Worker[] }` envelope shapes. Tolerance was for old gateway builds; gateway now returns object envelope. The bare-array branch is dead code that obscures the contract.**

## Root cause

`web/lib/api/team.ts:64-84` handles two envelope shapes:
- Bare array: `[Worker, Worker, ...]`
- Object envelope: `{ workers: [Worker, Worker, ...], total_count: N }`

The comment in the file says "temporary tolerance for old gateway builds." Now the gateway returns the object envelope (per the curl evidence in the audit). The array branch is dead.

Two coupled cleanups:

**F14 — drop the bare-array tolerance**:
- The tolerance forces a runtime discriminator check on every fetch
- The type carries `Worker[] | { workers: Worker[] }` which obscures the real shape downstream
- Dead code that the next refactor might preserve out of caution

**F15 — document canonical audit-action-detail endpoint**:
The audit's F15 (related, P3) notes two audit endpoint candidates 404 (`/api/v1/audit/actions`, `/api/v1/audit/events`); only `/api/v1/audit` (returns `{"items":[]}`) responds. The client-side `useAuditDetail` would call action-detail at some path; the surface-pinning ledger doesn't make the canonical path obvious. With empty audit data the drill-in flow can't actually be tested end-to-end. This is documentation, not code.

Bundle both because they're both small cleanup items in the API contract surface.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F14 [P3] — `/api/v1/team` envelope handler tolerates bare-array OR object; defensive but obscures contract
- scene-1-and-2-claim-1-and-2 F15 [P3] — Two audit endpoint candidates 404 (`/audit/actions`, `/audit/events`); canonical path undocumented

## Fix recipe

**For F14** (tighten team envelope):
1. Touch `web/lib/api/team.ts:64-84`.
2. Verify via cargo + curl that the live gateway ALWAYS returns the object envelope `{workers, total_count}`. If true:
   - Drop the bare-array branch
   - Tighten the type to `TeamResponse = { workers: Worker[]; total_count: number }`
3. Add a surface-contract test at `web/lib/api/team.test.ts` that asserts:
   - Gateway response shape matches `TeamResponse`
   - Bare-array input throws a clear `InvalidTeamEnvelope` error (don't silently accept it any more)
4. If any caller (search for `useTeam(`) was relying on the bare-array branch, surface that as a follow-up. None expected.

**For F15** (document canonical audit paths):
1. Add a comment block at the top of `web/lib/api/audit-detail.ts`:
   ```ts
   /**
    * Audit endpoint canonical paths (as of 2026-05-26):
    *   List feed:        GET  /api/v1/audit              → { items: AuditRow[] }
    *   Action detail:    GET  /api/v1/audit/<id>         → AuditDetail
    *   Replay verdict:   POST /api/v1/audit/<id>/replay  → GatewayReplayBody
    *
    * Deprecated / 404:
    *   /api/v1/audit/actions  — never existed; do not use
    *   /api/v1/audit/events   — never existed; do not use
    */
   ```
2. Add a brief mention in `docs/skycot/audit-api.md` (or create the file if absent) listing the same canonical paths.

## Acceptance criteria

**F14**:
- [ ] `web/lib/api/team.ts` type `TeamResponse` is the single shape `{ workers: Worker[]; total_count: number }`.
- [ ] Bare-array tolerance code removed.
- [ ] New test in `team.test.ts` asserts gateway-shape input parses correctly + bare-array input throws.
- [ ] `pnpm typecheck` clean; no callers broken.

**F15**:
- [ ] Comment block at top of `web/lib/api/audit-detail.ts` documents the canonical paths.
- [ ] `docs/skycot/audit-api.md` exists (or has a section) listing the same.
- [ ] Surface contract test (likely existing) is updated to pin the canonical paths.

## May-touch

- `web/lib/api/team.ts` (tighten type, drop tolerance)
- `web/lib/api/team.test.ts` (NEW or extended)
- `web/lib/api/audit-detail.ts` (header comment)
- `docs/skycot/audit-api.md` (NEW section or new file)
- `web/tests/surface_contract*.ts` (if existing surface pins need refresh)

## Out of scope

- Gateway-side audit endpoint additions (out of scope; canonical paths are accurate)
- Worker envelope tightening (separate brief if needed)
- Renaming `total_count` to `totalCount` or similar (out of scope; preserve gateway contract)

## Effort

~40-80 LOC including types + tests + doc. ~1 hour. Tech-debt cleanup; bundle when convenient.
