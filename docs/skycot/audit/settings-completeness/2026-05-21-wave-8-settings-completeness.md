# Wave 8 — Settings Tab Completeness Audit

Date: 2026-05-21
Auditor: Wave-8 agent #5
Scope: `~/code/argo/web/app/(app)/settings/**` plus referenced routes
Method: read-only (no edits)

## Pages that actually exist on disk

```
web/app/(app)/settings/page.tsx                  (landing — D top-tabs design)
web/app/(app)/settings/api-keys/page.tsx
web/app/(app)/settings/data-export/page.tsx
web/app/(app)/settings/members/page.tsx
web/app/(app)/settings/security/page.tsx
web/app/(app)/billing/page.tsx                    (top-level route; landing tile links here)
```

Files: 5 settings pages + 1 billing page reached from settings landing.

## Pages claimed by landing tiles but missing (will 404)

The landing `TILES` array in `web/app/(app)/settings/page.tsx` (lines 25-89) lists 9 tiles:

| Tile href                  | Disk presence | `coming: true` flag in landing? |
|----------------------------|---------------|---------------------------------|
| `/settings/workspace`      | MISSING       | yes (gated)                     |
| `/settings/members`        | EXISTS        | no                              |
| `/settings/notifications`  | MISSING       | yes (gated)                     |
| `/settings/security`       | EXISTS        | yes (gated, but exists!)        |
| `/settings/providers`      | MISSING       | yes (gated)                     |
| `/settings/api-keys`       | EXISTS        | no                              |
| `/settings/connectors`     | MISSING       | yes (gated)                     |
| `/billing`                 | EXISTS        | no                              |
| `/settings/data-export`    | EXISTS        | no                              |

The `coming: true` flag renders the tile as `aria-disabled` and not as a `<Link>`, so the missing routes do not actually 404 from the landing page — they are just unclickable. **However**, any deep link, breadcrumb, or sidebar nav that hardcodes those paths *would* 404, and the brief task list mentioned `/settings/profile`, `/settings/billing`, `/settings/notifications`, `/settings/integrations` as expected tabs — none of which exist:

- `/settings/profile` → MISSING (not even in landing tiles)
- `/settings/billing` → MISSING (landing links to `/billing` instead)
- `/settings/notifications` → MISSING (gated tile)
- `/settings/integrations` → MISSING (the landing's `/settings/connectors` covers this concept but doesn't exist either)

The `security` tile is incorrectly flagged `coming: true` in the landing array (line 53) — the page exists and ships SSO. **This is a data-drift bug**: the landing renders Security as "coming soon (v1.x)" while `/settings/security` is a fully-wired, shipped page. Anyone reading the settings landing will think SSO is not yet available.

## Per-tab assessment

### 1. `/settings` (landing) — Score: 7/10

- Real-data wired? N/A (static tile grid).
- Save/Cancel? N/A.
- Validation? N/A.
- Brand register: brand-restrained (Inter + JetBrains Mono eyebrow, gradient-free). Matches the locked D-top-tabs design memory.
- Sub-tabs 404? No 404s from this page (all missing routes are gated). But the data-drift bug above (Security marked `coming: true`) is a real ship-quality miss.
- Gaps: no top-tabs row as the design memory claims — it's actually a 2-column tile grid, not horizontal tabs.

### 2. `/settings/members` — Score: 4/10 — **STUB-ONLY**

- Real-data wired? **No.** `SEED_MEMBERS` (7 hardcoded rows: Sam Harlow, Olivia Chen, Diego Alvarez, Theo Park, Naomi Wright, Aroha Patel, all at `@acmeco.com`) loaded into `useState`. No API client exists — grep on `web/lib/api/` finds zero `members.ts` file, zero references to `/api/v1/members` or `/api/v1/workspace/members`.
- Invite member actually sends invite? **No.** `handleInvite` (lines 45-58) constructs a fake `mbr_invite_<Date.now()>` row and pushes into local state. The comment is explicit: `// Follow-up brief wires the real POST to /api/v1/workspace/members.`
- Edit member persists? **No.** `handleEditSave` (lines 34-43) mutates local state only. Comment: `// Follow-up brief wires the real PATCH to /api/v1/members/<id>.`
- Save/Cancel work? The modal Save/Cancel buttons fire local handlers and close the modal, but server-side nothing happens. Reloading the page wipes invitees and role changes.
- Validation? `InviteMemberModal` collects email + role + cohort scope but does not validate email format, does not check the address is unique, and the disabled state on Send only checks `email.length > 0` (verified by reading lines 26-100 of the modal).
- Brand register: brand-restrained. Matches design lock.
- 404 sub-tabs? Page is single-route. OK.
- **Procurement risk**: an enterprise buyer who clicks "Invite teammate" sees a row appear, but no email is sent. Within one refresh the invitee row is gone. This is the kind of demo that breaks the deal during a buyer-led walkthrough.

