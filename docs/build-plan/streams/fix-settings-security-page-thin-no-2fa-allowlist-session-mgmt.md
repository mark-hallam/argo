# fix-settings-security-page-thin-no-2fa-allowlist-session-mgmt

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-settings-security-page-thin-no-2fa-allowlist-session-mgmt`
**Source:** ux-audit-2026-05-27-2351

🟢 **P3 — `/settings/security` shows only a SAML SSO configure card. No 2FA setup, no IP allowlist, no session-management table, no audit-retention policy. Reads light against the "enterprise-grade audit-first" pitch.**

## Root cause

> No 2FA setup, no IP allowlist, no session-management table, no audit-retention policy. For an "enterprise-grade audit-first" pitch, this page reads light. Not a demo blocker (Scene 6 doesn't visit it), but worth knowing.
> — `whole-site-mvp-claims-coverage.md` F20

Per `feedback_enterprise_grade_defaults` (operator-locked), v1 implementations may be permissive behind tagged seams, but the SURFACE should be enterprise-shaped. `/settings/security` currently fails that bar — it shows only SAML SSO and nothing else.

Per `feedback_no_theater_in_demo`, the page should not fabricate working controls. Acceptable approach: show enterprise-shaped surfaces with honest "Available in {tier}" or "Configure with your admin" gating so the surface reads enterprise-credible without faking function.

## Findings this brief closes

- `whole-site-mvp-claims-coverage.md` F20 [P3] — `/settings/security` shows only a SAML SSO configure card

## Fix recipe

1. **Expand the page layout** at `web/app/(app)/settings/security/page.tsx` to render five cards:
   - **SSO** — existing SAML card, kept as-is.
   - **Two-factor authentication** — read-only card describing TOTP support; "Configure" CTA is disabled with tooltip "Per-user 2FA enrollment ships in v1.1" or routes to an external link to enterprise sales.
   - **IP allowlist** — read-only card listing 0 entries + an empty-state "Add IP range" CTA disabled with v1.1 tooltip.
   - **Active sessions** — table with one row (current session); other rows fetched from gateway endpoint if available, else "No additional sessions" empty state.
   - **Audit retention** — card showing the current retention policy (e.g. "All audit rows retained indefinitely on this tenant") + "Configure retention" CTA gated to enterprise.
2. **No fake controls** — every CTA either:
   - Routes to a real (even if minimal) sub-page
   - Is disabled with a clear "ships in vN" or "contact sales" tooltip
   - Opens a sonner toast with honest framing
3. **Component tests** at `web/app/(app)/settings/security/SecurityPage.test.tsx` (NEW) cover the five cards render + the gating language.

## Acceptance criteria

- [ ] `/settings/security` renders 5 cards (SSO + 2FA + IP allowlist + Active sessions + Audit retention) in a 2-column grid (mobile: single column).
- [ ] No fabricated working controls — disabled controls have honest tooltips.
- [ ] Existing SAML SSO card preserved.
- [ ] Component tests pass.
- [ ] PR body cites finding F20.

## May-touch

- `web/app/(app)/settings/security/page.tsx`
- `web/components/skycot/settings/SecurityCard.tsx` (NEW or extend)
- `web/app/(app)/settings/security/SecurityPage.test.tsx` (NEW)

## Out of scope

- Actually implementing 2FA / IP allowlist / session management (post-v1 features)
- SSO failed-to-fetch (covered by `fix-settings-security-saml-status-failed-to-fetch`)
- Settings tab matcher (covered by `fix-settings-tabs-prefix-startswith-double-highlights-workspace`)

## Effort

S — ~150-250 LOC + tests. ~3-4 hours. P3 because no demo impact; surfaces an enterprise polish gap.
