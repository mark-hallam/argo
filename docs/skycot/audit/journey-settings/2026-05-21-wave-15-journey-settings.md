# Wave 15 — User-Journey Settings Sweep

Date: 2026-05-21
Auditor: Wave-15 agent #4
Scope: `~/code/argo/web/app/(app)/settings/**` + `~/code/argo/web/app/(app)/billing/**` walked as a buyer journey, not a dimensional audit.
Method: read-only; traced six brief-listed journeys end-to-end through the page, the React hook, the API client, and the gateway handler.
Companion: builds on `audit/settings-completeness/2026-05-21-wave-8-settings-completeness.md` (dimensional pass); this wave asks whether the journey closes.

---

## How this audit differs from Wave 8

Wave 8 asked: "is the page wired?" Wave 15 asks: "as a real buyer running through the journey *invite a colleague → check security → set up billing → mint a key → tune my profile → tune my alerts*, does the journey **finish**?"

A page can pass Wave 8 (UI ships, save button fires) but fail Wave 15 (the side-effect never lands, no email is sent, the artifact doesn't persist across refresh, or the URL is unreachable from the operator's mental model).

---

## Journey 1 — Invite a second member (admin / operator / viewer)

**Route:** `/settings/members`
**Operator intent:** "I just signed up; my CTO needs to log in too."

| Journey step                                                  | Status         | Evidence |
|--------------------------------------------------------------|----------------|----------|
| 1. Land on /settings/members                                  | OK             | Page renders 7 seed members from `SEED_MEMBERS` |
| 2. Click "Invite teammate"                                    | OK             | `InviteMemberModal` opens; email input autofocus |
| 3. Pick role (owner / admin / user / guest)                   | OK             | 4-pill role-grid renders, default `user` |
| 4. Pick cohort scope                                          | OK             | `CohortScopePicker` renders (gated to user / guest) |
| 5. Submit                                                     | **FAILS**      | `handleInvite` (page.tsx:45-58) constructs a fake `mbr_invite_<Date.now()>` row and pushes into local `useState`. Comment: `// Follow-up brief wires the real POST to /api/v1/workspace/members.` |
| 6. CTO receives a magic-link email                            | **FAILS**      | No POST to gateway; no `members.ts` API client exists. No corresponding handler in `crates/argo-cli/src/handlers/gateway/` (verified by grep on `workspace/members`, `invite_workspace`, `members_invite` → zero hits). |
| 7. CTO clicks the link, lands on workspace                    | **N/A**        | Step 6 didn't happen. |
| 8. Refresh — invitee row still shows "invited"                | **FAILS**      | State is local `useState`; refresh resets to `SEED_MEMBERS`. |
| 9. Edit existing member's role / scope                        | **FAILS**      | `handleEditSave` (page.tsx:34-43) mutates local state only. Comment: `// Follow-up brief wires the real PATCH to /api/v1/members/<id>.` |
| 10. Remove a member                                           | **MISSING UI** | No remove action on `MemberRow`. Only edit. |

**Verdict: journey BROKEN.** The buyer's CTO will never receive an invite. The buyer's first hands-on test fails silently — the row appears, the modal closes, no error toast, but no email lands. Confidence destroyed on first refresh.

**Validation gaps:**
- Email format check is `email.includes("@")` only (`InviteMemberModal.tsx:109`). `a@` passes.
- No duplicate-email check against existing members.
- No upper bound on outstanding invites.
- Cohort-picker `null` (= all cohorts) leaks owner-scope to non-owner roles when role downgraded; no UI guard observed in the modal but `CohortScopePicker` does gate via `role` prop.

---

## Journey 2 — Set up SSO + audit retention (security)

**Route:** `/settings/security`
**Operator intent:** "Our security team requires SSO + a documented audit retention policy before deal close."