### 3. `/settings/security` — Score: 9/10 — **FULLY WIRED**

- Real-data wired? **Yes.** Calls `getSamlConfig()` → `GET /api/v1/auth/saml/config`. Save calls `saveSamlConfig()` → `POST /api/v1/auth/saml/config`. Test calls `POST /api/v1/auth/saml/config/test`. SP metadata download hits `GET /api/v1/auth/saml/metadata`. All 4 endpoints in `web/lib/api/sso.ts` (verified lines 149-178).
- Save/Cancel work? Yes, modal has explicit Save/Cancel/Test buttons with busy states (`saving` | `testing` | `idle`) and proper error surfacing.
- Validation? Save button is gated on `idpEntityId.length > 0 && ssoUrl.length > 0 && certificate.length > 0`. Test button gated on entityId + ssoUrl. Plain-text length checks only — no PEM/URL/UUID format validation.
- Brand register: brand-restrained (Inter + JetBrains Mono mono labels). Gradient used only on primary CTA, matching the brand-restrained register.
- 404 sub-tabs? Single route, no children. Sessions / audit retention / encryption-at-rest sub-sections mentioned in landing blurb are **not present** — only SSO ships.
- Gap: missing the other 3 concerns the landing blurb advertises (sessions, audit log retention, encryption-at-rest). These are likely procurement table-stakes for enterprise.

### 4. `/settings/api-keys` — Score: 8/10 — **WIRED WITH MOCK FALLBACK**

