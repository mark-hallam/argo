# Stream fix-web-audit-feed-envelope — fix /audit feed load failure

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `f3d1c66a6` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-web-audit-feed-envelope`.
🟡 **P1 DEMO-BLOCKER.** `/audit` renders the sidebar + page chrome but shows "The audit feed couldn't load. We hit a problem fetching the audit ledger." Diagnosed: gateway returns `{"items":[...]}` envelope with real audit rows; frontend `fetchAuditFeed` expects type `AuditFeedPage` (likely shape `{rows, ...}` or different envelope key). The mismatch causes the page to throw or hit its error state.

## Scope

Reconcile the audit feed response shape between gateway and frontend. Likely:
- Gateway returns `{items: AuditRow[]}` or similar
- Frontend `AuditFeedPage` type expects different key naming (e.g. `rows`, `entries`, `data`)
- Or the envelope has additional fields the frontend isn't accommodating (pagination cursors, totals)

Pattern template: commit `1d1085aab` (the `/team` envelope unwrap fix).

The demo voiceover at 7:30-8:00 walks through `/audit` ("Every action is here. Worker, skill, status, cost. Even mistakes are traceable."). The page needs to show a populated audit feed — empty state is acceptable for this brief, but the load itself must succeed without erroring.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `web/lib/api/audit.ts` — primary fix surface
- `web/lib/types/audit.ts` — type definitions (`AuditFeedPage`, `AuditRow` if shape changes)
- `web/lib/sample-audit*` — sample data alignment if envelope shapes change
- `web/app/(app)/audit/page.tsx` — only if page-level data shape changes (prefer keeping fix in API layer)

## Files you must NOT touch

- Any `crates/argo-*/**` file — gateway is returning the correct shape; this brief reconciles the frontend
- Other API client files unless sharing helpers
- Other web/app/(app)/ pages

## Acceptance criteria

- [ ] `/audit` loads without "The audit feed couldn't load" error
- [ ] Page renders either populated rows OR a clean empty state (no error UI)
- [ ] `pnpm typecheck` clean
- [ ] `pnpm test` — no regressions; audit-related tests updated for new shape
- [ ] PR body includes the curl response shape that diagnosed the mismatch
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-fix-web-audit-feed-envelope`
- PR title: `web(audit): reconcile feed envelope shape with gateway (fix-web-audit-feed-envelope)`

## Notes for the implementer

- Diagnosed during the 2026-05-24 demo walkthrough. Confirmed gateway returns `{"items": [...]}` with populated audit data (hipaa_compliance_check, etc.).
- The frontend's `AuditFeedPage` type is in `web/lib/types/audit.ts`. Read both ends + reconcile.
- Sibling pattern to commit `1d1085aab` (/team envelope unwrap) — apply the same shape of fix.
