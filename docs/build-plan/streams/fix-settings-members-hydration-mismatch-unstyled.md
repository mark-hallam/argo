# fix-settings-members-hydration-mismatch-unstyled

**Owner**: claude or codex.
**Authored against**: origin/main `d15461a3a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-settings-members-hydration-mismatch-unstyled`.
**Source**: e2e-ux-audit Agent (whole-site-mvp-claims-coverage) finding F3 (`docs/skycot/ux-audits/2026-05-25-2309/whole-site-mvp-claims-coverage.md`).

🔴 **P0 — Settings → Members renders as an unstyled vertical text dump because of a SSR/CSR hydration mismatch on `MemberExpiryPill`.**

## Root cause

`/settings/members` compiles and serves 200, but after hydration React throws:

> A tree hydrated but some attributes of the server rendered HTML didn't match the client properties

…inside `MembersPage` → `MemberRow` → `MemberExpiryPill expiresAt="2026-06-24…"`. The classic locale/timezone trap: SSR formats the date one way (UTC, server locale), client re-formats it another (browser locale). React detects the mismatch, drops the styled-jsx class application, and the rendered DOM is the raw text without card chrome — role badges show only as colored bar backgrounds, no chip outlines, all rows stack vertically.

Confirmed via the audit:
- DOM emits `class="jsx-385397…"` but the corresponding `<style>` block does not appear to apply at runtime.
- Console error: "A tree hydrated but some attributes of the server rendered HTML didn't match the client properties".

Per memory `feedback_styled_jsx_requires_use_client`: pages that use `<style jsx>` blocks fail hard when something upstream of the styled-jsx block trips React out of hydration sync — the styled-jsx class is emitted but the corresponding inline-style insertion never completes.

## Findings this brief closes

- whole-site-mvp-claims-coverage F3 [P0] — `/settings/members` hydration mismatch + unstyled render (whole "Team & roles" page is a vertical text dump)

## Fix recipe

1. **Identify the date formatter site**. Open `web/app/(app)/settings/members/page.tsx` + `MemberRow.tsx` + `MemberExpiryPill` component. Find the `expiresAt` prop and how it's rendered. Most likely `new Date(expiresAt).toLocaleDateString()` or `formatDistance(new Date(expiresAt), new Date())` — both server-vs-client unstable.

2. **Choose ONE of three fixes**:
   - **(a) Pre-format on the server** (preferred): in the server component or `getServerSideProps`/RSC fetcher, format the date to a fixed `en-AU` string and pass `expiresLabel: string` down. Client never re-formats.
   - **(b) Mount guard**: wrap the date render with `const [mounted, setMounted] = useState(false); useEffect(() => setMounted(true), []); return mounted ? <FormattedDate/> : <Placeholder/>`. Suboptimal — shows a flash of placeholder.
   - **(c) `suppressHydrationWarning`** on the pill element. Suppresses the React warning but does NOT fix the styled-jsx insertion failure. Do not use as the primary fix.

3. **Smoke**: open `/settings/members`, verify card chrome paints, role badges have chip outlines, no console hydration error.

4. **Pin with a test** at `web/app/(app)/settings/members/page.test.tsx`: render server-side + hydrate client-side, assert no React hydration warnings (use `expect(console.error).not.toHaveBeenCalledWith(...)` or `react-error-boundary` capture).

## Acceptance criteria

- [ ] `/settings/members` renders with the locked design (card chrome visible, role badges as outlined chips, vertical stacked rows have spacing).
- [ ] DevTools console shows zero "A tree hydrated but some attributes…" errors on the page.
- [ ] The styled-jsx CSS rules actually paint (verify by inspecting an element + confirming the CSS class has applied styles).
- [ ] `page.test.tsx` passes with no hydration warning capture.

## May-touch

- `web/app/(app)/settings/members/page.tsx`
- `web/app/(app)/settings/members/MemberRow.tsx`
- `web/app/(app)/settings/members/MemberExpiryPill.tsx` (or wherever the date renders)
- `web/app/(app)/settings/members/members-styles.ts` (only if styled-jsx block needs splitting)
- `web/app/(app)/settings/members/page.test.tsx` (NEW or extend)

## Out of scope

- Broader settings D-top-tabs design adoption (separate brief `fix-settings-page-implement-d-top-tabs-design`)
- Hydration warnings on other app-shell pages (separate brief `fix-spa-hydration-warnings-theme-and-motion-providers`)
- Multi-tenant settings page redesign

## Effort

~30-80 LOC + 1 test pin. ~1 hour. Demo-blocker AND customer-visible — ship soon.