- Real-data wired? **Yes, with fallback.** Calls `GET /api/v1/api-keys`, `POST /api/v1/api-keys`, `DELETE /api/v1/api-keys/<id>` via React-Query hooks. On 404 or network error in non-prod, falls back to `SAMPLE_API_KEYS` mock and tags `source: "mock-offline"` (lines 98-111 of `lib/api/api-keys.ts`).
- Create works? Yes — `createApiKey` returns `CreateApiKeyResponse` with both row + one-time-visible token. The reveal panel shows the token once with a confirm-close gate. Real production behaviour depends on gateway endpoint shipping — the comment in `lib/api/api-keys.ts:6-9` says "Gateway endpoints land later (planned brief `mission-gateway-api-keys-endpoint`)". Without that brief landed, this is mock in production too unless strict-prod env vars are set.
- Revoke works? Yes — DELETE call with optimistic cancel.
- Save/Cancel? Create modal has Submit + Cancel; submit gated on `name.length > 0 && scopes.size > 0`. Revoke uses a confirm dialog.
- Validation? Name + ≥1 scope required. No length cap. Expiry presets are 4 fixed plus custom-date string (custom is a free-text string with no ISO parse — potential bug, didn't deep-trace).
- Brand register: brand-restrained, gradient only on primary "Create" pill. Reveal-once flow is good security UX.
- 404 sub-tabs? Single route. OK.
- Gap: still riding mock fallback in dev because the gateway endpoint is in a "planned brief," not a shipped one. Production strict-mode would throw, which is the right failure mode.

### 5. `/settings/data-export` — Score: 9/10 — **FULLY WIRED**

- Real-data wired? **Yes.** `useExportRequest()` hook drives the state machine (idle → requesting → polling → ready → error/rate_limited). Download calls `apiGet` against the returned `xport.downloadUrl` and base64-decodes into a Blob, then triggers a real browser download via `<a download>`.
- Save/Cancel? N/A — this is a request/poll/download flow, not a config form.
- Validation? The 24-hour rate limit is enforced server-side; client surfaces `rate_limited` status.
- Brand register: uses CSS-var design tokens (`var(--primary)`, `var(--muted-foreground)`) — not bespoke gradient. Brand-restrained.
- 404 sub-tabs? Single route. OK.
- Gap: the design is plain Tailwind / CSS-vars and does not match the locked "Settings page D-top-tabs" design memory (Inter throughout, JetBrains Mono eyebrow, white card on lavender). Visually it looks like an older settings page. **This is a UI drift gap, not a functional one.**

## Tabs the brief listed that are not on disk

| Brief-listed tab          | Status                                          | Score |
|---------------------------|-------------------------------------------------|-------|
| `/settings`               | EXISTS — landing tile grid                      | 7/10  |
| `/settings/profile`       | **MISSING** — would 404 if linked               | 0/10  |
| `/settings/members`       | EXISTS — stub-only                              | 4/10  |
| `/settings/security`      | EXISTS — fully wired (SSO #1891)                | 9/10  |
| `/settings/billing`       | **MISSING** — `/billing` exists at top level    | 0/10 (route 404) — `/billing` itself is 8/10 |
| `/settings/api-keys`      | EXISTS — wired with mock fallback               | 8/10  |
| `/settings/notifications` | **MISSING** — gated in landing, no route        | 0/10  |
| `/settings/integrations`  | **MISSING** — listed as `/settings/connectors` in landing, gated, no route | 0/10  |

## Roll-up

- Fully wired (real API end-to-end, save/cancel, validation): **2** — `/settings/security`, `/settings/data-export`.
- Partial (UI wired but missing major sub-tabs or with mock fallback paths): **2** — `/settings/api-keys` (wired with mock fallback to a gateway endpoint that hasn't shipped), `/settings` (landing exists, data drift on Security tile).
- Stub-only (UI ships but no real persistence): **1** — `/settings/members` (everything is local `useState`).
- 404 / not-on-disk (brief listed but missing): **4** — `/settings/profile`, `/settings/billing`, `/settings/notifications`, `/settings/integrations`.

## Top 3 procurement-impacting gaps

1. **Members tab is a demo, not a feature.** Invite Member appears to work in the UI (a row pops in, modal closes, no error) but no invite email is sent and the row disappears on refresh. A buyer doing a hands-on walkthrough will discover this immediately and lose trust. Comment in the page literally says `Follow-up brief wires the real POST` — the brief hasn't landed.

2. **`/settings/billing` is unreachable except via landing tile detour to `/billing`.** Enterprise buyers click on "Settings → Billing" expecting `/settings/billing`; the URL pattern they'd type or share doesn't exist. The landing tile redirects to `/billing` (a top-level route), which is functional, but URL discoverability and breadcrumb consistency are broken. Worse, the four locked design-language memories (audit / workers / approvals / cohorts / etc.) treat settings as one app surface — billing being outside that surface is inconsistent.

3. **No `/settings/profile` route at all.** Every SaaS settings panel has a user-profile tab (avatar, display name, email, password / magic-link preference, MFA, timezone). SKYCOT has zero. Procurement checklists almost always require "users can manage their own profile" — a "we haven't built it yet" answer is a hard flag at enterprise tier.

Honourable mentions (not in the top-3 but worth flagging):

- The Settings landing flags `/settings/security` as `coming: true` even though SSO shipped via #1891. **This is a one-line bug** with a high-visibility consequence: a buyer reading the landing thinks SSO is not yet available.
- `/settings/notifications` and `/settings/integrations` are both "v1.x" gated, but for enterprise buyers those are mandatory: notification routing for approvals + integration roster (Slack/GitHub/Linear/Email) is half of what they're buying.

## Single highest-impact fix

**Wire `/settings/members` invite + edit to the real gateway endpoints** (or at minimum hide the "Invite teammate" button behind a feature flag until they exist).

Why: Members is the surface buyers test first on a hands-on walkthrough. Right now it visibly fakes a write (invitee row appears, modal closes, no error) but produces no email, no audit row, no persisted change. A buyer who refreshes the page sees the invitee gone. This single behaviour will sink demos that pass every other audit (architecture, design, audit chain). The Page comment + `members-data.tsx` header both explicitly defer this to a follow-up brief — that brief hasn't landed.

The fix surface is small: add `web/lib/api/members.ts` with `inviteMember`, `updateMember`, `removeMember`, `fetchMembers`; either wire those to the existing capability-based `/api/v1/...` routes if they exist gateway-side, or — same fallback pattern as `api-keys.ts` — return `mock-offline` in dev so the demo at least persists across page navigation while the backend brief ships. Either of those takes a single Track-E web brief.

## Brand-register consistency

- `/settings` landing: brand-restrained, Inter + JetBrains Mono eyebrow. OK.
- `/settings/members`: brand-restrained. OK.
- `/settings/security`: brand-restrained. OK.
- `/settings/api-keys`: brand-restrained. OK.
- `/settings/data-export`: uses generic Tailwind CSS-vars, not the locked Inter / JetBrains Mono / lavender card aesthetic of the other settings pages. **UI drift** — not a procurement gap but a brand-consistency gap.

## Notes on operator instructions

The operator's design memory says "Settings page locked — D top-tabs (horizontal tabs + sub-tabs + 2-col content with aside cards), brand-restrained register, no hero search". The landing page does not have a horizontal tabs row — it's a 2-column tile grid. The pages themselves do not render under a shared `layout.tsx` with persistent top-tabs nav. The design lock has not been implemented at the layout level — each settings page is a standalone route with its own header. **This is a layer-2 design drift that the auditor flags but didn't deep-trace.**
