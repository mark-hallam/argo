# fix-brand-components-page-event-handler-prerender-error

**Owner**: claude or codex (fast-mode candidate, ~15-30 LOC).
**Authored against**: origin/main `bded3714e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-brand-components-page-event-handler-prerender-error`.
**Source**: PR #2384 body footnote — `pnpm build` fails on `/(app)/brand/components` page during static prerender with `Event handlers cannot be passed to Client Component props.`. Pre-existing on `origin/main` `ded274874`. Confirmed by this brief author 2026-05-26 against `bded3714e`.

🟡 **P2 — broken Next.js production build.** The `web/app/(app)/brand/components/page.tsx` design-system showcase page is a Server Component (no `"use client"` directive) but passes inline `onSubmit={() => {}}` callbacks to `<SignupForm>` (lines that look like `<SignupForm onSubmit={() => {}} />` × 3). Next.js 16 rejects this at prerender time. Effect: `pnpm build` fails, blocking any production-mode SSR / static export of the web app.

## Root cause

`web/app/(app)/brand/components/page.tsx:1-15` imports a Server Component but uses client-only props at:

```tsx
<Section title="SignupForm">
  <SignupForm onSubmit={() => {}} />
  <SignupForm onSubmit={() => {}} state="loading" />
  <SignupForm onSubmit={() => {}} state="success" />
</Section>
```

`SignupForm` is a Client Component, and inline function props can't be serialised across the server→client boundary.

## Fix recipe (~15-30 LOC, single-file)

Pick ONE of:

**Option A (preferred — minimal)**: Add `"use client";` to the very top of `web/app/(app)/brand/components/page.tsx`. Whole page becomes a Client Component. Acceptable because this is an internal design-system showcase, not a perf-sensitive marketing page. **~1 LOC**.

**Option B (if A is unacceptable)**: Extract the three SignupForm examples into a small sibling client component (e.g. `web/app/(app)/brand/components/SignupFormExamples.tsx` with `"use client";` at the top), import + render it from the page. Page stays a Server Component. **~25-40 LOC**.

Acceptance criteria:
- [ ] `cd web && pnpm build` exits 0 (or at minimum, no longer fails on `/(app)/brand/components`).
- [ ] `curl -sf http://localhost:3000/brand/components` renders the page (run dev server, click through to confirm all sections render).
- [ ] No other test changes — purely a build/prerender fix.

## May-touch

- `web/app/(app)/brand/components/page.tsx` (Option A)
- OR: `web/app/(app)/brand/components/page.tsx` + `web/app/(app)/brand/components/SignupFormExamples.tsx` NEW (Option B)

## Out of scope

- Other pre-existing vitest flakes (separate brief: `fix-pre-existing-vitest-flakes-bundle`).
- Any restructure of the `Section` helper or page layout — fix the prerender error only.
