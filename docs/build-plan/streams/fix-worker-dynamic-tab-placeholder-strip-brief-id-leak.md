# fix-worker-dynamic-tab-placeholder-strip-brief-id-leak

**Owner**:
**Authored against**: origin/main `354de50a0c5fdf561125c7e931699fe94daa326d` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-dynamic-tab-placeholder-strip-brief-id-leak`.
**Source**: e2e-ux-audit T0739 — `docs/skycot/ux-audits/2026-05-27-0739/scene-1-and-2-claim-1-and-2.md` F4 + F11.

🟡 **P2 — Six of seven non-Overview worker tabs render placeholder text that LEAKS internal engineering brief-IDs ("Activity — coming in E4-c.", "Memory — coming in E4-d.", etc.). Engineering vocabulary on a customer surface. Compounds with F11 where the "See all in Activity →" link on the Overview card routes directly to one of these brief-ID-leaking placeholder tabs, so even on the locked Overview route the customer is one click away from reading "E4-c".**

## Root cause

`web/app/(app)/workers/[workerId]/[tab]/page.tsx:17-49` is the dynamic per-tab fallback. The `TAB_BRIEFS` map ties each tab to its engineering brief-ID (`E4-b`, `E4-c`, `E4-d`, `E4-e`, `mission-trust-grade-ui`, `web-worker-detail-escalation-and-trust-chart`, `E4-f`) and the panel renders the literal template:

```
{Label} — coming in {brief}.
```

inside a dashed-border placeholder. Only the Overview tab has its own route (not the dynamic one) and renders real content.

`web/components/skycot/worker/RecentActivityCard.tsx:54` links to `/workers/${workerId}/activity` — which IS the dynamic-tab placeholder. So clicking "See all in Activity →" leads from a real-ish list of activity rows directly into a "coming in E4-c." stub. The footer chevron promises a destination that isn't there.

Per `feedback_brief_authoring_preflight_rules` + `feedback_no_theater_in_demo`: engineering brief-IDs should never reach a customer surface. The placeholder copy is shipping internal sprint board language in the product.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F4 [P2] — Six of seven non-Overview worker tabs render "coming in E4-X" placeholder text
- scene-1-and-2-claim-1-and-2 F11 [P2] — "See all in Activity" footer link routes to a placeholder tab

## Fix recipe

1. **Rewrite the placeholder copy** at `web/app/(app)/workers/[workerId]/[tab]/page.tsx:17-49`. New template:
   ```
   {Label} is rolling out in our next update — check back soon.
   ```
   Drop the `{brief}` interpolation entirely. Remove `TAB_BRIEFS` from the file (or rename to `TAB_LABELS` keyed off the tab id, no brief-id field).
2. **Hide or redirect the "See all in Activity →" link** on the Overview card. Two paths:
   - (a) Hide the link until the Activity tab ships. `web/components/skycot/worker/RecentActivityCard.tsx:54` — gate the rendering of the see-all link on a feature flag or simply omit until the dynamic tab has real content.
   - (b) Redirect to `/audit?worker=<id>` — the audit feed with a worker filter. NOTE: requires the audit-list `worker=` query param to be wired (verify via `rg 'worker[=:]' web/lib/api/audit.ts`). If the filter doesn't exist, file a follow-on `fix-audit-list-worker-filter` and pick option (a) for now.
3. **Sweep for other brief-ID leaks in customer surfaces.** `rg -n 'E4-[a-h]|mission-[a-z-]+-ui|web-worker-' web/components/ web/app/(app)/` to find sibling leaks. Per the audit, `WorkerOverviewContent.tsx:74-81` toasts "Add responsibility dialog ships in E4-h" and "Edit schedule ships in E4-h" — covered by separate brief `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.md` (existing). This brief is scoped to the dynamic-tab page only.

## Acceptance criteria

- [ ] `rg 'coming in E4-' web/app/(app)/workers/` returns zero hits.
- [ ] `rg 'coming in mission-' web/app/(app)/workers/` returns zero hits.
- [ ] Dynamic tab pages still render a clean "coming soon" message (no error, no broken layout).
- [ ] The "See all in Activity →" link is either hidden OR routes to a real destination (audit-feed with worker filter), not to the dynamic-tab placeholder.
- [ ] Vitest snapshot pin: dynamic-tab placeholder does NOT contain "E4-" or "mission-" tokens.

## May-touch

- `web/app/(app)/workers/[workerId]/[tab]/page.tsx` (rewrite placeholder + drop brief-id map)
- `web/components/skycot/worker/RecentActivityCard.tsx` (hide or redirect see-all link)
- `web/app/(app)/workers/[workerId]/[tab]/__tests__/page.test.tsx` (snapshot pin)

## Out of scope

- The Activity tab implementation itself (brief `E4-c` continues to own that scope)
- The duty-toggle edit-click toast brief-id leak (covered by `fix-worker-duty-toggle-fake-mutation-and-edit-toast-brief-id-leak.md`)
- The audit-list `worker=` query filter (file `fix-audit-list-worker-filter` if option b chosen)

## Effort

XS — ~30-50 LOC across SPA + pin. ~30-60 minutes. P2 because every tab click during a Scene-1 tabs sweep leaks brief-IDs.
