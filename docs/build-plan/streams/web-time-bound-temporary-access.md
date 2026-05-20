# web-time-bound-temporary-access — invite + capability expiry for temporary access (auditor: 30-day read-only)

**Owner**: claude. **Sizing**: ~200-300 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-time-bound-temporary-access`.
**Priority**: P2 — enterprise auditors, external consultants, and short-term contractors all need read-only access that expires automatically. This brief adds expiry to the invite flow + capability gate.

## Scope

### 1. Invite-member modal — Access-expires field

The existing /settings/members invite-member modal gets a new field:

- **Access expires** — radio:
  - Never (default — for permanent staff)
  - 7 days
  - 30 days
  - 90 days
  - Custom date (date picker → defaults to 30 days from today)
- Helper text under the radio: "Time-bound access is useful for auditors, external consultants, or short-term contractors. The user will lose access automatically on the expiry date."

When the invite is sent:
- POST to gateway `/api/v1/members/invite` includes `{ ..., expires_at: ISO date | null }`.
- The new member's record carries `expires_at`.

### 2. Member list — Expiry pill

Members with an `expires_at` show a pill in the row:
- Green ("Expires in 14 days") if >7 days remaining.
- Amber ("Expires in 3 days") if ≤7 days.
- Red ("Expired 2 days ago") if past expiry (and the row is grayed out + login is blocked).
- No pill for permanent members.

### 3. Edit Member modal — Adjust expiry

Reuse the modal from `web-cohort-scoped-access` (or its predecessor); add the Access-expires field alongside Role + Cohort scope. Editing it PATCHes the member record.

### 4. Expired session banner

When a member with `expires_at` in the past tries to sign in:
- Magic-link / SSO flow short-circuits with a friendly page: "Your SKYCOT access expired on <date>. Contact your workspace admin if you need to renew."
- Brand-restrained register (matches existing auth-error pages).

When a member with `expires_at` close to now (within 7 days) is signed in:
- Render a slim amber banner at the top of every page: "Your access expires in N days. Ask your admin to extend if needed." with a Dismiss button (persists for 1 day in localStorage).

### 5. Auto-revoke job

Document (in the brief, not necessarily implemented in this brief) the gateway job that runs nightly to:
- Mark members with `expires_at < now()` as `revoked_at = expires_at`.
- Invalidate any active session cookies for those users.

The web brief's job is to surface the UX cleanly; gateway-side auto-revoke is a follow-up if not already implemented.

### 6. Audit entries

Each time-bound action emits audit:
- "Invite sent: <email>, access expires <date>"
- "Access extended: <member>, new expiry <date>"
- "Access expired: <member>" (emitted by the auto-revoke job)

### 7. Activity history for time-bound members

In the existing `/audit` page (or per-member activity view if it exists), time-bound members' actions render with a small clock icon next to their name + tooltip "Time-bound access — expired <date>".

## Acceptance criteria

- [ ] Invite-member modal includes Access-expires radio (Never / 7 / 30 / 90 / Custom).
- [ ] /settings/members row shows expiry pill (green / amber / red) based on remaining time.
- [ ] Edit Member modal allows adjusting expiry.
- [ ] Expired-member login attempts route to a friendly expired-access page.
- [ ] Active sessions within 7 days of expiry render a slim banner with Dismiss (1-day persist).
- [ ] Audit feed shows invite-sent / access-extended / access-expired entries.
- [ ] Time-bound members' actions in /audit render with clock icon + tooltip.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Gateway-side nightly auto-revoke job (separate brief if not already implemented).
- Per-resource time-bound access (e.g. "only this workflow expires") — v1.x.
- Renewal-request workflow from the member's side — v1.x.
- Bulk-set expiry across multiple members — v1.x.
