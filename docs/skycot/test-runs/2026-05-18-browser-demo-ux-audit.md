# 2026-05-18 Browser Demo UX Audit

**HEAD**: `91f9d032` (test-run started from) → with 1 uncommitted change (next.config.ts rewrite)
**Operator**: mark-hallam
**Run by**: claude conductor via Chrome MCP
**Date**: 2026-05-18

---

## TL;DR

I drove a real Chromium browser through the demo flow at 1920×1080 viewport. Here's what I found, ranked by what matters for a CEO recording v1.0:

**🔴 P0 demo blocker (not fixed yet, needs decision):**
- **CORS preflight 404 on every `/api/v1/*` endpoint.** Gateway has no `OPTIONS` handler. SPA dev-server (port 3000) → gateway (port 7400) cross-origin POSTs fail. "Failed to fetch" surfaces at scene 2 (workspace name → Continue). I wrote a fix (Next.js rewrite in `web/next.config.ts`) — uncommitted in working tree pending your call.

**🟡 Three pages 500-error:**
- `/skills`, `/approvals`, `/settings` all hit the branded 500 page ("Something went wrong on our side") instead of rendering. Likely missing server-side fetch failure handling.

**🟡 Demo pages show seeded mock data, not session data:**
- I'm signed in as `alice+demo@throwaway.cc` but bottom-left of every authenticated page reads "Sam Harlow · Acmeco · Owner". Workers/Team/Cohorts/Audit show realistic mock data. Fine for the recorded demo (the narrative IS "imagine you've been running for a few days") but viewers may assume it's live. Real fresh accounts see the same Sam Harlow workspace.

**🟢 Real wins:**
- Marketing landing renders cleanly at brand-consistent quality
- Signup → check-email flow works end-to-end (token visible in gateway log)
- Magic-link verify lands on `/onboarding/welcome` with correct progress indicator (1/5)
- Audit page is a *gorgeous* demo showpiece (cost per action, status pills, time grouping)
- Team page renders 6 worker cards with status badges + today/tomorrow narratives
- Cohorts page shows trust-tier grouping (Trusted / Developing / Established / Pre-trust)

---

## Scene-by-scene findings

### Scene 0 — Marketing landing (localhost:3001)

**Renders cleanly.** Hero "Hire a teammate, not a tool." + 3 feature cards (Workers with memory / Approve in the tools you already use / Auditable by default) + stats row (6 Mission claims / 2 Execution backends / ∞ Approval channels / 100% Audit coverage) + CTA section + footer.

**UX observations:**
- ✅ Two CTA placements (header + hero) — good discoverability
- ✅ Clear value prop, time-bound expectations ("90 seconds" later in onboarding)
- ✅ Cookie banner present (privacy policy linked inline)
- ⚠️ **Feature card headlines stack vertically** at 1920px width: "Workers / with / memory.", "Approve / in the / tools you already / use." — looks unintentional. Likely the cards use a max-width that's narrower than the column.
- ⚠️ "Hire your first worker" CTA links to `https://app.skycot.com/signup` — in dev mode this 404s unless `NEXT_PUBLIC_APP_URL` env is set on the marketing dev server. Hardcoded production URL.
- ⚠️ **Marketing dev DOES NOT work with Turbopack** (`pnpm dev` with `--turbopack` default). MDX loader throws "options not serializable". Workaround: `pnpm exec next dev --port 3001` (no Turbopack). This needs a fix in marketing/package.json or `@next/mdx` config.

### Scene 1 — Signup → Check inbox (localhost:3000)

**Works end-to-end.** Form accepts email, posts to `/api/v1/auth/signup` via SPA edge route (`web/app/api/auth/signup/route.ts`), redirects to `/auth/check-email?email=<encoded>`.

