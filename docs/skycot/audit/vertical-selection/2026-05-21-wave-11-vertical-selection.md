# Wave-11 audit #4 — vertical-selection step (/onboarding/pick-vertical)

**Date:** 2026-05-21
**Auditor:** Wave-11 agent #4 (read-only static analysis)
**Scope:** `web/app/(setup)/onboarding/pick-vertical/page.tsx` (the picker UI) + the `/api/v1/onboarding/install-vertical` backend endpoint it calls.
**Method:** read every line of the page, traced the API call into the gateway dispatch, cross-referenced the on-disk verticals catalogue and prior wave-3 audit, checked the ledger for the in-flight backend brief.
**Inputs:**
- Wave-3 audit (`docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md`): flagged silent-404 bug at line 74 + 3-of-4 onboarding endpoints unimplemented.
- Operator memory: F1 editorial-center is the locked design for all wizards; brief `web-gateway-endpoints-onboarding-batch` is CLAIMED, not MERGED.

---

## Headline verdict

The picker UX has been **partially fixed** since wave-3: the page now wraps in `OnboardingShell` (F1 brand-on shell) and renders a 5-card grid with `aria-pressed`, sample duties, and a Back button. **The silent-404 bug at line 83 (was line 74 at wave-3 — code is unchanged in spirit, line moved due to refactor) is NOT fixed** — `if (!res.ok && res.status !== 404)` still treats 404 as success, and the backend brief that would make the endpoint real (`web-gateway-endpoints-onboarding-batch`) is CLAIMED but unmerged as of `50e65e9f`. The page is also showing **5 hardcoded verticals** when there are **36 vertical packs on disk** — the picker is a static curated subset, not the real catalogue.

---

## The 5 audit dimensions

### Dimension 1 — Count + visual distinguishability

**Score: 4/10.**

- **Count:** 5 verticals hardcoded inline in `VERTICALS` array (page.tsx:18-54): `gtm-engineer`, `marketing-intern`, `sales-sdr`, `customer-support`, `finance-ar`.
- **On-disk reality:** `ls verticals/` returns **36 vertical packs** including `ap-specialist`, `bookkeeping-assistant`, `executive-assistant`, `hr-screener`, `lead-research`, `legal-paralegal`, `medical-coder`, `procurement-assistant`, `qa-engineer`, `real-estate-agent`, `recruiter`, `recruiter-screener`, `research-analyst`, `sales-follow-up`, `social-media-drafter`, `support-tier1`, `tax-prep`, `vendor-manager`, etc. **The picker hides 31 of them.**
- **Gateway has `GET /verticals` already** (`routes_verticals.rs`) — returns the real catalogue from disk via `argo_vertical::load_dir`. The page is NOT calling it; it ships its own typed `VerticalCard[]` inline.
- **Visually distinguishable:** Each card uses the same border/colour treatment with the displayName + summary + skillCount + 3 sample duties. No vertical-specific iconography (no logos), no colour-coding, no department-tab grouping. All 5 cards are visually identical except for text content.
- **Layout:** `grid grid-cols-1 sm:grid-cols-2` — mobile stacks vertically, desktop is 2-up so the 5th card hangs alone on the right at the bottom. Single-row visual rhythm is broken at the 5-card length.

### Dimension 2 — Per-vertical metadata (name + logo + 1-line + workflow/skill count preview)

**Score: 5/10.**

| Surface | Present? | Notes |
|---|---|---|
| Name (displayName) | yes | `GTM Engineer`, `Marketing Intern`, etc. (page.tsx:21,28,35,42,49) |
| Logo / icon | **no** | Zero per-vertical iconography; only the global Check ✓ when selected (page.tsx:139). No emoji, no colour swatch, no avatar. |
| 1-line description | yes | `summary` field, 1 sentence each (page.tsx:22,29,36,43,50). Quality varies — some are sharp ("Tier-1 ticket triage, FAQ replies, escalation routing"), some are generic ("Go-to-market automation"). |
| Workflow count | **no** | Workflows not surfaced. The picker only shows `skillCount` (page.tsx:14). |
| Skill count | yes (sort of) | `skillCount` is hardcoded as `5, 5, 5, 5, 4` — **not read from manifest**. Real manifests live in `verticals/<id>/manifest.toml`; the picker doesn't load them. |
| Sample duties | yes | 3 bullets each (page.tsx:24,31,38,45,52) — these ARE valuable copy and the most differentiating signal per card. |

Hardcoded skill counts is a drift hazard: if `verticals/gtm-engineer/manifest.toml` gains a 6th skill tomorrow, the picker still says "5 skills" forever. Should call `GET /verticals/<id>` per card or fold into `GET /verticals` response.

### Dimension 3 — Selection UX (click / visual feedback / confirm before continuing)

**Score: 4/10.**