| Journey step                                          | Status     | Evidence |
|------------------------------------------------------|------------|----------|
| 1. Land on /settings/security                         | OK         | `security/page.tsx:38-194` |
| 2. See current SSO status                             | OK         | `getSamlConfig()` → `GET /api/v1/auth/saml/config`; status pill renders "Not configured" or "Configured: <flavor>" |
| 3. Click "Configure SAML"                             | OK         | `SamlConfigModal` opens |
| 4. Pick IdP flavor (Okta / Azure AD / Custom)        | OK         | Select element renders 3 options |
| 5. Enter IdP entity ID + SSO URL + X.509 cert         | OK         | 3 inputs render; gated `Save` button checks `length > 0` on each |
| 6. Click "Test connection"                            | OK         | `POST /api/v1/auth/saml/config/test` returns `{ ok, message }`; result chip renders |
| 7. Click "Save configuration"                         | OK         | `POST /api/v1/auth/saml/config` upserts |
| 8. Download SP-metadata XML                           | OK         | `<a download="skycot-sp-metadata.xml" href={samlMetadataUrl()}>` |
| 9. View audit-log retention policy                    | **FAILS**  | Landing tile blurb advertises "audit log retention"; page header copy advertises "audit retention"; **no audit-retention UI ships**. `security/page.tsx:18` flags it as "future sub-tab (sessions, audit retention, encryption-at-rest) plug in alongside". |
| 10. Set audit-log retention duration                  | **FAILS**  | Same — no UI, no endpoint, no client. |
| 11. View / revoke active sessions                     | **FAILS**  | Same — no UI for sessions. |
| 12. Configure encryption-at-rest                      | **FAILS**  | Same — no UI; only the marketing `/security` page (`web/app/security/page.tsx`) talks about encryption-at-rest. |

**Validation gaps on the bits that ship:**
- IdP entity ID accepts any non-empty string; no URL/URN validation.
- X.509 PEM textarea length check is `> 0`; no PEM-format validation. A bare string `"not-a-cert"` would pass the client gate and rely on gateway-side validation.
- Optional attribute-mapping JSON textarea has no JSON parse guard client-side.

**Verdict: journey HALF-COMPLETE.** SSO ships fully (the load-bearing enterprise-procurement need); but the three other concerns the landing tile and page copy *promise* (sessions, audit retention, encryption-at-rest) are vapor. A security-team reviewer reading the page header will assume retention is configurable and ask the AE where the control lives. The AE has no answer.

---

## Journey 3 — View plan + change plan + view invoices

**Route:** `/billing` (top-level, NOT `/settings/billing`)
**Operator intent:** "I'm ready to upgrade from Free to Pro."

| Journey step                                                       | Status      | Evidence |
|-------------------------------------------------------------------|-------------|----------|
| 1. Navigate Settings → Billing tile                                | OK (detour) | Landing tile `href: "/billing"` (settings/page.tsx:76) — not `/settings/billing`. URL inconsistency. |
| 2. Type `/settings/billing` directly                               | **FAILS**   | Route doesn't exist; will 404. |
| 3. See current tier + this-month usage                             | OK          | `useQuery({ queryFn: () => apiGet("/api/v1/tenant/usage") })`; gateway handler at `crates/argo-cli/src/handlers/gateway/usage_endpoint.rs` |
| 4. See 3 plan offers (Free / Pro / Enterprise)                    | OK          | `PLANS` constant renders 3 cards |
| 5. Click "Upgrade to Pro"                                          | **CONDITIONAL** | Requires `NEXT_PUBLIC_STRIPE_PRICE_PRO` env var. If unset, button is disabled with no helper text explaining why. |
| 6. Land in Stripe Checkout                                         | OK          | `useCheckout` posts to `/api/v1/billing/checkout`, redirects to `response.url` (`use-checkout.ts:34-49`) |
| 7. Pay, return to `/billing?upgrade=ok`                            | OK          | Success-URL is wired in `use-checkout.ts:39`; landing param not rendered in any tier-change banner though. |
| 8. View invoices                                                   | **FAILS** in product | No in-product invoice list. Manage subscription → opens Stripe Portal session (`/api/v1/billing/portal` → `use-portal.ts:27-49`) which surfaces invoices on Stripe's hosted page. **For most buyers this is acceptable**; for security-paranoid buyers ("I don't want to leave your app to view a Pro invoice") it isn't. |
| 9. Change plan from Pro → Enterprise inside the app               | **FAILS**   | Same — `Manage subscription` redirects to Stripe Portal. There's no in-product downgrade flow. |
| 10. Cancel subscription                                            | OK (Stripe) | Same Portal redirect. |
| 11. Update card                                                    | OK (Stripe) | Same Portal redirect. |