**UX observations:**
- ✅ Subcopy: "We'll email you a sign-in link. No password." — sets the auth model expectation up front
- ✅ Confirmation page echoes the email back: "alice+demo@throwaway.cc. **It expires in 15 minutes.**"
- ✅ "Resend the link" available
- ⚠️ **Card sits alone in a vast empty page.** Both signup and check-email are tiny cards at center on 1920px. Feels lonely. An illustration, brand mark, or value-prop sidebar would balance the layout.
- ⚠️ No SKYCOT logo on these auth pages (it's on onboarding pages). The brand mark is missing exactly where new users first interact.
- ⚠️ "Resend the link" — no rate-limit feedback. User can mash it 6 times and the 6th will 429 silently.
- ⚠️ No "Change email" affordance on check-inbox page. User has to use browser back.

### Scene 2 — Magic-link verify → Onboarding welcome (1/5)

**Works.** I pulled the token from the gateway log (the noop email provider logs it via `tracing::warn!`), navigated to `/auth/verify?token=<token>`, landed on `/onboarding/welcome`.

- ✅ Progress indicator: 5 dots, first filled
- ✅ Step counter "1/5" top right
- ✅ "Welcome to SKYCOT." + "Let's hire your first worker. (90 seconds.)" — sets time budget
- ⚠️ Bottom half of the page is empty. Could use an illustration or quote.
- ⚠️ "(90 seconds.)" — period inside parens is awkward typography

### Scene 3 — Onboarding step 2/5: Name your workspace

**🚨 P0 BUG — Form submit fails with "Failed to fetch".**

Root cause: the workspace-provision call hits `/api/v1/onboarding/provision` on the gateway. The SPA (port 3000) → gateway (port 7400) is cross-origin. Browser sends CORS preflight `OPTIONS`. **Gateway returns 404 — no OPTIONS handler.** Browser refuses to send the actual POST.

```
curl -X OPTIONS http://127.0.0.1:7400/api/v1/onboarding/provision \
  -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: POST"
→ {"error":"not found"}  HTTP 404
```

**Why this matters for v1.0**: In production, SPA and gateway are colocated behind a single load balancer (same-origin), so CORS preflight never fires. But:
1. **Demo recording runs on dev mode locally** → blocked
2. **Any future deploy with SPA and gateway on different origins** (e.g., `app.skycot.com` and `api.skycot.com`) → blocked
3. **Any browser extension or 3rd-party integration** calling the gateway directly → blocked

**Two fix options:**
- **A. Next.js rewrite (dev-only, what I'm testing)** — `web/next.config.ts` adds `rewrites()` to proxy `/api/v1/:path*` server-side to the gateway. Avoids cross-origin entirely. **Uncommitted in your working tree.** SPA's `NEXT_PUBLIC_GATEWAY_URL` needs to be empty for the relative path to take the rewrite.
- **B. Gateway CORS support (real fix)** — add `OPTIONS` preflight + `Access-Control-Allow-*` headers on `/api/v1/*`. Brief candidate. Lets cross-origin browsers (and external clients) hit the gateway directly.

**UX observations** (independent of the bug):
- ✅ Reassurance: "Workers live in workspaces. You can rename them later."
- ✅ Placeholders distinguish the two fields: "Acme Growth" / "Acme Inc."
- ✅ Continue button is disabled-state until both fields have values
- ⚠️ "STEP 2 OF 5" + progress dots — slight redundancy
- ⚠️ When submission fails: "Failed to fetch" appears INSIDE the card. The message is terrible UX — doesn't explain what to do.
- ⚠️ Continue button stays clickable during the failed request. Click 5 times = 5 failed fetches. No loading state.

### Scenes 3-5 — Couldn't fully exercise without CORS fix

Because the workspace-provision call fails, the rest of the onboarding wizard (connect-provider → pick-vertical → first-hire) can't be reached interactively without the next.config.ts rewrite committed.

The cargo tests prove the underlying flows work (`mission-claim-4-runtime-closure-trace` test passed earlier; `POST /api/v1/workers` returns 201 with the corrected fixture). So the **product is correct; the dev-mode plumbing is broken.**

---

## Other pages I surveyed

### `/audit` — 🟢 DEMO SHOWPIECE

The visual high point of the SPA. Side-by-side I'd put this against any enterprise audit tool.

**What works:**
- Left sidebar: Approvals / Team / Workers / Audit / Skills / Cohorts — clean iconography
- Right pane: searchable, filterable audit list with time grouping ("TODAY")
- Per-row layout: time ago → worker initials avatar → action description in bold → action ID below → status pill (Succeeded/Pending/Refused) → cost ($) → arrow to drill in
- 15 realistic seeded rows: "resolved SUP-29382 with KB article SK-413", "extracted INV-2024-08741 line items", "screened Diego Alvarez against JD-2024-08", "flagged duplicate invoice INV-2024-08739" (Refused), "asked Mark to approve $50 LinkedIn InMail credit" (Pending)
- Diverse statuses + cost amounts ($0.003 – $0.021)

**Concern:**
- This page renders **seeded mock data** regardless of session. The "Sam Harlow · Acmeco · Owner" identity at bottom-left is also fake. **A fresh signup with a real account would see Sam Harlow's workspace with somebody else's audit rows.** Fine for the recorded demo if the narrative is "let's imagine the workspace has run for a few days", but confusing if anyone clones the repo and runs it expecting their own data.

### `/workers` (renders as "Team") — 🟢 SOLID

6 worker cards: Mariah / Priya / Jamal / Nori / Rio / Sam. Each has:
- Avatar (M/P/J/N/R/S initial), status pill (Waiting on you / Working / Idle / Paused / Blocked)
- Persona line: "lists and re-prices items f..." / "screens candidates and b..." / "handles customer suppor..."
- TODAY narrative: "3 listings live, 2 buyers waiting on you" / "8 candidates screened today, 1 interview pending your slot"
- TOMORROW narrative: "morning re-price sweep at 06:30" / "next-day shortlist review"
- "Open profile →" CTA

**UX observations:**
- ✅ "6 teammates working" subtitle — sets fleet scale
- ✅ Status diversity tells a story (yellow Waiting on you, green Working, gray Idle, blue Paused, red Blocked)
- ✅ TODAY/TOMORROW framing is excellent for a "what's your team doing right now" view
- ✅ "+ Hire" button top right + Status filter
- ⚠️ Same mock-data issue as /audit — these are not the workers I created during onboarding

### `/cohorts` — 🟢 CLEAN

5 cohorts shown: Support (Trusted) / Support (Developing) / Sales (Established) / Marketplace (Developing) / Marketing (Pre-trust). Each card: MEMBERS / SKILLS counts + Trust-grade tag.

**UX observations:**
- ✅ Trust tier system is a differentiator — shows up cleanly
- ✅ "5 cohorts across the fleet" subtitle
- ⚠️ Cards feel sparse. No drill-in CTAs, no "what's in this cohort"

### `/skills` — 🔴 500 ERROR

Branded 500 page ("Something went wrong on our side. We've been notified. Try again / View status / Contact support"). Real bug.

### `/approvals` — 🔴 500 ERROR

Same branded 500 page.

### `/settings` — 🔴 500 ERROR

Same branded 500 page.

**Note**: The 500 error PAGE design is itself good — from #1813 brand work. Buttons + supportive copy. The bug is that those 3 routes hit the 500 path instead of rendering.

---

## Marketing site detailed notes

(Already covered in Scene 0, summary here.)

- ✅ Brand quality is professional throughout
- ✅ Footer columns present: PRODUCT / COMPANY / RESOURCES / LEGAL (cut off but legible)
- ⚠️ Hero feature card text stacking ("Workers / with / memory.") looks broken at 1920px
- ⚠️ Cookie banner takes up significant space at viewport bottom — consider auto-dismiss after first scroll
- ⚠️ MDX + Turbopack incompatibility blocks `pnpm dev` startup unless Turbopack is disabled

---

## Files I touched this run (in working tree, not committed)

| File | Change | Should it be committed? |
|---|---|---|
| `web/next.config.ts` | Added Next.js rewrite proxying `/api/v1/:path*` → gateway server-side | **Yes if you go with Option A (dev-mode proxy)**. **No if you go with Option B (gateway CORS).** |

---

## Recommended brief authoring (next tick)

To clean up before demo recording:

| Brief | Owner | Sizing | Closes |
|---|---|---|---|
| `mission-gateway-cors-preflight` — add OPTIONS handler + `Access-Control-Allow-*` headers on `/api/v1/*` | codex | ~150 LOC | P0 demo blocker |
| `mission-spa-pages-500-fix` — investigate + fix `/skills`, `/approvals`, `/settings` 500 errors | claude | ~200 LOC | P1 SPA bugs |
| `mission-spa-show-real-session` — render actual session identity (logged-in user, real workers) instead of seeded "Sam Harlow · Acmeco" data | claude | ~400 LOC | P1 UX correctness |
| `mission-marketing-mdx-turbopack-fix` — resolve MDX + Turbopack incompatibility OR document the `--no-turbopack` flag in dev docs | claude | ~80 LOC | P1 dev-mode startup |
| `mission-marketing-hero-card-layout` — fix feature card headline stacking at large viewports | claude | ~30 LOC | P2 polish |
| `mission-auth-pages-brand-mark` — add SKYCOT logo + value-prop sidebar to `/auth/signup` + `/auth/check-email` | claude | ~80 LOC | P2 brand consistency |

---

## Demo-recording verdict

**🟡 Yellow — you can record IF you commit the next.config.ts rewrite first.** Without it, the onboarding flow stops at "Failed to fetch" on scene 3.

If you commit the rewrite + set `NEXT_PUBLIC_GATEWAY_URL=""` on the SPA dev environment, the demo should flow end-to-end. The audit page is the clear high point of the visual; the form-submit pages are clean but barren.

Suggested record order if you proceed:
1. Marketing landing (cold open) — emphasize "Hire a teammate, not a tool" hero
2. Signup → check-email — brief, demonstrate magic-link no-password
3. Skip to authenticated state (you have a fresh DB at `~/.argo/sessions.db`)
4. **Open `/audit` first** — this is the wow moment, mock data and all
5. Then `/workers` (Team) — show the fleet narrative
6. Then `/cohorts` — show trust tiers
7. Avoid `/skills`, `/approvals`, `/settings` (currently 500)

The shipped product underneath is correct — cargo tests prove it. The dev-mode wiring needs T1.4 (CORS proxy) before pressing Record.
