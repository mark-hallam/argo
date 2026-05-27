# fix-audit-row-click-handler-swallows-navigation

**Owner:** claude
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-audit-row-click-handler-swallows-navigation`
**Source:** ux-audit-2026-05-27-2351

🟡 **P1 — Clicking an audit row at `/audit` does not navigate, even though the row is wrapped in `<a href="/audit/action/<id>">`. A captured-but-not-handled click handler swallows the event silently.**

## Root cause

`web/components/skycot/audit/AuditDayGroup.tsx` (and/or the row component it mounts, likely `web/components/skycot/audit/ActivityRow.tsx` or `web/components/skycot/audit/AuditRow.tsx`) renders rows as `<a href="/audit/action/<id>" data-testid="activity-row-<id>">…</a>`. The audit reports confirm:

> Clicking the row container `[data-testid="activity-row-<id>"]` (which is the visible target) does not navigate to `/audit/action/<id>` even though the row IS rendered as an `<a>` with that href. The click is captured silently. Manual `window.location.href='/audit/action/<id>'` does navigate (and lands on the "Action not found" empty state, see F01).
> — `whole-site-mvp-claims-coverage.md` F06

Likely cause is an `onClick={(e) => e.preventDefault()}` or a `onClick` that calls `e.stopPropagation()` on a descendant span/button (e.g. a "copy hash" icon or a status pill) without an early-return for clicks outside that affordance. The Next.js `<Link>` wrapper may also be conflicting with a child interactive element (a11y rule: anchor-inside-anchor or button-inside-anchor breaks click bubbling).

This is masked today by `fix-audit-list-detail-endpoint-id-mismatch` (the destination 404s anyway), but once that ships the click handler will need to actually route. Scene 6's headline beat — "click an audit row, see real provenance" — depends on this.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F06 [P1] — audit-page click on row swallows the navigation

## Fix recipe

1. **Grep for the suspect handlers**: `rg "preventDefault|stopPropagation" web/components/skycot/audit/` and identify which child handler is firing on parent-level row clicks.
2. **Make affordance handlers locally-scoped**: copy-hash buttons, status-pill click handlers, and any nested interactive child should:
   - Stop event propagation only when the click target is the affordance itself (use `e.target === e.currentTarget` or `e.target.closest('button[data-row-affordance]')` checks).
   - Not preventDefault on the parent anchor.
3. **Or — simplify**: make the row container a plain `<div>` and use `next/link`'s programmatic `router.push(href)` in an explicit row-level `onClick` that early-returns when the click originated inside a child `<button>`.
4. **Playwright pin** at `web/playwright/audit/row-click-navigates.spec.ts`:
   - Visit `/audit`.
   - Click the first row container.
   - Assert URL contains `/audit/action/<id>` within 1s.
   - Assert no console errors.
5. **Component test** at `web/components/skycot/audit/AuditRow.test.tsx`:
   - Render with a known id.
   - Fire `click` on the row.
   - Assert the test's `<Link>` mock receives the right `href`.
   - Render with a child status-pill button.
   - Fire click on the button.
   - Assert the row-level `onClick` was NOT triggered (button is scoped).

## Acceptance criteria

- [ ] Clicking an audit row navigates to `/audit/action/<id>` in the browser.
- [ ] Clicking a child affordance (copy-hash, status pill, redaction badge) does NOT navigate — only the affordance fires.
- [ ] Playwright spec passes cold.
- [ ] No regression in keyboard navigation (Enter / Space on a focused row still navigates).
- [ ] PR body cites the audit-report finding.

## May-touch

- `web/components/skycot/audit/AuditDayGroup.tsx`
- `web/components/skycot/audit/ActivityRow.tsx` (or `AuditRow.tsx`)
- `web/components/skycot/audit/AuditRow.test.tsx` (NEW if absent)
- `web/playwright/audit/row-click-navigates.spec.ts` (NEW)

## Out of scope

- Fixing the 404 destination (covered by `fix-audit-list-detail-endpoint-id-mismatch`)
- Audit-row visual design (locked per `project_audit_page_design_locked`)
- Audit feed semantic-html table refactor (covered by `fix-audit-feed-table-semantic-html-not-tabular-screen-reader`)

## Effort

S — ~50-100 LOC + 1 Playwright + 1 component test. ~2-3 hours.
