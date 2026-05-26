# fix-onboarding-step-pages-tab-title-via-layout-metadata

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-step-pages-tab-title-via-layout-metadata`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/whole-site-mvp-claims-coverage.md` F1/F9 [P1].

🟠 **P1 — All 5 onboarding step pages (`/onboarding/welcome|workspace|connect-provider|pick-vertical|first-hire`) are client components without `metadata`, so all five show site-default `"SKYCOT — Hire your first AI teammate"` in the browser tab. Scene 3 records every step — tab reads the same generic string across all five.**

## Root cause

`web/app/(setup)/onboarding/{welcome,workspace,connect-provider,pick-vertical,first-hire}/page.tsx` all start with `"use client";` and have no `metadata` export. Per Next.js rules, client components cannot export metadata. The pages fall through to the root `web/app/layout.tsx` default title.

Fix: server-component layouts CAN set metadata for client-component pages. Add a `layout.tsx` per step.

## Findings this brief closes

- whole-site-mvp-claims-coverage F1 [P1] / F9 [P1] — All 5 onboarding step pages leak default browser tab title

## Fix recipe

1. **Create a `layout.tsx` per step** at `web/app/(setup)/onboarding/{step}/layout.tsx`:
   ```tsx
   import type { Metadata } from "next";
   import type { ReactNode } from "react";
   
   export const metadata: Metadata = { title: "Welcome · Onboarding" };
   
   export default function Layout({ children }: { children: ReactNode }) {
     return <>{children}</>;
   }
   ```

2. **Per-step titles**:
   - `/onboarding/welcome/layout.tsx` → "Welcome · Onboarding"
   - `/onboarding/workspace/layout.tsx` → "Workspace · Onboarding"
   - `/onboarding/connect-provider/layout.tsx` → "Connect provider · Onboarding"
   - `/onboarding/pick-vertical/layout.tsx` → "Pick vertical · Onboarding"
   - `/onboarding/first-hire/layout.tsx` → "First hire · Onboarding"

3. The global template `title.template = "%s · SKYCOT"` adds the brand suffix automatically — each tab reads e.g. "Welcome · Onboarding · SKYCOT".

4. **Alternative if title-template-doubles-up**: set `title: { absolute: "Welcome · Onboarding · SKYCOT" }` to bypass the template.

5. **Add a Playwright spec** at `web/playwright/onboarding/per-step-tab-titles.spec.ts`:
   - For each of the 5 steps, navigate + assert `document.title` is unique + matches per-step expectation

## Acceptance criteria

- [ ] Each of the 5 onboarding step pages has a unique, per-step browser tab title
- [ ] No tab reads the default "SKYCOT — Hire your first AI teammate" during Scene 3
- [ ] Brand suffix `· SKYCOT` appears exactly once per title
- [ ] New Playwright spec passes

## May-touch

- `web/app/(setup)/onboarding/welcome/layout.tsx` (NEW)
- `web/app/(setup)/onboarding/workspace/layout.tsx` (NEW)
- `web/app/(setup)/onboarding/connect-provider/layout.tsx` (NEW)
- `web/app/(setup)/onboarding/pick-vertical/layout.tsx` (NEW)
- `web/app/(setup)/onboarding/first-hire/layout.tsx` (NEW)
- `web/playwright/onboarding/per-step-tab-titles.spec.ts` (NEW)

## Out of scope

- Page content (preserve)
- Other tab-title bugs (covered by sibling briefs)
- Browser-tab favicon (separate concern)

## Effort

~30-60 LOC across 5 new files + 1 spec. ~1 hour. P1 because Scene 3 records all 5 steps and the demo recording shows the same generic title on every clip.