**Validation gaps:**
- Plan card with no env-var price id silently disables the CTA. There's no explanatory tooltip — only a stuck button.
- The success/cancel landing URLs (`?upgrade=ok` / `?upgrade=cancel`) are not read by the billing page; no toast / confirmation banner renders after a successful upgrade. The buyer returns to the same page with no feedback that the upgrade worked. Confidence-eroding.
- "Contact sales" CTA on Enterprise plan opens Stripe Checkout (if a price is set) rather than triggering a sales-lead form / mailto. For real Enterprise that's wrong — usually you don't want self-serve Checkout on a custom-price tier.

**Verdict: journey MOSTLY-COMPLETE for the Free → Pro path.** Stripe-hosted detour for invoices + cancel is acceptable for SaaS norms but is a discoverability gap (buyer must learn that "Manage subscription" is where invoices live). Two real bugs: (a) no post-upgrade confirmation banner, (b) URL inconsistency (`/billing` vs `/settings/billing`). One UX miss: Enterprise tier shouldn't go through self-serve Checkout.

---

## Journey 4 — Create + revoke + rotate API keys

**Route:** `/settings/api-keys`
**Operator intent:** "I'm wiring SKYCOT into our CI pipeline; I need a long-lived bearer token I can rotate quarterly."

| Journey step                                                       | Status     | Evidence |
|-------------------------------------------------------------------|------------|----------|
| 1. Land on /settings/api-keys                                      | OK         | Page renders, `useApiKeys()` fires `GET /api/v1/api-keys` |
| 2. See existing keys (empty if first-time)                         | OK         | `EmptyState` component on `keys.length === 0` |
| 3. Click "Create API key"                                          | OK         | `CreateModal` opens |
| 4. Name the key                                                    | OK         | `name.length > 0` gates submit |
| 5. Pick ≥1 scope                                                   | OK         | `scopes.size > 0` gates submit |
| 6. Pick cohort scope                                               | OK         | (cohort picker shared with members) |
| 7. Pick expiry (never / 30d / 90d / 1y / custom)                  | OK         | 4 presets + custom-date string; **custom is a free-text string with no ISO parse validation** — a `"yesterday"` literal would round-trip through `expiryToIso()` and serialize as-is (`api-keys.ts:138-146`) |
| 8. Submit                                                          | **CONDITIONAL** | `POST /api/v1/api-keys` — **no gateway handler exists** (verified by grep on `api-keys` in `crates/argo-cli/src/handlers/gateway/` → zero hits in production code). Page comment on `lib/api/api-keys.ts:5-9` acknowledges: "Gateway endpoints land later (planned brief `mission-gateway-api-keys-endpoint`)." Dev: falls back to `mockKeyFrom()` and mints a fake `sk_live_<32>` token (`api-keys.ts:114-121`). Strict prod: throws. |
| 9. Reveal full token once                                          | OK (mock)  | `RevealPanel` shows token; confirm-close gate prevents accidental dismissal |
| 10. Copy + paste into CI env                                       | OK         | Browser clipboard via the panel UI |
| 11. After 90 days: rotate the key                                  | **FAILS**  | **No rotate endpoint, no client function, no UI affordance.** Verified by grep on `rotate` in `api-keys.ts` + `use-api-keys.ts` + page → zero hits. Rotation workflow is: revoke old, create new, swap CI env var manually. Not a "rotate" affordance. |
| 12. Revoke a key                                                   | OK (mock)  | `RevokeDialog` confirms; `DELETE /api/v1/api-keys/<id>`; falls back to `{ ok: true }` envelope in dev. |