- **Click-to-select:** Each card is a `<button onClick={() => onChoose(v.id)}>` (page.tsx:124-126). One-click commits — both the API call AND the route advance happen on the same click.
- **Visual feedback:** ✅ selected card gets a purple border + purple shadow + Check ✓ icon + `aria-pressed=true` (page.tsx:128-141). Good a11y. Hover state is `border-purple/40`. `disabled:cursor-wait` while `busy` (page.tsx:130).
- **Confirm before continuing:** ❌ **NONE.** There is no two-step "select → confirm". `onChoose` immediately fires `setVertical(v.id)`, calls `fetch()`, then `router.push("/onboarding/first-hire")` — all in one synchronous handler. There is no review affordance, no "are you sure?", no "this will install N workflows and M skills" preview.
- **Cannot un-pick:** Once a card is clicked, the user is forced onto first-hire. The only way back is the Back button to `/onboarding/connect-provider` (page.tsx:168) — but the install API call has already (allegedly) fired. If the install actually succeeded server-side, going back-and-forward would re-install or fail idempotency.
- **Mis-click cost:** High. The 5 cards are visually identical. A trackpad slip on the wrong tile installs the wrong vertical and silently advances. Wave-3 flagged this; not addressed.

### Dimension 4 — Skip / pick-later option

**Score: 1/10.**

- **No Skip button.** No "I'll pick later" link. No "Try them all" or "Just give me defaults" CTA.
- **Back is the only escape.** Pressing Back goes to `/onboarding/connect-provider` — which itself has a Skip → mock provider. From pick-vertical there is no Skip → first-hire.
- **Comparison:** `connect-provider` has Skip (uses a mock provider). `first-hire` has no Skip. `pick-vertical` should mirror connect-provider's pattern — let the user defer the choice (default to `gtm-engineer` as the demo path) and continue. Today the user MUST commit to one of the 5 hardcoded options to leave the page.
- **Consequence:** A user who is undecided between two verticals has no safe exit. They will Back → ConnectProvider → re-do (which has no-op semantics on a stored key) → ForwardArrow back into the same picker. Pure friction.

### Dimension 5 — Backend status + silent-404 bug

**Score: 1/10.** This is the load-bearing finding.

#### 5a) Endpoint status: **STILL NOT LANDED.**

- `grep -n "install.vertical\|install_vertical" crates/` returns ZERO matches in any Rust file.
- Confirmed via `routes_dispatch_post.rs:191-238` (api_v1_dispatch): only `/api/v1/onboarding/provision` is wired (line 206-209). No `install-vertical`, `provider-key`, or `first-hire` handler exists. The gateway falls through to the 404 default.
- Brief `web-gateway-endpoints-onboarding-batch` is **CLAIMED 2026-05-20T16:51:40Z on mac-host**, NOT yet IN_REVIEW / MERGED. The brief explicitly proposes the install-vertical endpoint with `{installed:true, workflows_added:N, skills_added:M}` shape (streams/web-gateway-endpoints-onboarding-batch.md:25).
- Mock data path: the e2e test at `web/tests/e2e/onboarding.spec.ts:150-156` page.routes the request to a fulfilled-200 stub — meaning the green-bar e2e is testing the **stub**, not a real backend. There is no integration test that exercises a live gateway response.

#### 5b) Silent-404 bug: **NOT FIXED.**

