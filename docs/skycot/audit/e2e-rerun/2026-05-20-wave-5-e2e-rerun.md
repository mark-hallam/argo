# E2E demo journey re-run — wave 5 regression catch (2026-05-20)

Auditor: Wave-5 audit agent #1 (Claude Opus 4.7 1M context)
Method: live walk via Chrome MCP at 1512x805 against `http://localhost:3000`
Source-of-truth runbook: `docs/skycot/operator/demo-runbook.md`
Compared against: `docs/skycot/audit/e2e-demo/2026-05-20-wave-1-e2e-demo-journey.md`
Wave-1/2/3/4 fix commits: `09b5f1cc`, `0b58719d`, `4f4059d9`, `d04a7032`

---

## TL;DR

**NOT-RECORD-READY.** The demo path has improved on 6 of the 8 Wave-1-claimed fixes but **one critical regression makes the demo unrecordable as scripted**: on a fresh navigation to `/` (the Approvals inbox the demo opens on at 01:15), the first paint shows "**Inbox zero. / 0 waiting**" for ~3-4 seconds before the seed data populates and the page re-renders to "**7 decisions need you. / 7 waiting**". The voiceover script reads "your inbox … 7 decisions need you" — on camera the operator will see "Inbox zero" for the opening seconds. Plus, the `/` page now has a permanently rendered **"SKYCOT preview — design system Track C" dev-toolbar at the bottom** (light/dark/theme switcher) that wasn't visible in Wave-1 — this will be on camera at the demo's emotional money shot.

