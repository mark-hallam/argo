# fix-policy-pages-sla-maintenance-offline-missing-metadata

**Owner**: claude.
**Authored against**: origin/main `74077bf676` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-policy-pages-sla-maintenance-offline-missing-metadata`.
**Source**: ux-audit-2026-05-26-2214 `whole-site-mvp-claims-coverage.md` F7 [P2].

🟢 **P2 — Three policy/status pages — `/sla`, `/maintenance`, `/offline` — don't export `metadata`, so the browser tab reads the marketing-default `"SKYCOT — Hire your first AI teammate"` instead of a page-specific title. Each title is one line of code. Small polish, low risk.**

## Root cause

The three pages at `web/app/sla/page.tsx`, `web/app/maintenance/page.tsx`, and `web/app/offline/page.tsx` don't export a `metadata` object, so the title falls through to the root layout's marketing-default `"SKYCOT — Hire your first AI teammate"`.

DOM evidence:

```
curl -s http://localhost:3000/sla         → <title>SKYCOT — Hire your first AI teammate</title>
curl -s http://localhost:3000/maintenance → <title>SKYCOT — Hire your first AI teammate</title>
curl -s http://localhost:3000/offline     → <title>SKYCOT — Hire your first AI teammate</title>
```

## Findings this brief closes

- whole-site-mvp-claims-coverage F7 [P2] — `/sla`, `/maintenance`, `/offline` inherit site-default title

## Fix recipe

Add a `metadata` export to each of the three pages:

**`web/app/sla/page.tsx`** — add at top:

```tsx
import type { Metadata } from "next";

export const metadata: Metadata = { title: "SLA" };
```

**`web/app/maintenance/page.tsx`** — `export const metadata: Metadata = { title: "Maintenance" };`

**`web/app/offline/page.tsx`** — `export const metadata: Metadata = { title: "Offline" };`

The global `template: "%s · SKYCOT"` in the root layout will compose each as `"SLA · SKYCOT"`, `"Maintenance · SKYCOT"`, `"Offline · SKYCOT"`.

NB: if any of these pages start with `"use client";`, the `metadata` export must live in a sibling `layout.tsx` (Next.js disallows metadata in client components). Pattern:

```tsx
// web/app/sla/layout.tsx
import type { Metadata } from "next";
export const metadata: Metadata = { title: "SLA" };
export default function SLALayout({ children }: { children: React.ReactNode }) {
  return children;
}
```

Verify with `grep -l "use client" web/app/sla/page.tsx web/app/maintenance/page.tsx web/app/offline/page.tsx`.

## Acceptance criteria

- [ ] `curl -s http://localhost:3000/sla` → `<title>SLA · SKYCOT</title>`
- [ ] `curl -s http://localhost:3000/maintenance` → `<title>Maintenance · SKYCOT</title>`
- [ ] `curl -s http://localhost:3000/offline` → `<title>Offline · SKYCOT</title>`
- [ ] `pnpm --dir web exec tsc --noEmit` green
- [ ] No new vitest spec strictly required (3 trivial metadata additions)

## May-touch

- `web/app/sla/page.tsx` OR `web/app/sla/layout.tsx` (~5 LOC)
- `web/app/maintenance/page.tsx` OR `web/app/maintenance/layout.tsx` (~5 LOC)
- `web/app/offline/page.tsx` OR `web/app/offline/layout.tsx` (~5 LOC)

## Out of scope

- The 7 in-app routes inheriting "Approvals · SKYCOT" (covered by `fix-spa-page-titles-metadata-leak-default-approvals`)
- The 5 onboarding step pages (covered by `fix-onboarding-step-pages-tab-title-via-layout-metadata`)
- The 6 `/for/*` double-suffix issue (covered by `fix-marketing-for-titles-double-suffix-skycot`)
- The 4 settings sub-pages (covered by `fix-settings-sub-pages-title-suffix-consistency`)

## Effort

~15 LOC across 3 files. ~15 min. P2 polish; bundle with the other title-leak briefs in one PR if convenient.