Page.tsx:83 (was wave-3's line 74; refactor shifted by 9 lines):
```ts
if (!res.ok && res.status !== 404) {
  throw new Error(`gateway returned HTTP ${res.status}`);
}
router.push("/onboarding/first-hire");
```

The condition explicitly excludes 404 from the error path. A 404 falls through to the `router.push("/onboarding/first-hire")` happy path. **The current production reality (no backend endpoint) is the 404 case — every onboarding user is silently traversing the "treat 404 as success" branch.**

The wave-3 audit called this out verbatim; the wave-3-batch brief explicitly lists "fix the silent-404 bug at line 74" in its may-touch (line 50 of the brief). The fix has NOT landed because the brief is CLAIMED-not-merged.

#### 5c) Hand-rolled fetch vs `apiPost`

Still using raw `fetch()` with `${process.env.NEXT_PUBLIC_GATEWAY_URL ?? ""}` (page.tsx:75-82). Wave-3 flagged: every other onboarding call uses `lib/api/client.ts`'s `apiPost`; this one doesn't, breaking the "single client" audit promise. **Not fixed.** The empty-string fallback means a relative URL when env unset, which works locally only because Next.js dev-server proxies to gateway — production deploys will break if `NEXT_PUBLIC_GATEWAY_URL` is unset.

---

## Top 3 issues (in fix-order)

### Issue #1 — Silent-404 happy path is masking a 100% backend miss (P0, ship-blocker)

**Where:** `page.tsx:83` — `if (!res.ok && res.status !== 404)` then `router.push("/onboarding/first-hire")` on the catch branch too (line 93).
**Impact:** Every onboarding user today silently advances "successfully" while the gateway returns 404. The choice is saved to `useOnboardingDraft` (zustand) but no real install happens. On first-hire (the next step), users will hit a second 404 (`/api/v1/onboarding/first-hire` also unimplemented) and hard-stick on step 5 per wave-3. The product is hard-broken end-to-end for any user past step 4.
**Fix:** Land `web-gateway-endpoints-onboarding-batch` (currently CLAIMED). Then remove `&& res.status !== 404` so 404 throws like any other error. Until the backend exists, the picker should at minimum render a visible warning banner ("vertical install is in beta; choice saved locally").

### Issue #2 — 5 hardcoded cards hides 31 of 36 vertical packs (P1)

**Where:** `page.tsx:18-54` — `VERTICALS` inline array.
**Impact:** Operator-decision-locked memory says the project is enterprise-focused with a mature 36-pack catalogue (recruiter, hr-screener, ap-specialist, legal-paralegal, medical-coder, procurement-assistant, qa-engineer, etc.). The picker shows 5 demo packs. A potential customer who wants "Accounts Payable" gets nothing on this screen and silently churns. Also, the 5 hardcoded `skillCount` numbers will drift from the manifests; nothing pins them.
**Fix:** Call `GET /verticals` (already exists at `routes_verticals.rs`, returns the on-disk catalogue with `{id, name, version, description, tags, root}`). Group by `tags` or by directory metadata into 6-8 department tabs (consistent with the locked cohorts-page design: 6 dept tabs + asymmetric layout per memory `project_cohorts_page_design_locked.md`). Add a search box with `⌘K` (consistent with skills-page design: `project_skills_page_design_locked.md`).

### Issue #3 — One-click commit with no skip and no confirm (P1)

**Where:** `page.tsx:69-97` — `onChoose` calls API + advances route in one synchronous handler.
**Impact:** Mis-click cost is high (5 visually-identical cards) and there's no Skip affordance for the "I'll decide later" user. Inconsistent with sibling `connect-provider` step which has Skip → mock provider.
**Fix (smallest):** Split selection from commit — first click highlights, second click (a "Continue" CTA) fires the install. Add a "Skip — install gtm-engineer default" link in the footer next to Back. Both are no-net-LOC adjustments (move the API call from `onChoose` to a separate `onContinue` handler).

---

## Honourable mentions (not in top 3)

- **No "this includes" preview.** Card shows `skillCount` and 3 sample duties but no workflow count. Brief promises `{workflows_added:N, skills_added:M}` post-install — the picker should preview those numbers PRE-install so the user knows what they're committing to.
- **No vertical logos / iconography.** All 5 cards are text-only. Per locked design memories, the brand has 6 dept "department tabs" in the cohorts page each with a distinguishing colour treatment — apply that same vocabulary here (sales = blue, marketing = pink, support = green, finance = amber, etc.) so cards are scannable at a glance.
- **`OnboardingShell` IS in use** (page.tsx:114 wraps the section). Wave-3's complaint that the page didn't use the F1 shell has been addressed — this is the one real improvement since wave-3.
- **Vertical IDs match.** The 5 hardcoded `id` values (`gtm-engineer`, `marketing-intern`, `sales-sdr`, `customer-support`, `finance-ar`) all exist as real directories under `verticals/` — they are not vapourware. The bug is the picker hides the other 31.
- **`useOnboardingDraft` has no TTL** — wave-3 noted; still true. A user who abandons mid-flow has an indefinite stale draft in localStorage.
- **Test mocks describe a contract that doesn't exist.** `web/tests/e2e/onboarding.spec.ts:150-156` mocks `install-vertical` to return `{installed: true}`. The real backend (when landed) is supposed to return `{installed: true, workflows_added: N, skills_added: M}` per the brief. The mock doesn't match the planned contract — it's a thinner stub that will silently pass even when the backend ships a richer shape.

---

## Score summary

| Dimension | Score | One-line |
|---|---|---|
| 1. Count + distinguishability | 4/10 | 5 hardcoded vs 36 on disk; all cards visually identical |
| 2. Per-vertical metadata | 5/10 | name/summary/duties yes; no logo, no workflow count, skill count is a literal |
| 3. Selection UX | 4/10 | a11y is good; one-click commit, no confirm, no un-pick |
| 4. Skip option | 1/10 | none — users must commit one of 5 to leave |
| 5. Backend + silent-404 | 1/10 | endpoint STILL missing; silent-404 happy path STILL present |
| **Overall** | **3.0/10** | wave-3 was ~3, modest brand-fix progress, no functional progress |

---

## Recommended brief follow-ups (beyond the in-flight batch)

1. **`web-onboarding-pick-vertical-real-catalogue`** — replace `VERTICALS` inline array with `useQuery(['verticals'], () => apiGet('/verticals'))` plus department tabs + ⌘K search. Sits on top of the already-shipped `GET /verticals` endpoint. ~150 LOC. Should not block on the install-vertical backend.
2. **`web-onboarding-pick-vertical-skip-and-confirm`** — split highlight from commit; add Skip → install default. ~50 LOC. Trivially landable today.
3. **`web-onboarding-pick-vertical-error-surfacing-fix`** — drop the `&& res.status !== 404` masking once `web-gateway-endpoints-onboarding-batch` ships; surface real errors via the existing yellow banner; switch to `apiPost`. ~30 LOC. Trivially landable once the batch brief MERGES.