Three secondary regressions:
1. `/workflows` search input is now HALF-wired (counter updates "0 of 8" but cards don't filter — visually contradicts the counter), worse than Wave-1's "dead input".
2. Audit detail `INV-[PHONE]` / `QT-[PHONE]` placeholder text still rendered (Wave-1 P0 finding — no fix shipped in any of waves 1-4).
3. `/privacy`, `/terms`, `/security` legal stubs render their H1 with `font-family: "Audiowide, Inter, sans-serif"` declared, but the **Audiowide font is not actually loading** on these standalone pages (`document.fonts` shows `audiowideLoaded: false`). Falls back to Inter sans-serif. Brand-promise violation on the legal pages.

Two new minor issues found:
4. Tab title leak on app pages (`/`, `/team`, `/workers/...`, `/audit`, `/audit/action/*`, `/cohorts`) — all still show "SKYCOT — Hire your first AI teammate" instead of "Approvals · SKYCOT" etc. Wave-2 SEO fix covered marketing + auth surfaces but missed the in-app shell.
5. Auth.js `ClientFetchError` console-error storm still present (Wave-1 finding — root cause is Chrome's leveldb running out of disk on this machine). Mitigation requires fresh Chrome profile for recording.

The 5 GREEN fixes that held: legal pages 200, /compare public-accessible, Save+publish toast fires, Test run toast fires, Propose toast fires, brand-loud-hero respects prefers-reduced-motion, global-error.tsx exists, builder error.tsx exists, Approvals search input has aria-label, Approvals h1 element exists, /landing footer routes to /privacy + /terms + /security, Audit row IDs now uppercase + drill-in lands on real detail page.

---

## Wave-1-vs-wave-5 score on the 8 specific fixes the operator listed

| # | Fix | Wave-1 outcome | Wave-5 verify | Verdict |
|---|---|---|---|---|
| 1 | `/compare` is publicly accessible | Was behind `(app)` auth group | `curl -L --max-redirs 0 → 200` (no redirect); navigated as unauthenticated browser → loads | **HOLDS** |
| 2 | `/workflows` search filter | Wave-1 said "dead input — typed query did nothing" | Programmatic value="approval" set → counter shows "0 of 8" but all 8 cards still rendered in DOM and visually | **REGRESSED (half-wired, worse)** |
| 3 | BrandLoudHero `prefers-reduced-motion` | Was missing | `web/components/skycot/BrandLoudHero.tsx:99` `@media (prefers-reduced-motion: reduce)` block found | **HOLDS** |
| 4 | `/landing` footer Privacy → /privacy 200 | Was placeholder `href="#"` | Real `<Link>` to /privacy verified, curl returns 200 | **HOLDS (route)**, but Audiowide font on the page doesn't actually load → **NEW ISSUE** |
| 5 | `/workflows/.../build` Save + publish click fires toast | Wave-1 said all 3 builder CTAs were silently broken | Clicked the Save+publish button via accessibility ref → toast "Published v3 · Workflow is live for the tenant." fired | **HOLDS** |
| 6 | `/audit` drill-in shows a real detail page | Wave-1 said clicks fell through to generic synthetic chain | Drill-in to `act_01J0JAMAL_025` lands on a polished detail page with breadcrumb "Audit > resolved SUP-29382 with KB article SK-413", Inputs + Provenance + Decision chain rendered | **HOLDS** |
| 7 | Approvals search input has aria-label | Missing | `aria-label="Search approvals by worker, action, or detail"` confirmed via JS | **HOLDS** |
| 8 | `/global-error.tsx` exists at app root | Was missing | `web/app/global-error.tsx` exists (3541 bytes, 2026-05-20 22:41) + `web/app/(app)/workflows/[v]/[n]/build/error.tsx` (1527 bytes) | **HOLDS** |

**7 of 8 Wave-1-4 fixes verified intact, 1 regressed.**

---

## Page-by-page re-walk

### 1. `/landing` — HOLDS

- 200 OK, brand-loud register, Audiowide hero loads correctly
- Hero says "Hire a **teammate**" (Wave-3 voice fix)
- Trust pellets: "NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · ISO 27001 CONTROLS SUPPORTED · GDPR · HIPAA WORKFLOWS · 2,847 TEAMS THIS MONTH" — Wave-3 over-claim softening confirmed
- "Hire your first **teammate**" CTA (Wave-3)
- Footer: SKYCOT · COMPARE · CUSTOMERS · PRICING · GITHUB · DOCS · PRIVACY · TERMS · SECURITY · CONTACT · "© 2026 SKYCOT · MADE IN MELBOURNE · SKYCOT.COM" — all links resolve to real pages now
- Tab title: "Hire your first AI teammate · SKYCOT" ✓
- **Still unfixed from Wave-1**: worker bubbles overlap the SKYCOT logo (~50px-wide quote columns)
- **Still unfixed from Wave-1**: top-nav "Features" link → 404

### 2. `/auth/signup` — HOLDS

- 200 OK
- Page-specific title "Hire your first AI teammate · SKYCOT" (Wave-2 SEO fix)
- 3 OAuth buttons still labeled `v1.x` (Wave-1 polish item, unfixed)
- Magic-link form fully wired

### 3. (skip magic-link) — N/A

Direct navigation to `/` was attempted, see #4.

### 4. `/` (Approvals) — **CRITICAL REGRESSION**

- 200 OK eventually
- **First-paint regression (~3-4 second flash)**: hero shows "**Inbox zero. / Nothing waiting on you right now. Your AI teammates have it handled.**" with WAITING 0, body shows spinner + "Loading approvals…". Then re-renders to "**7 decisions need you.**" with WAITING 7 and 7 approval cards. The demo voiceover at 01:15 says "your inbox … 7 decisions need you" — but the camera will catch "Inbox zero" for the opening seconds.
- **NEW dev-toolbar visible at bottom**: "SKYCOT preview — design system Track C. Switch theme to test white-label:" with light/dark/theme switcher. Permanently rendered. Will be on camera. Source: `web/app/(app)/page.tsx:337`.
- Hero h1 element present (a11y Wave-2 fix) ✓
- Search input has `aria-label="Search approvals by worker, action, or detail"` (Wave-2 a11y fix) ✓
- "mock (offline)" pellet HIDDEN ✓ (Wave-1 fix)
- "connecting…" pellet HIDDEN ✓ (Wave-1 fix)
- "+ Hire teammate" CTA wired to /hire ✓ (Wave-1 fix)
- **Tab title leak**: "SKYCOT — Hire your first AI teammate" instead of "Approvals · SKYCOT"
- Mariah's approval card + Approve button: works once loaded; counter ticks 7→6 (kept from Wave-1)
- Gateway returns 503 in browser network panel but page falls back to seed data correctly. Underlying gateway is healthy (`curl http://127.0.0.1:7400/api/v1/approvals → 200 {"items":[]}`).

### 5. `/team` — HOLDS

- 200 OK
- h1 element: "Team" ✓ (Wave-2 a11y)
- Hero: "6 teammates, one team." with WORKING 1 / ACTIONS/WK 1,122 / HOURS SAVED 72
- All 6 worker cards present (Mariah, Priya, Jamal, Nori, Rio, Sam)
- Tab title leaks "SKYCOT — Hire your first AI teammate" — should be "Team · SKYCOT"
- Wave-1 commit added `TeamErrorState` branch — not visible in happy path

### 6. `/workers/wkr_01J0MARIAH/overview` — HOLDS

- 200 OK
- Tabs Overview / Activity / Memory / Skills / Trust / Settings present
- Personality chips: Patient · Detail-oriented · Picture-perfect listings · Polite ✓
- Great at + Escalate when columns match runbook verbatim
- Signature behaviors sub-section ✓
- Tab title leaks again

### 7. `/workflows` — **REGRESSION (search filter)**

- 200 OK
- Tab title "Workflows · SKYCOT" (Wave-2 SEO fix) ✓
- All 8 workflow cards render
- **Search filter REGRESSED**: setting input value to "approval" via JS shows:
  - Counter pill: "0 of 8" (Wave-1 fix wired the counter)
  - But all 8 cards STILL visually rendered (grid uses `SEED_WORKFLOWS.map(...)` at line 498 instead of `filtered.map(...)`)
  - Source: `web/app/(app)/workflows/page.tsx:498` — bug in the Wave-1 patch
- This is worse than Wave-1 baseline because the counter now lies (says "0 of 8" while showing 8)
- Bonus filter logic bug: "approval" doesn't match "approve_refund" because filter checks `wf.name.includes("approval")` not `"approve"` — Wave-1's filter targets (name + vertical + description) miss the obvious case
- "Browse 50+ templates" CTA still promissory (page has 8)

### 8. `/workflows/sales/qualify_lead/build` — HOLDS (with caveats)

- 200 OK, React Flow canvas renders cleanly
- **Propose toast** fires on click: "Proposal ready for review · 1 step change proposed. Approve to apply to the canvas." (Wave-1 fix CONFIRMED LANDING)
- **Test run toast** fires on click: "Test run started → Test run complete · All 5 steps passed. No errors." (Wave-1 fix CONFIRMED)
- **Save + publish toast** fires on click: "Published v3 · Workflow is live for the tenant." (Wave-1 fix CONFIRMED)
- Caveat: typing into the Propose input has a focus issue — typed text doesn't always appear in the visible field even though the toast still fires. Operator must demo this carefully.
- "React Flow" attribution watermark still visible bottom-right (Wave-1 polish item, unfixed)
- "Add st" button still partially clipped behind mini-map (Wave-1 polish item, unfixed)
- Has `error.tsx` boundary (Wave-4 fix)

### 9. `/compare` — HOLDS

- 200 OK, publicly accessible (Wave-1 fix verified)
- Tab title: "SKYCOT vs Zapier, n8n, Crew.ai, Lindy · SKYCOT" (Wave-2 SEO) ✓
- Hero: "A teammate isn't a workflow. It's the thing the workflow serves."
- 5-card competitor strip (SKYCOT / Zapier / n8n / Crew.ai / Lindy)
- Comparison matrix renders cleanly
- **Mobile wrap**: table container has `overflow-x: auto` (Wave-2 responsive fix verified)
- FOUC on first paint (Wave-1 finding) — couldn't re-verify objectively; the page now renders fast enough on this run that I didn't catch unstyled flash

### 10. `/audit` — HOLDS structurally; `[PHONE]` placeholder still unfixed

- 200 OK
- Row IDs are now uppercase: `act_01J0JAMAL_025`, `act_01J0SAM_010`, `act_01J0PRIYA_000`, etc. (Wave-4 fix)
- Drill-in to `/audit/action/act_01J0JAMAL_025` lands on a polished detail page (Wave-4 fix CONFIRMED)
- Detail page breadcrumb "Audit > resolved SUP-29382 with KB article SK-413" — exact match with the feed-row text
- Inputs + Provenance + Decision chain all render
- **`[PHONE]` placeholder NOT FIXED**: 5 rows in the audit feed still contain raw `INV-[PHONE]` / `QT-[PHONE]` strings ("extracted INV-[PHONE] line items", "validated QT-[PHONE] against pricing rules", "flagged 32% discount on QT-[PHONE] for Deal Desk", "flagged duplicate invoice INV-[PHONE]", and the breadcrumb of the detail page also reads "extracted INV-[PHONE] line items"). This is a Wave-1 P0 finding that none of waves 1-4 addressed.
- Tab title leaks "SKYCOT — Hire your first AI teammate" on both /audit and /audit/action/*

### 11. `/privacy`, `/terms`, `/security` — partial regression

- All three 200 OK ✓
- All three have correct page-specific tab titles ("Privacy · SKYCOT", "Terms · SKYCOT", "Security · SKYCOT") ✓
- All three pages render content (legal stubs)
- **REGRESSION**: H1 declares `font-family: "Audiowide, Inter, sans-serif"` but the **Audiowide font is not loaded** on these pages. JavaScript probe:
  - `document.fonts → audiowideLoaded: false` on /privacy, /terms, /security
  - `document.fonts → audiowideLoaded: true` on /landing
  - Source files (`web/app/privacy/layout.tsx`, etc.) don't import the Audiowide font; the H1 inline style relies on the font being globally available, but it isn't on these standalone pages
- Visual effect: H1 falls back to Inter sans-serif, rendered with purple→pink gradient. Looks fine but is **NOT brand-on Audiowide as the wave-1 runbook expected.**
- Body content is appropriate (workspace isolation, no training, encryption-at-rest, hash-chained audit, GDPR/CCPA rights for /privacy; similar for /terms, /security)

---

## NEW issues found in wave-5 (not in any prior wave)

1. **Approvals first-paint flash** — opens to "Inbox zero" + "Loading approvals…" for 3-4 seconds before the seed populates. Demo opens here, voiceover script reads "7 decisions need you." Wave-2 fix (h1 on empty state) inadvertently created the strong "Inbox zero" empty-state hero.

2. **Dev-toolbar visible on `/`** — "SKYCOT preview — design system Track C. Switch theme to test white-label:" with light/dark/theme switcher rendered at the bottom of the Approvals page. Source: `web/app/(app)/page.tsx:335-340`. Not behind a debug flag. Will be on camera at 01:15.

3. **`/workflows` search filter half-wired** — counter responds but cards don't filter. Worse than Wave-1's "dead input" because the counter now visibly lies.

4. **Audiowide font fails to load on /privacy, /terms, /security** — declared but not imported on these standalone-layout pages. H1 falls back to Inter sans-serif.

5. **Tab title leak persists on all in-app pages** — `/`, `/team`, `/workers/...`, `/audit`, `/audit/action/*`, `/cohorts` all still leak "SKYCOT — Hire your first AI teammate".

---

## Issues from Wave-1 that NO patch addressed

- `INV-[PHONE]` / `QT-[PHONE]` placeholder text in /audit feed (P0 in Wave-1)
- /features 404 but linked from /landing top-nav
- OAuth buttons labeled `v1.x` on /auth/signup
- React Flow attribution watermark on /workflows/.../build
- "Add st" button clipped behind mini-map on /workflows/.../build
- Worker bubbles overlap on /landing hero
- Auth.js ClientFetchError console storm (Wave-1 said "Chrome profile disk-full" — caller should clear Chrome profile for recording)

---

## Console + network summary

- 441 error/exception messages in current session
- Repeating: `Error: IO error: …/065394.ldb: FILE_ERROR_NO_SPACE` + `ClientFetchError: There was a problem with the server configuration` (auth.js SessionProvider)
- `http://127.0.0.1:7400/api/v1/approvals → 503` in browser network panel (despite `curl → 200`) — likely a browser-side fetch issue (CORS, credentials, or fetch with stale headers) that the client falls back from gracefully

---

## Top 3 last-mile fixes before tomorrow's record

1. **Suppress the "Inbox zero" first-paint flash on `/`.** Either render the loading skeleton with the same gradient hero copy ("7 decisions need you") pre-populated from cached/synthetic data, OR delay the hero render until the data has resolved. The demo opens here and the operator's voiceover at 01:15 says "7 decisions need you" — currently the camera will catch "Inbox zero" for 3-4 seconds. **5 minutes of work**: swap initial empty-state hero copy to use a non-load-bearing placeholder ("Pulling your inbox…") instead of "Inbox zero."

2. **Hide the dev-toolbar on `/`** (`web/app/(app)/page.tsx:335-340`). Wrap the `<footer>` in a `process.env.NEXT_PUBLIC_DEMO_MODE !== "true"` check, OR remove unconditionally. **2 minutes of work.** Same fix could ship with a `?debug=1` re-enable for follow-up.

3. **Substitute the `[PHONE]` placeholder strings in /audit feed.** Find `INV-[PHONE]` / `QT-[PHONE]` in the sample-data fixtures and replace with real-looking IDs like `INV-39482` / `QT-29381`. This is a P0 from Wave-1 that no fix shipped against. **10 minutes**: grep for `[PHONE]` in the audit seed file and replace.

Honorable mention: fix the /workflows search filter to actually filter (change line 498 `SEED_WORKFLOWS.map` to `filtered.map`), but this is below-the-fold of the runbook script — the operator's voiceover doesn't demo the search input.

---

## Final go/no-go recommendation

**NOT-RECORD-READY as-is.**

The blockers are #1 (the "Inbox zero" first-paint flash on the page the demo opens on) and #2 (the dev-toolbar on camera). Both are 5-10 minute fixes. With those two fixes shipped and a fresh Chrome profile (to silence the auth.js console storm), the demo is recordable. The Wave-1 P0 `[PHONE]` placeholder is still an issue but the operator can skip the affected rows when panning over /audit.

**Estimated time to record-ready:** 30 minutes of frontend work + fresh Chrome profile setup.

Without those fixes, the demo records but the **opening 01:15 beat will visibly contradict the voiceover** ("your inbox … 7 decisions need you" while the screen says "Inbox zero · 0 waiting") and the **theme-picker toolbar will be visible to the audience**.

---

Auditor signature: Wave-5 e2e re-run agent (Claude Opus 4.7, 1M context window) · 2026-05-20
