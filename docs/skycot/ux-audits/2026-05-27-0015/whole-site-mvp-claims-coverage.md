# Whole-site MVP claim coverage + load-bearing rollup

**Audit run**: 2026-05-27 00:15 AEST (14:15 UTC)
**HEAD**: `9c407cd20` (status: claim fix-gateway-mod-rs-loc-overage-extract-store-helpers) — == origin/main
**Prior whole-site audit**: 2026-05-26 23:09 AEST (HEAD 2cc2ec42b) — ~55 minutes earlier
**Gateway**: 7400, demo-mint tenant `019e5057-1d3f-7fa2-9c1b-c34ef01cc21e` — alive (`/health` → 200)
**SPA**: `pnpm dev` on :3000 via launch.json with `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_ARGO_DEMO_MODE=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`
**Method**: `mcp__Claude_Preview__*` (snapshot/screenshot/console/network until the renderer locked up mid-run) + `curl` against gateway + source-code grep across `web/`. No cargo, no mutations.
**Auditor**: e2e-ux-journey-test-agent (whole-site rollup scope)
**Scope notes**: 4 sibling agents are covering Scene-1+2, Scene-4+5, Scene-6+approvals, and onboarding. This report deliberately stays out of those flows and covers the gaps: marketing perimeter (×13 routes), `/cohorts`, `/settings` + 12 sub-pages, `/brand`, `/team`, `/workflows/templates`, theme toggle, viewport responsiveness, plus the 6-MISSION-claim rollup. There is no `/skills-canvas` route in the repo.

## TL;DR