**Validation gaps:**
- Custom expiry is unguarded — a string like `"2026-99-99"` round-trips.
- No name length cap; "Untitled key" fallback exists for empty names but no UI for over-long names.
- No scope-collision warning when picking conflicting scopes (e.g., `read:audit` + `write:audit` — both fine, but if the buyer picks `write:workers` thinking it implies `read:workers`, there's no helper).
- Token reveal panel does not lock until copied — buyer could dismiss without copying and lose the token forever. (Though confirm-close prevents this.)

**Verdict: journey COMPLETE-IN-DEMO, FAILS-IN-PROD without gateway brief.** Create + revoke work end-to-end against the mock fallback so the demo doesn't break. Rotation is wholly absent — for any buyer with key-rotation in their security policy, this is a procurement objection. Custom expiry validation gap is a small bug.

---

## Journey 5 — Change avatar + name + email + 2FA (profile)

**Route:** `/settings/profile`
**Operator intent:** "I want to change my display name; I also want to enable 2FA for my owner account."

| Journey step                            | Status     | Evidence |
|----------------------------------------|------------|----------|
| 1. Land on /settings/profile            | **FAILS**  | Route does not exist on disk. Verified by `ls web/app/(app)/settings/` — no `profile/` directory. Will 404 if linked. |
| 2. Click on user avatar in sidebar      | **FAILS**  | No avatar dropdown exists; no profile-edit affordance found. (`grep avatar` on web/app/(app) returns 0 production hits.) |
| 3. Change display name                  | **FAILS**  | No UI; no `/api/v1/me` or `/api/v1/auth/me` endpoint in the web client (verified by grep) |
| 4. Change email                         | **FAILS**  | Same |
| 5. Change avatar                        | **FAILS**  | Same |
| 6. Enable 2FA / TOTP                    | **FAILS**  | No 2FA / TOTP / MFA UI anywhere in web app (verified by grep on `2FA`, `two-factor`, `mfa`, `totp` → zero production hits) |
| 7. Change magic-link preference         | **PARTIAL** | Only via `/settings/security` SSO modal toggle `allowOwnerBypass` — owner-level workspace setting, not personal-account preference |

**Verdict: journey DOES NOT EXIST.** Every SaaS settings panel has a personal-profile tab. SKYCOT has none. The only thing close is the "allow owner bypass" toggle inside the SAML modal, which is a workspace-level setting not a personal one. Procurement checklists almost always require *"users can manage their own profile + enable MFA"* — both "no" answers are red flags at enterprise tier.

---

## Journey 6 — Set notification preferences per channel + per worker

**Route:** `/settings/notifications`
**Operator intent:** "I want approvals from Worker A on Slack but Worker B on email; weekly digest only on Mondays."

| Journey step                                       | Status     | Evidence |
|---------------------------------------------------|------------|----------|
| 1. Land on /settings/notifications                 | **FAILS**  | Route does not exist on disk. Landing tile flags it `coming: true` so it's not clickable from the landing, but a deep link, breadcrumb, or external doc that points to `/settings/notifications` will 404. |
| 2. Set per-channel preference (Slack vs email)     | PARTIAL    | Workspace-level preference does not exist. Per-worker preference DOES exist at `/workers/<id>/settings` → `ChannelsSection` (`web/components/skycot/worker/settings/ChannelsSection.tsx`) — toggles each kind, picks one primary. **But this is per-worker, accessed from inside a worker detail page, not centrally.** |
| 3. Set per-worker preference                       | OK (per-worker) | Same `ChannelsSection`; data shape is `ChannelsConfig` in `lib/api/worker-settings.ts`. |
| 4. Set quiet hours                                 | OK (per-worker) | `QuietHoursSection` in `components/skycot/worker/settings/`. |
| 5. Set workspace-level digest cadence (D / W / M)  | **FAILS**  | No UI for this at any level. |
| 6. Set escalation channel ladder                   | OK (per-worker) | `Escalation` sub-tab on worker detail; not a workspace setting. |
| 7. Mute a worker entirely                          | **PARTIAL** | Workers can be set to "off" via `ChannelsSection` (toggle all channels off) but no central pause-all toggle. |
| 8. Set notification rules across all workers       | **FAILS**  | No surface for workspace-wide notification policy. |

**Verdict: journey BROKEN at the workspace level.** Per-worker channel + quiet-hours + escalation settings DO exist (good!), but they're scattered across each worker's detail page. A buyer who wants to set workspace defaults ("default: Slack for everyone, with email fallback after 30min") has to repeat the config N times — once per worker. The `/settings/notifications` route the landing tile advertises does not exist.

---

## Top 3 incomplete settings paths (buyer-impact ranked)

### 1. Invite-teammate journey (Journey 1) — silently fails

A buyer's first hands-on test is "add my colleague." The current flow opens a modal, accepts the invitee, closes the modal, shows the row, and produces no side effect: no email is sent (no gateway handler exists), no audit row is appended, no persistence (refresh wipes the row). This is the kind of demo break that ends the deal mid-walkthrough. The follow-up brief that wires the real POST has not landed; the page comment explicitly defers it.

**Fix surface:** Author `mission-gateway-workspace-members-endpoint` (POST + PATCH + DELETE + GET `/api/v1/workspace/members`), wire `web/lib/api/members.ts` with mock-offline fallback (same pattern as `api-keys.ts` / `team.ts` / `notifications.ts`), and connect `handleInvite` / `handleEditSave` to the client. ~200 LOC web brief + matching ~250 LOC gateway brief.

### 2. Profile + 2FA journey (Journey 5) — wholly absent

There is no `/settings/profile` route. There is no avatar dropdown in the sidebar. There is no 2FA / MFA / TOTP setup anywhere. Procurement checklists require both. The closest thing — the "allow owner bypass" toggle inside the SAML modal — is a workspace setting, not a personal one. Enterprise buyers reading the table-of-contents page see this and ask, "Where do users change their name?" and the answer is "Nowhere." Red flag.

**Fix surface:** Add `/settings/profile/page.tsx` with avatar upload + display-name + email-change + magic-link-vs-OAuth preference, plus 2FA setup (TOTP via QR + recovery codes). Pair with `/api/v1/auth/me` GET + PATCH endpoints. Roughly two briefs (one web, one gateway) of moderate size.

### 3. Notification-policy journey (Journey 6) — fragmented across workers

Per-worker channel/quiet-hours/escalation settings exist and are well-built, but there's no workspace-level notification policy and no `/settings/notifications` route despite the landing tile advertising one. A buyer who wants to configure defaults once (instead of N times across N workers) has nowhere to do that. Today they can only configure each worker individually, which scales poorly the moment they hit 5+ workers.

**Fix surface:** Add `/settings/notifications/page.tsx` with workspace defaults (default channel, quiet-hours, digest cadence), plus a "Per-worker overrides" link out to each worker. Backend brief authors `/api/v1/workspace/notification-policy` GET + PATCH.

---

## Cross-cutting issues found while walking the journeys

1. **URL inconsistency: `/billing` vs `/settings/billing`.** The settings landing tile sends users to `/billing`. A buyer who types `/settings/billing` (the obvious mental model) hits a 404. Either move `/billing` under `/settings/billing` or add a redirect.

2. **`coming: true` data drift on Security tile (Wave-8 finding still present).** Landing tile marks `/settings/security` as v1.x even though the page exists and ships SSO end-to-end. One-line bug, high-visibility consequence: a buyer reading the landing thinks SSO is not yet available.

3. **No post-upgrade confirmation banner on `/billing`.** Successful Stripe Checkout returns to `/billing?upgrade=ok`. The query param is set but never rendered as a toast / banner / tier-change confirmation. The buyer pays, gets dumped back on the same page, with no in-app proof the upgrade worked until they see the next month's invoice.

4. **Custom expiry strings on API-keys round-trip unvalidated.** `expiryToIso()` returns the custom string as-is if it doesn't parse a preset (`api-keys.ts:138-146`). A buyer entering `"yesterday"` would see no error and the gateway would receive the literal string.

5. **No remove/delete affordance on Members.** Only edit. To deprovision a leaving employee the operator has to wait for the (not-yet-shipped) PATCH endpoint and then change role to a tier with no access — but there's no "Remove" button anywhere. Real deals require offboarding.

6. **No central notification preferences surface.** Per-worker settings exist (good); workspace defaults don't (bad).

---

## Brand-register / journey-feel consistency

- `/settings`, `/settings/members`, `/settings/security`, `/settings/api-keys`: consistent brand-restrained register (Inter + JetBrains Mono eyebrow + lavender card). Matches the locked design memory.
- `/settings/data-export`: uses generic Tailwind / CSS-vars — visually drifts from the other four. Wave-8 already flagged.
- `/billing`: visually a totally different page — uses `Card` / `Button` UI primitives, no eyebrow, no lavender card. Looks like a different app. **Journey-killing if a buyer is mid-walkthrough**, because they'll feel the seam. Either re-style `/billing` to the settings register or move it under `/settings/billing` with a layout that re-skins it.

---

## Verdict — can a real customer self-serve onboarding in settings?

**No.** A real enterprise customer cannot complete onboarding through settings without help from sales/support today. Five of the six core journeys are blocked or partial:

| Journey                                  | Buyer-complete? |
|------------------------------------------|-----------------|
| 1. Invite teammate                       | **NO** — silent fake-write |
| 2. SSO + security policy                 | **PARTIAL** — SSO works, retention/sessions/encryption vapor |
| 3. View plan + change plan + invoices    | **YES** (with Stripe-Portal detour) |
| 4. Create + revoke API key               | **YES** (in dev with mock); no rotate; no production gateway |
| 5. Profile + 2FA                         | **NO** — route doesn't exist |
| 6. Notification preferences              | **PARTIAL** — per-worker yes, workspace no |

Buyer-completable: 1.5 / 6 (Billing fully + half of API-keys because rotate is missing).

The single highest-impact fix remains the same as Wave-8 flagged: **wire the members endpoints**. The next two — **add `/settings/profile` + 2FA** and **add `/settings/notifications`** — are mid-sized briefs (a few hundred LOC each) but each closes a procurement-mandatory journey. The three together convert a "we'll get back to you on settings" sales conversation into a "yes, the customer can fully self-serve" claim.

For v1 demos against a friendly buyer (who tolerates the Stripe detour for invoices and doesn't probe profile/2FA), Billing + Security + API-keys + per-worker notifications carry the demo. For a procurement-grade evaluation, Journeys 1, 5, and 6 are stop-the-deal gaps.

---

## Files of interest (absolute paths, for follow-up briefs)

- `~/code/argo/web/app/(app)/settings/page.tsx` (landing — `coming: true` data drift on Security tile)
- `~/code/argo/web/app/(app)/settings/members/page.tsx` (handleInvite + handleEditSave deferred)
- `~/code/argo/web/app/(app)/settings/members/InviteMemberModal.tsx` (`includes("@")` is the only email validation)
- `~/code/argo/web/app/(app)/settings/security/page.tsx` (SSO ships; retention/sessions/encryption don't)
- `~/code/argo/web/app/(app)/settings/api-keys/page.tsx` (create + revoke; no rotate)
- `~/code/argo/web/lib/api/api-keys.ts` (gateway brief planned, not landed; mock fallback active)
- `~/code/argo/web/app/(app)/settings/data-export/page.tsx` (visually drifts from settings register)
- `~/code/argo/web/app/(app)/billing/page.tsx` (URL mismatch; no post-upgrade banner)
- `~/code/argo/web/lib/billing/use-checkout.ts` + `use-portal.ts` (Stripe wires fully)
- `~/code/argo/web/components/skycot/worker/settings/ChannelsSection.tsx` (per-worker channel UI; not workspace-level)
- `~/code/argo/crates/argo-cli/src/handlers/gateway/billing.rs` (gateway billing routes confirmed shipped)
