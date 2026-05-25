# fix-worker-detail-charter-object-vs-string-contract-mismatch

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-detail-charter-object-vs-string-contract-mismatch`.
**Source**: e2e-ux-audit Agent 1 findings F5 + F8 + Agent 2 finding F1 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md` + `docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🔴 **P0 DEMO-GATE — investor-narrative destination page crashes for any real hire. Scene 1 click-through is unrecordable.**

## Root cause

Gateway `GET /api/v1/workers/<id>/overview` returns:

```json
{
  "charter": {
    "text": "Drafts blog posts based on briefs and asks for approval before publishing.",
    "version": 1,
    "last_edited_at": "...",
    "last_edited_by": "system:hire-endpoint"
  },
  ...
}
```

SPA-side, `web/lib/api/worker-overview.ts:61` declares `charter: string` and `web/components/skycot/worker/WhatSheDoesCard.tsx:156` renders `{charter}` directly as a child of `<p>`. React throws:

> Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version})

The worker `error.tsx` boundary catches it and renders the full-page 500 shell. The header (breadcrumb + idle badge + tabs) renders, then the body crashes. Independently observed by both Agent 1 (visiting Maya at `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`) and Agent 2 (curl + live SPA walk).

The contract drift is the bug — either gateway returns string OR SPA reads the object. Pick one.

## Findings this brief closes

- Agent 1 F5 [P0] — `/workers/<id>/overview` crashes for any newly-hired worker
- Agent 1 F8 [P0] — gateway returns `charter` as object; SPA expects string (same root cause, called out separately so the contract can be assigned to the right owner)
- Agent 2 F1 [P0] — worker-detail Overview crashes to 500 on real-gateway `charter` object

## Fix recipe

**Preferred — SPA-side, preserves the gateway's richer envelope (the version + last_edited_* fields are useful for the future charter-editor scaffold the audit references):**

1. Update `web/lib/api/worker-overview.ts:61` type:
   ```ts
   charter: { text: string; version: number; last_edited_at: string; last_edited_by: string }
   ```
2. Update `web/components/skycot/worker/WhatSheDoesCard.tsx:156` to render `{charter.text}` and surface `last_edited_at` / `last_edited_by` in a small metadata strip (e.g. "v{version} · edited {relative-time} by {author}").
3. Update `web/components/skycot/worker/WorkerOverviewContent.tsx:139` (`charter={overview.charter}`) — the prop type cascades; verify the consumer signature.

**Alternative (gateway-side, if simpler)**: gateway unwraps `charter.text` into the response root and surfaces version+metadata as siblings (e.g. `charter: string` + `charter_version: number` + `charter_last_edited_at: string`). Find the handler under `crates/argo-cli/src/handlers/gateway/` (likely `routes_workers_overview*.rs`).

## Acceptance criteria

- [ ] Visiting `/workers/<any-real-worker-id>/overview` against the live gateway renders the page WITHOUT a React error (no 500 overlay).
- [ ] curl-test: `GET /api/v1/workers/<id>/overview` returned shape matches the SPA's declared `OverviewResponse` type (TypeScript compile passes).
- [ ] The `WhatSheDoesCard` renders the charter text + (if SPA-side fix) shows version + last-edited metadata.
- [ ] No regression on Mariah/Priya/Jamal-style fixture-data workers.
- [ ] Playwright or component test pinning the contract shape so the next gateway-shape drift trips CI.

## May-touch

- `web/lib/api/worker-overview.ts` (type declaration)
- `web/components/skycot/worker/WhatSheDoesCard.tsx` (render `.text`, add metadata strip)
- `web/components/skycot/worker/WorkerOverviewContent.tsx` (prop pass-through)
- Or: `crates/argo-cli/src/handlers/gateway/routes_workers_overview*.rs` (if gateway-side fix chosen)

## Out of scope

- `isWorkerTabId` server/client mismatch (separate brief: `fix-worker-detail-tab-helper-server-client-mismatch`)
- Worker personality fallback (separate brief: `fix-worker-personality-and-level-derive-from-real-data`)
- Page title showing UUID (separate brief: `fix-worker-detail-page-title-uuid-fallback`)

## Effort

~30-60 LOC + 1 contract-pin test. ~30-60 min.