26 findings: **5 P0** · **9 P1** · **9 P2** · **3 P3**. The biggest load-bearing surprise this run isn't on the surface — it's underneath: the dev-run `launch.json` sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` (the **legacy short name**), but every API helper in `web/lib/api/*.ts` reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (the **prefixed new name**). The auth-bypass dev shortcut **is not firing for any SPA surface in this run** — every page falls through to the real gateway, gets sparse data, then either renders an empty state or trips the `isDemoMode()` second-chance fallback. That single character drift explains the persistent "MOCK-OFFLINE" pill on `/team`, the one-cohort `/cohorts` page (gateway returns 1, bypass never triggers, magazine-grid never paints), and why the prior-run audit kept noting "the SPA seed renders" without the seed actually loading via the intended bypass path. Five P0s carry over from prior runs (workflows dark-mode unreadable, Team+Workers dual nav, MOCK-OFFLINE chip on camera, /docs sub-routes all 404, /genesis orphaned + 503) — those are stable; the env-var-name finding (F1 below) is **new in this run** and would close several of them on a one-character fix in `.claude/launch.json`. The 4 vertical-landing pages additionally over-claim regulatory compliance (GDPR · HIPAA · AFSL) on every page regardless of vertical, against a `/security` page that honestly says "SOC 2 attestation in progress" — that's a legal/marketing mismatch worth flagging. Orientation score: **66 / 100** (down 2 from 68 — the env-var finding compounds against the prior-run gaps rather than replacing them).

## Headline blockers (P0 only)

1. **F1 [P0]** — Env-var name drift between `.claude/launch.json` (`NEXT_PUBLIC_GATEWAY_AUTH_BYPASS`) and every consumer in `web/lib/api/*.ts` (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`). The dev auth-bypass shortcut never fires, every page hits the real gateway, sparse data shows through.
2. **F2 [P0 — carry-over]** — `/workflows`, `/workflows/templates`, and `/workflows/{id}` cards still have **8 hardcoded `background: white` declarations** across `page.css`. Unreadable in dark mode (the operator's preferred recording aesthetic). The `fix-workflows-dark-mode-card-white-on-white` brief is in flight but not merged.
3. **F3 [P0 — carry-over]** — `/genesis` is alive (renders "Describe your AI teammate" wizard) **but has zero links pointing at it from any nav / CTA / sidebar** (`grep -rn 'href="/genesis' web/` → no matches). `POST /api/v1/genesis` returns **503**. The investor-moment surface per memory `feedback_genesis_is_ai_teammate_onboarding_studio` is currently unreachable. `fix-genesis-orphan-discoverability-and-provider-key-503` brief queued — not merged.
4. **F4 [P0 — carry-over]** — Sidebar nav exposes **both "Team" and "Workers"** as separate rows in `web/components/skycot/nav-items.tsx:24-25`. Clicking Workers shows a `Loader2` spinner then bounces to `/team`. Two visible nav rows for the same surface.
5. **F5 [P0 — carry-over]** — `/docs` lists 10 sub-routes (`/docs/api`, `/docs/audit`, `/docs/getting-started`, `/docs/workflows`, etc.). **All 10 return 404**. Marketing nav → Docs → click any deep link → 404. `fix-docs-sub-routes-all-404` shipped as PR #2416 → still in review per `git log --grep=2416`.

## All findings, in order

### F1 [P0] — Env-var name drift: `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` vs `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`

The dev runner `.claude/launch.json:9` exports `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`. Every consumer in `web/lib/api/*.ts` (and `web/lib/hooks/use-approval-bundles.ts`, `use-viewer-policy.ts`, `use-redaction-policy.ts`) reads the **prefixed** name `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`. The two strings differ by 5 characters (`_ARGO`).

```
$ grep -rn "NEXT_PUBLIC_GATEWAY_AUTH_BYPASS\b" web/ — 0 matches
$ grep -rn "NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS" web/lib | wc -l — 17 matches
```

Result: the dev auth-bypass shortcut never fires for any SPA surface in this `pnpm dev` run. Every page hits the gateway, gets sparse data, and either shows an empty state, falls through to the secondary `isDemoMode()` mock-offline path, or renders the gateway's 1-cohort / 0-skill / 0-worker reality. This **explains** the prior audit's complaint that `/cohorts` shows only 1 dept tab (gateway returns 1 cohort, bypass doesn't kick the SAMPLE_COHORT_SUMMARIES path), the persistent MOCK-OFFLINE pill on `/team` (mock-offline is the fallback after gateway 404 for `/workers`, not the intended mock-bypass), and the audit feed's all-`http.request` rows (self-generated, no seeded agent actions).

The fix is **one character in `.claude/launch.json`**: change `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` to `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. Existing tests (`web/lib/api/skills.test.ts:55,112`) already expect the prefixed name. A worktree at `.claude/worktrees/intelligent-gagarin-b98b47/` still has the legacy name in its `skills.test.ts` and `skills.ts` — that's the rename history showing through.

**Repro**: `grep -rn "NEXT_PUBLIC_GATEWAY_AUTH_BYPASS" web/lib/` returns 0 matches; the same grep with `ARGO_` returns 17.
**Suggested fix**: rename in `.claude/launch.json:9` (single character delete-the-prefix → add `_ARGO_`).

### F2 [P0 — carry-over] — Workflows dark-mode: 8 hardcoded `background: white` declarations across cards

`web/app/(app)/workflows/page.css` lines 59, 110, 146, plus one more = 4 white-bg rules. Same file count in `web/app/(app)/workflows/templates/page.css`. Total: **8 hardcoded `background: white` declarations** that don't respond to `data-theme="skycot-dark"`. Workflow names + TRY pill chips render white-on-white in dark mode. Demo recording aesthetic is dark; this trips the camera. `fix-workflows-dark-mode-card-white-on-white` brief queued T2309MEL → not yet merged on `origin/main` (`git log --grep="workflows-dark"`).

**Repro**: `grep -cE "^\s*background:\s*white" web/app/\(app\)/workflows/page.css web/app/\(app\)/workflows/templates/page.css` → 4 each.
**Suggested fix**: replace `white` with `var(--card)` / `var(--surface)`; replace `color: var(--foreground, #1e1b4b)` with `var(--foreground)` (no hardcoded fallback that breaks dark theme).

### F3 [P0 — carry-over] — `/genesis` orphaned: 0 nav links + POST 503

`web/app/(app)/genesis/page.tsx` exists and renders the "Genesis — describe a role" / "Describe your AI teammate" wizard. **Zero `href="/genesis"` strings exist anywhere in `web/`** (`grep -rn 'href="/genesis' web/` → no matches). The page is reachable only by typing the URL. Operator memory `feedback_genesis_is_ai_teammate_onboarding_studio` says this is **the** investor moment — the thing that distinguishes Skycot from "ChatGPT with prompts". Backend layer: `curl -X POST :7400/api/v1/genesis -d '{}'` → **503**, so even if a CTA existed the wizard's create call would fail.

**Repro**:
```
$ grep -rn 'href="/genesis' ~/code/argo/web/ — 0 matches
$ curl -s -o /dev/null -w "%{http_code}\n" -X POST -H "Content-Type: application/json" \
    -d '{}' http://127.0.0.1:7400/api/v1/genesis — 503
```
**Suggested fix**: (a) add `{ href: "/genesis", label: "Onboard a teammate", icon: <Sparkles /> }` to `web/components/skycot/nav-items.tsx` OR add a primary CTA on `/onboarding/welcome` that routes to `/genesis`. (b) Fix `POST /api/v1/genesis` 503 — separate gateway brief.

### F4 [P0 — carry-over] — Sidebar nav has "Team" and "Workers" as separate rows

`web/components/skycot/nav-items.tsx:23-30`:
```ts
{ href: "/", label: "Approvals", icon: <Inbox /> },
{ href: "/team", label: "Team", icon: <Users /> },
{ href: "/workers", label: "Workers", icon: <Workflow /> },
```
Two adjacent sidebar rows for the same data. `web/app/(app)/workers/page.tsx:19-46` documents it: "When workers exist, redirect to /team". Click sequence: sidebar → Workers → `Loader2` spinner → `/team`. Two-row IA for one surface.

**Repro**: load any in-app page; count sidebar rows; observe Team THEN Workers as adjacent rows.
**Suggested fix**: drop `{ href: "/workers", … }` from `NAV_ITEMS`. Let `/workers` remain as a deep-link entry-point that always redirects to `/team`. The empty-state CTA still works from `/team` (`HireStarterPrompts` is shared).

### F5 [P0 — carry-over] — All 10 `/docs/<sub-route>` links return 404

The `/docs` index page renders 10 deep-link cards: `/docs/api`, `/docs/audit`, `/docs/audit/trust-ratchet`, `/docs/getting-started`, `/docs/getting-started/hire-first-teammate`, `/docs/integrations`, `/docs/integrations/slack-approvals`, `/docs/skills`, `/docs/workflows`, `/docs/workflows/visual-canvas`. **Every one returns 404.** Marketing nav → Docs → click anything → 404 page. PR #2416 `fix-docs-sub-routes-all-404` is in review per `git log --grep=2416` — landed as a status row but not merged yet.

**Repro**:
```
$ for p in api audit audit/trust-ratchet getting-started getting-started/hire-first-teammate integrations integrations/slack-approvals skills workflows workflows/visual-canvas; do \
    printf "%-40s " "docs/$p"; curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3000/docs/$p; \
  done
```
**Suggested fix**: ship PR #2416 — adds stub MDX pages with "coming soon" copy + the canonical docs schema.

### F6 [P1] — Vertical landing pages over-claim compliance: `GDPR · HIPAA · AFSL` on every page

`web/app/for/_components/VerticalLandingShell.tsx:631` renders the trust strip `GDPR · HIPAA · AFSL` on **every** `/for/<vertical>` page regardless of vertical. HIPAA (US healthcare) and AFSL (Australian Financial Services License) are regulated, specific claims. They appear on `/for/sales`, `/for/marketing`, `/for/recruiting`, `/for/ops` where the vertical has no plausible relationship to health-care PHI or AFSL-regulated activity. The `/security` marketing page is honest in contrast — it says "SOC 2-ready audit trail" and "External SOC 2 Type II attestation is in progress", with **no** HIPAA / ISO 27001 / AFSL claim. The vertical-landing claims are inconsistent with the security page and legally adventurous.

**Repro**: `grep -n "GDPR · HIPAA · AFSL" web/app/for/_components/VerticalLandingShell.tsx` → line 631; `grep -rn "HIPAA\|AFSL" web/app/security/` → no matches.
**Suggested fix**: replace with vertical-appropriate honest claims. Sales/Marketing pages → "SOC 2-ready audit trail · GDPR-aligned data export". Finance → "SOC 2-ready · SOX-aligned audit trail". Healthcare-specific vertical (if created) → keep HIPAA. Drop AFSL until you actually hold one.

### F7 [P1 — carry-over] — Marketing perimeter introduces non-canonical worker names (Skylar, Fen, Mira)

`web/app/for/_data/verticals.ts` introduces named workers in the signature-workflow cards: Skylar (sales, lines 90-94), Fen (finance, lines 246-249), Mira (marketing, lines 298-301). The canonical demo roster across `/landing`, `/team`, `/workers` and the v1.0 runbook is **Mariah / Peta / Priya / Jamal / Nori** — 5 names. A visitor reading `/for/sales` who clicks through to `/team` sees no Skylar. The vertical pages introduce 3 worker names a customer cannot find in the product. Recruiting (`Priya`), Support (`Jamal`), Ops (`Patrick` — canonical for the M21/M22 Project & Ops cohort per memory `project_pm_cohorts_peta_patrick`) are consistent. Sales / Finance / Marketing are not.

**Repro**: `grep -nE 'actor: "[A-Z]' web/app/for/_data/verticals.ts` — observe Skylar, Fen, Mira.
**Suggested fix**: rename Skylar→Mariah (or her vertical-counterpart), Fen→Peta (or a Project & Ops finance variant), Mira→Nori. Worker-name discipline is a brand-consistency concern, not a runtime concern.

### F8 [P1] — All 6 vertical-landing pages reference non-existent OG images (`/og/og-<vertical>.png` → 404)

`web/app/for/_data/verticals.ts` lines 64, 113, 165, 217, 269, 321 set `ogImage: "/og/og-<vertical>.png"`. Each `/for/<vertical>/layout.tsx` consumes that into the page's `openGraph.images` metadata. The `web/public/og/` directory **does not exist**. All 6 paths return 404:

```
$ for f in og-sales og-recruiting og-support og-finance og-marketing og-ops; do \
    printf "%-15s " "$f"; curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3000/og/$f.png; \
  done
og-sales        404
og-recruiting   404
og-support      404
og-finance      404
og-marketing    404
og-ops          404
```

Any LinkedIn / X / Slack share of these URLs will render a broken-image OG card. Marketing perimeter footgun.
**Suggested fix**: either (a) generate the 6 OG images via the dynamic Next.js OG endpoint pattern already used at `web/app/opengraph-image.tsx`, OR (b) point `ogImage` at `/opengraph-image` (already 200) until per-vertical art exists.

### F9 [P1 — carry-over] — `/team` MOCK-OFFLINE diagnostic chip visible to demo viewers

`web/app/(app)/team/page.tsx:318-329` defines `TeamDataSourceChip` which renders `"MOCK-OFFLINE"` in dev mode (production-only is hidden). The sibling `DataSourceChip` at `web/components/skycot/DataSourceChip.tsx:35` explicitly suppresses `mock-offline` "to avoid leaking 'this data is fake' onto camera during the v1.0 demo runbook" — but `TeamDataSourceChip` doesn't share that suppression. The chip is visible right next to the page-title `Team` heading. If F1 is fixed (env-var name), this chip would show `MOCK-AUTH-BYPASS` instead, which is also visible in dev. Two chip implementations with diverging suppression rules.

**Repro**: load `/team`, look at the header next to the word "Team".
**Suggested fix**: align `TeamDataSourceChip` with `DataSourceChip` — suppress `mock-offline` (and any new mock source) in dev unless `?debug=1` is present.

### F10 [P1] — `/team` hero "0 working right now · 935 actions this week · saving you ~60 hours"

The brand-loud hero on `/team` reads "0 working right now" against "5 teammates, one team" + 5 visible cards each showing "Waiting on you". This is the same finding as the prior audit (F13). The `0` reads as a contradiction. The other two KPIs (935 / 60h) are dressing — fine. The `0` is misleading function-side text the camera will catch.

**Suggested fix**: either populate from the real audit stream (`SELECT COUNT(*) WHERE status='in_progress' AND last_action > now() - interval '15 min'`) or reword: "5 teammates ready · 935 actions this week · saving you ~60h".

### F11 [P1 — carry-over] — `/cohorts` shows ONE cohort and ONE department tab ("Other")

Gateway returns 1 cohort (`marketing-intern::unknown`); `web/app/(app)/cohorts/page.tsx:117` only triggers the magazine-grid layout when cohorts > 0 (the page never falls through to the `CohortsEmptyPreview` branch because gateway returns a non-empty array). The locked design (`project_cohorts_page_design_locked`, D3 magazine tiles with 6 dept tabs incl. Project & Ops with Peta + Patrick) is therefore not visible. A fix to F1 (env-var name) would route the page through `SAMPLE_COHORT_SUMMARIES` (13 cohorts → 6 dept tabs) and the locked design would render.

**Repro**: `curl http://127.0.0.1:7400/api/v1/cohorts | python3 -m json.tool` → `total_count: 1`. `wc -l web/lib/sample-cohorts.ts` → 13 cohorts in the seed. Load `/cohorts` → single tab.
**Suggested fix**: depends on F1. Independently, the `cohorts.length === 0` empty-state branch in `page.tsx:105` could be extended to `cohorts.length < 4 && isDemoMode()` so the preview kicks in when the gateway returns a degenerate count too.

### F12 [P1] — `/settings/providers` is read-only — no inline "use my API key" form

`/settings/providers` renders the static "Primary provider: Anthropic / Fallback: Fail closed / Per-worker overrides: Audit logged" copy with a sidelined "Review API keys" link to `/settings/api-keys`. The Pro-tier path (per `/pricing`) requires the operator to plug in their own provider keys. The setup page exposes only metadata; the actionable form is one click away. For the v1.0 enterprise positioning, the providers page should be the place a buyer drops in their key.

**Repro**: load `/settings/providers`, observe no input fields, only navigation.
**Suggested fix**: inline a "Use my API key" form (provider picker + key field + save) on the providers page. The `/settings/api-keys` page can remain as the audit / rotation surface.

### F13 [P1] — Preview renderer locks up after demo-mint POST flow

While auditing, the preview browser's eval/click/screenshot all started timing out after the page navigated through `/auth/signup`. The dev-server log showed `POST /api/auth/demo-mint 200` followed by `GET /workers/demo-worker-receptionist-mclr0xqz/overview` and then `POST /api/auth/demo-mint 400` (recovery) `POST /api/auth/demo-mint 200`. The renderer was stuck for the remainder of the audit (eval timeouts at 30s, screenshots returning the same `/auth/signup` view despite log activity). Suggests a redirect / suspense / promise-not-resolving in the demo-mint path that's blocking the React renderer's event loop.

**Repro**: load `/landing`, click "Hire your first teammate" → `/auth/signup` → click "Enter demo workspace" with a placeholder email → wait. Confirm the eval/click MCP tools time out.
**Suggested fix**: add a 5-second client-side timeout to the demo-mint flow with a visible "stuck? try again" CTA; investigate the 400-then-200 retry pattern (probably an idempotency-key race condition).

### F14 [P2] — `/settings/security` hero "Security." renders as light-purple gradient text on dark-mode bg (low contrast)

Same pattern as the prior audit (F17). Brand register prefers gradient hero headings, but at the page-title position the contrast drops below WCAG AA (~3:1 for hero scale). The page is internal-team / operator-facing in current scope, so not a demo blocker; if a customer ever lands here it's a P2 polish.

### F15 [P2] — `/brand` lives at `/(app)/brand/page.tsx` — exposed via direct URL inside the app shell, not linked from any nav

`web/app/(app)/brand/page.tsx` (227 LOC) is an internal design-system reference (color tokens, typography samples, wordmark panel). It renders inside `(app)/layout.tsx` so the sidebar nav is present, the demo banner is on top, and a logged-in user can land here by typing the URL. Not linked from anywhere a customer would discover it. Either link from `/settings` as a developer-reference (with a "internal designer-only" badge), or move to `_dev/brand` or `app/(setup)/brand` so it doesn't pollute the customer route tree.

### F16 [P2] — `/customers/<slug>` cards carry over-specific industry claims ("4.2 million orders", "HIPAA-compliant out of the box", "AFSL-holder")

Dressing-side, per the function-vs-dressing rule. Not a POC demo blocker (the runbook doesn't visit `/customers`). But the case-study copy at `web/app/customers/<slug>/page.tsx` reads as if these were real customers; an investor browsing in another tab might assume they are. Recommend adding a `<p data-pending-real-quote="true">` flag at the top of each case-study + footnote: "Sample tenants for product preview — replaced with real attributable customer stories on public launch."

### F17 [P2 — carry-over] — `/for/<vertical>` social-proof "2,847 teams this month" hardcoded on every page

`web/app/for/_components/VerticalLandingShell.tsx:635` renders "2,847 teams this month" on every vertical-landing page. Per the function-vs-dressing rule this is **dressing — permitted for POC scope**. Flagged only because a public-launch polish pass will need to either replace with real telemetry or remove (CLAUDE.md cites "1,122 weekly actions" as another instance of the same pattern in `feedback_no_theater_in_demo`).

### F18 [P2] — `/pricing` Free tier reads as a toy ("Mock provider only", "Single channel (web inbox)")

Carry-over from prior audit (F8). Not a POC demo blocker (the runbook doesn't show pricing). Worth softening to "Test SKYCOT in a sandbox without credit card" + "Production-ready with your own API key" so a real visitor doesn't conclude the free product is non-functional.

### F19 [P2] — `/help` is a permanent 307 redirect to `/docs` (which has 10 broken sub-routes)

`curl -sIL http://127.0.0.1:3000/help` → `307 → /docs`. The /docs landing renders, but `/docs/<sub-route>` returns 404 (F5). A customer typing `/help/quickstart` or arriving via a docs-bot link will land on the broken sub-route. Reasonable trade-off (one source of truth) but exacerbates F5.

### F20 [P2] — `/settings/connectors` Connect CTA flow not E2E-verified

29 catalog cards render with "Not connected" + "Connect" CTAs. The click-through to OAuth or stub flow was not verified in this audit (preview renderer locked up — F13). Surface looks production-ready; the actual handler is unverified. Worth a focused journey test of one OAuth path before claim-5 demo recording.

### F21 [P2] — `/settings/data-export` + `/settings/notify-channels` + `/settings/knowledge` not verified for actionable behavior

All three return 200, but the content + actionability was not exercised in this audit (renderer constraint). They appear in the settings nav. Worth a 5-minute sweep before recording: do "Export audit trail" / "Add Slack webhook" / "Upload SOP PDF" actually do something, or are they stubs?

### F22 [P2] — Multiple `/api/v1/<path>` 404s noted (workers, tenants, connectors, openapi.json, skills/learned-this-month)

```
$ for p in workers tenants connectors openapi.json skills/learned-this-month; do \
    printf "%s " "$p"; curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:7400/api/v1/$p; \
  done
workers 404
tenants 404
connectors 404
openapi.json 404
skills/learned-this-month 404
```

Each one causes a SPA fallback path to fire. None are necessarily new bugs (workers is canonicalized to /team per a shipped PR), but the absence of `openapi.json` from the gateway means there's no machine-readable surface contract — operator + future tool authors have to read source. The `skills/learned-this-month` endpoint is fetched but missing, so the marketplace's "skills learned this month" pill shows zero.

### F23 [P3] — `/customers` listing page CTA "Read story →" is decorative span inside whole-card Link

Same as prior F9. The card is the clickable target (correct accessibility); the CTA span is non-focusable. Visually reads like a button but isn't one. Polish-level finding.

### F24 [P3] — Marketing nav doesn't visibly carry an "Apply for early access" affordance distinct from "Hire your first teammate"

The landing trust line says "EARLY ACCESS · APPLY" but the only CTA is "Hire your first teammate" which routes to `/auth/signup`. If "early access" means a different flow (e.g. waitlist), it isn't reachable from the nav. If they're the same, the eyebrow is misleading.

### F25 [P3] — Marketing pages mix "teammate" (modern) and "worker" (legacy code-canonical) in copy

Per memory `project_vocabulary_duty_vs_schedule` the operator-locked convention is UI = customer-facing terms, code = canonical primitive. UI copy generally uses "teammate". But `/landing` how-it-works step-02 says "SKYCOT spins up a named **worker** (Mariah, Peta, Priya, Jamal, or Nori)…" — `worker` leaking into customer copy at the very moment the page is teaching the reader what a teammate is. Same drift in `/landing` step 04 ("audit-logged with **worker**, action, cost, latency"). Minor — but visible.

### F26 [P3] — Maintenance + offline pages render correctly (200) but with `/landing`-style nav, which feels off for a maintenance state

`/maintenance` and `/offline` both return 200 with full marketing nav (How it works / Features / Compare / Roster / Pricing / Docs / Changelog). A user who lands on `/maintenance` should see something minimal — current page suggests "we're down, but please browse our pricing." Polish-level only.

## Load-bearing rollup — the 6 mission claims, SPA-visibility only

Per `docs/skycot/MISSION.md` (operator-locked) and the function-vs-dressing rule. CLAUDE.md records all 6 as LIVE in the runtime. This rollup asks "do the claims work functionally **in the SPA's MVP scope**?" — a different and narrower question.

### Claim 1 — Teammate, not tool · 🟡 partially visible

- ✅ `/landing`, `/team`, `/workers/{id}` use teammate-first vocabulary ("5 teammates, one team", traits chips, "How Mariah works", great-at / escalate-when).
- ✅ Workers carry names + personalities, not function-codes (Mariah/Peta/Priya/Jamal/Nori).
- ⚠ `/for/<vertical>` pages introduce 3 non-canonical names (Skylar, Fen, Mira) — F7. Vocab drift in step 2 + 4 of /landing how-it-works ("worker") — F25.
- ⚠ Sidebar nav has both Team + Workers (F4).
- ⚠ `/team` hero says "0 working right now" against 5 visible teammates (F10).

### Claim 2 — Delegation is the unit · 🟡 partially visible

- ✅ `/workflows/{id}` shows step-typed playbooks (TRIGGER → SKILL → DECISION → SKILL) with plain-English summaries (verified in prior audit).
- ✅ Builder at `/workflows/build` exposes the plain-English propose pattern.
- ⚠ Workflow cards unreadable in dark mode (F2).
- ⚠ `/approvals` ops-dashboard renders but `/` inbox shows "Loading approvals…" indefinitely (gateway returns empty + the loading state never resolves) — observed in this audit's screenshot.

### Claim 3 — Platform builds workers · 🔴 regression

- ✅ `/onboarding/welcome` ships the F1-locked editorial-center wizard (verified in prior audit).
- 🔴 **`/genesis` is orphaned (F3)** — the load-bearing investor-moment surface is unreachable from any nav/CTA, AND `POST /api/v1/genesis` returns 503. Backend + frontend both broken. The 4-card "Build your AI teammate" wizard renders but cannot complete.
- ⚠ `/settings/providers` is read-only (F12) — no inline "use my API key" form.

### Claim 4 — Skills shared + learned · 🔴 not visible in SPA

- ✅ `/skills` marketplace renders the A1-locked design.
- 🔴 **Skill detail page has no install / add-to-worker CTA** (prior audit F19 — still open). The runbook's "click result → install flow" doesn't exist.
- ⚠ Gateway returns 0 skills; SAMPLE_TEAM seeded 6 visible cards via demo-mode fallback. The "skills learned this month" pill fetches `/api/v1/skills/learned-this-month` which 404s (F22) so the count shows zero — undermines the claim's "learned" word.

### Claim 5 — Any-PC execution · 🟡 not visible in SPA

- ✅ `/settings/connectors` catalog visible (29 cards) but Connect flow not E2E-verified (F20).
- ⚠ No SPA surface for computer-use macOS visible from a customer perspective. Likely lives behind "Run now" on a workflow with a computer-use step — Scene 5's purview.
- ⚠ Operator-blocking per CLAUDE.md item (c): macOS perms not granted on argo binary.

### Claim 6 — Multi-channel + multi-tenant · 🟡 partially visible

- ✅ Workflow first-step copy enumerates "Slack DM, email, or web inbox" — 3 channels in one trigger description.
- ✅ Builder palette enumerates 6 trigger types.
- ✅ Account menu shows "Sam Harlow · Acmeco · Owner" — tenant context surfaced.
- ⚠ No SPA surface for switching tenants. `/api/v1/tenants` returns 404 (F22). Cross-tenant isolation is invisible from this view.

### Composite verdict

| Claim | Runtime (per CLAUDE.md) | SPA visibility (this audit) | Shift vs prior whole-site audit |
|---|---|---|---|
| 1. Teammate not tool | 🟢 LIVE | 🟡 (vocab drift + 2-row nav + 0-working) | stable |
| 2. Delegation is the unit | 🟢 LIVE | 🟡 (dark-mode + loading-approvals never resolves) | stable |
| 3. Platform builds workers | 🟢 LIVE | 🔴 (Genesis orphan + 503) | stable (was 🟡, downgraded by DELTA correctly) |
| 4. Skills shared + learned | 🟢 LIVE | 🔴 (install flow missing) | stable |
| 5. Any-PC execution | 🟡 perms-pending | 🟡 (connector flow unverified) | stable |
| 6. Multi-channel + multi-tenant | 🟢 LIVE | 🟡 (channels visible, tenant-switch invisible) | stable |

Runtime is ahead of SPA on 5 of 6 claims. The single biggest SPA-only gap remains Claim 4 (no skill-install). Claim 3 (Genesis orphan + 503) is the second-biggest gap and the only one that affects the investor-moment surface.

## Resolved since prior audit (2026-05-26 23:09 AEST)

- HEAD moved from `2cc2ec42b` → `9c407cd20` (~6 commits, mostly status flips). The 5 P0s from prior whole-site audit are **all still open** (no P0-closing merge in the 1-hour window). PR #2418 `fix-demo-banner-sidebar-overlap-desktop` is in review and tangentially related to F9 (chip layout) but doesn't close it.
- PR #2419 `fix-hire-demo-mode-stub-uuid-worker-roster-invisibility` is in review — that closes a Scene-1 P0 from the onboarding sibling audit but is out of this report's scope.

## Orientation score: 66 / 100

Marketing perimeter is solid on the surface but carries (a) HIPAA/AFSL claims with legal/regulatory weight on every vertical-landing page regardless of vertical, (b) 6 broken OG images that will burn every social share, (c) 10 broken `/docs/<sub-route>` deep links. The audit feed remains placeholder-only. The Genesis investor-moment surface is unreachable AND its POST endpoint 503s. The env-var-name drift between `launch.json` and the API helpers is a one-character fix that would unblock several of the prior P0s by triggering the intended `mock-bypass` (seeded-data) path everywhere instead of the `mock-offline` (gateway-fallback) path. The drop from 68 → 66 reflects the env-var finding compounding rather than replacing prior gaps — fix that one and the score jumps materially.
