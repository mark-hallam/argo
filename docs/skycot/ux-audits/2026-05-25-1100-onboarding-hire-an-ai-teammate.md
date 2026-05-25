# SKYCOT e2e UX audit — Hire an AI teammate journey

**Audit date**: 2026-05-25 11:00 AEST (Melbourne)
**Auditor**: Claude (e2e UX journey agent)
**Tools**: Claude Preview MCP (Chromium) on SPA `:3000` + gateway `:7400`
**Scope**: full `/landing → /auth/signup → /onboarding/* → /workers/{id}` flow — the operator's primary "hire an AI teammate as you would a human" investor narrative
**Setup**: gateway live (schema v81, uptime ~5h), env `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_ARGO_DEMO_MODE=1`, tenant has 1 worker (Maya, marketing-intern, ULID `019e5dbb-c03d-79a0-a9a4-ee60bb3c8218`, 0 actions, status=idle, charter from gateway)
**Reference**: prior audit at `docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` (operator-rejected as 87-finding noise); this audit aims for 10–25 high-leverage findings.

---

## TL;DR

**16 findings**: 4 P0 (demo + investor-narrative blockers), 6 P1 (visible but workable), 4 P2 (polish), 2 P3 (nits).

**Biggest blockers**:
1. Worker detail `/workers/<id>/overview` crashes for any newly-hired worker — gateway returns `charter` as an object, SPA renders it as a string → React throws. The narrative's destination page is broken (F5, P0).
2. `/audit/action/<id>/replay` fabricates a "Policy regression detected" diff for any real action ID. If Scene 6's camera drills into an audit row, it lands on theatre (F12, P0). Carries from prior audit F67.
3. The `/team` hero banner hard-codes `workers.length × 187 actions/week, × 12 hours saved` — Maya has 0 actions yet the banner claims 187 / 12. The first frame after hire lies about activity (F4, P0).
4. Marketing `/landing` carries "2,847 teams this month" + 12 fictitious customer logos + 3 fictitious testimonials — dressing-side per the function-vs-dressing rule, but the v1 demo's first frame is camera-visible (F2, P0 only if the demo opens on `/landing`; otherwise P1).

**Function/dressing call**: most theatre is dressing-side (marketing copy, fixture personas, hardcoded landing stats). The function-side blockers are F5 (worker detail crash), F12 (replay fabrication on real IDs), F4 (team hero KPIs lie about real activity), and F8 (charter is an object the SPA can't render).

**Hire flow exists, but the "hire-an-ai-teammate" metaphor leaks**: the flow walks like onboarding a human (welcome → workspace → tools → role → first task → meet them) — the language and pacing are right. But the function fails at the destination (F5), the new worker doesn't get a real personality (F7), and the team page lies about their first-week impact (F4). Mariah/Priya/Jamal et al. work because they're hardcoded; any worker the user actually hires becomes a stranger.

---

## Headline blockers — investor-narrative load-bearing

| # | Sev | Finding | Narrative impact |
|---|---|---|---|
| **F5** | **P0** | `/workers/<id>/overview` crashes on real worker — gateway returns `charter` as an object, SPA expects string. React error visible. | The investor-narrative destination page (the "meet your teammate" reveal) is broken for any real hire. Camera lands on Maya → blank/error. |
| **F12** | **P0** | `/audit/action/<real-id>/replay` shows fabricated "Policy regression detected" mock diff. Re-uses the action ID as the diff content. | Camera drills into real audit row, finds theatre. Carries forward from prior audit F67 — still unfixed. |
| **F4** | **P0** | `/team` hero hardcodes "0 working right now · 187 actions this week · saving you ~12 hours." Maya has 0 actions; the hero claims 187/12. | Defeats the "audit-first / no theatre" thesis on the first frame after hire. |
| **F8** | **P0** | Gateway `/api/v1/workers/<id>/overview` returns `charter: {text, version, last_edited_at, last_edited_by}` — SPA's `WhatSheDoesCard` types it as `string`. SSR + client both throw. | Same root cause as F5; calling it out separately because the fix is contract-side (gateway returns object, SPA expects string — pick one). |
| **F2** | **P0/P1** | `/landing` shows "2,847 teams this month" pill + 12 fictitious customer logos + 3 fictitious testimonials (Daniela Reyes / Marcus Lin / Aisha Patel). | P0 if v1 demo records `/landing`; P1 otherwise. Function-vs-dressing rule says dressing-OK, but multiple of the 12 logos (Acmeco Retail / Polaris Marketing / Zenith Support / etc.) don't even exist in `/customers` — internal inconsistency. |

---

## All findings, in order

### Marketing / `/landing`

#### F1 [P2] — Landing hero stat "2,847 teams this month" reads as a real number

URL: `/landing`. File: `web/app/landing/page.tsx:177`.

Inline in the trust strip alongside "NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · ISO 27001 · GDPR/HIPAA". Decorative dressing-side label per the function-vs-dressing rule — reads fine as marketing badge, would read as a lie if the camera framed it next to a real product metric.

**Reproduction**: open `http://localhost:3000/landing`, scroll trust strip under hero CTA. **Suggested fix**: either truth-up the number (operator's actual cohort count) or remove the pill — the four security badges already carry the trust register.

#### F2 [P1] — 12 fictitious customer logos + 3 fictitious testimonials on `/landing`

URL: `/landing`. File: `web/app/landing/page.tsx:63-99`.

"Already shipping with SKYCOT" → 12 made-up customer names (Northwind Commerce, Acmeco Retail, Polaris Marketing, Zenith Support, Harbour Legal, Meridian Logistics, Atlas Manufacturing, Lumen Education, etc.). Testimonials by "Daniela Reyes (Head of CX, Northwind Commerce)", "Marcus Lin (CCO, Beacon Health)", "Aisha Patel (Ops Director, Polaris Marketing)". Per `feedback_no_theater_in_demo` these are dressing-side and acceptable if the demo doesn't claim "real customers". HOWEVER 7 of the 12 logo cards reference companies that don't appear on `/customers` (which has 5 case studies, all 5 LOGOS that appear there are wired to story pages). The 7 orphans break the dressing-side contract: a viewer hovering them sees "Acmeco Retail" with no story.

**Reproduction**: open `/landing`, scroll to "Already shipping with SKYCOT" strip. **Suggested fix**: trim LOGOS to the 5 with backing case studies; mark the others "case study coming" or drop them.

#### F3 [P2] — `/landing` hero illustration uses Mariah/Priya/Jamal/Nori not Maya

URL: `/landing`. File: `web/app/landing/page.tsx:182-196`.

The right-hand "art bubbles" reference Mariah/Priya/Jamal/Nori as illustrative personas. Operator memory `project_pm_cohorts_peta_patrick` introduces Peta + Patrick as the canonical first PM workers, and the only worker on the demo tenant is Maya. If demo MP4 cuts from `/landing` → `/team` Maya appears in isolation while the marketing page hyped Mariah et al. — visual mismatch. Acceptable as dressing-side per the rule, but inconsistent.

### Auth / `/auth/signup`

#### F4 [P0] — `/team` hero hardcodes "187 actions/wk, 12 hours saved" per worker

URL: `/team`. File: `web/app/(app)/team/page.tsx:147-148`.

```
const weeklyActions = workers.length > 0 ? workers.length * 187 : 0;
const hoursSaved = workers.length > 0 ? workers.length * 12 : 0;
```

Maya has 0 actions per gateway (`/api/v1/team` returns `headline.bullets: ["No activity yet."]`, `last_active_at: null`, `status: "idle"`), but the team hero claims "0 working right now · 187 actions this week · saving you ~12 hours." The first frame after a brand-new hire LIES about their week.

**Reproduction**: hire any worker (or visit current Maya), open `/team`, read the hero subtitle. Cross-check `curl http://127.0.0.1:7400/api/v1/team`. **Suggested fix**: derive these from the gateway's audit count; if no activity, show "Just hired — your first teammate hasn't acted yet" instead of fabricated stats.

#### F5 [P0] — `/workers/<id>/overview` crashes for the only real worker on the tenant

URL: `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`. File: `web/components/skycot/worker/WhatSheDoesCard.tsx:156` (`{charter}`) + `web/components/skycot/worker/WorkerOverviewContent.tsx:139` (`charter={overview.charter}`).

React error: "Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version})". The header (Maya breadcrumb + idle + tabs) renders, but the body content area shows the dev "500" overlay. ALSO secondary error: `Attempted to call isWorkerTabId() from the server but isWorkerTabId is on the client` — the `[tab]/page.tsx` server route calls a client-marked helper.

**Reproduction**:
- gateway live, visit `/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`
- open browser devtools — two errors above
- the body shows a "500" / "!" overlay; bottom-left badge "1 Issue".

**Suggested fix**: either (a) gateway returns `charter: string` (the `.text` field unwrapped) — preferred; or (b) SPA reads `overview.charter.text` and threads `last_edited_*` to the metadata strip. Pick one — the contract drift is the bug.

#### F6 [P2] — `/auth/signup` desktop hero is right-of-center (off-screen left at 1600px)

URL: `/auth/signup`. File: `web/components/auth/WizardEditorialShell.tsx` (the editorial-center grid).

At native preview width (~1600×900) the "Hire your first AI teammate" hero sits in the right half of the page; the left half is blacked-out (dark mode background). Looks like the editorial-center column is positioned with absolute coordinates that assume a different viewport. The mobile (375) layout reflows correctly. NOT broken — content is reachable, just visually unbalanced.

**Reproduction**: open `/auth/signup` at 1600×900. **Suggested fix**: confirm `wiz-stage` is `max-width: 720px; margin: 0 auto;` (or equivalent) on both auth + onboarding.

### Onboarding wizard — F1 editorial-center

#### F7 [P1] — "SOC 2-ready from day one" copy on `/onboarding/welcome` overclaims

URL: `/onboarding/welcome`. File: `web/app/(setup)/onboarding/welcome/page.tsx:40`.

Pillar card body: "Every action signed and replayable — SOC 2-ready from day one." Operator memory `feedback_no_theater_in_demo` flags compliance-overclaim language. SOC 2 readiness is a multi-month audit, not "day one". Carries from prior audit F16. **Suggested fix**: "SOC 2-ready audit trail" (matches the landing trust pill) — drop "from day one".

#### F8 [P0] — Gateway returns `charter` as an object; SPA expects string

See F5 above; calling out the contract separately so it can be assigned to the right owner. Gateway `/api/v1/workers/<id>/overview` shape:

```json
{
  "charter": {
    "text": "Drafts blog posts based on briefs and asks for approval before publishing.",
    "version": 1,
    "last_edited_at": "...",
    "last_edited_by": "system:hire-endpoint"
  },
  ...
}
```

SPA `WhatSheDoesCard.charter: string`. **Suggested fix**: align the contract. Gateway file: `crates/argo-cli/src/handlers/gateway/workers_overview*` (find the relevant handler) OR SPA file: `web/lib/hooks/use-worker-overview.ts` + `web/components/skycot/worker/WhatSheDoesCard.tsx`.

#### F9 [P2] — `/onboarding/pick-vertical` only ships 5 templates, no Marketplace seller / Recruiter

URL: `/onboarding/pick-vertical`. File: `web/app/(setup)/onboarding/pick-vertical/page.tsx:22-58`.

Verticals available: GTM Engineer / Marketing Intern / Sales SDR / Customer Support / Accounts Receivable. Missing from the operator's narrative: Marketplace seller (Mariah), Recruiter (Priya). The personas the landing page + customer stories rely on aren't installable from the onboarding wizard. A user hiring "Priya the recruiter" today picks "Sales SDR" or "Customer Support" — neither maps. **Suggested fix**: either align verticals to landing personas, or rename verticals so the mapping is one-to-one.

#### F10 [P1] — Newly-hired worker gets the DEFAULT_PERSONALITY fallback, not a real persona

URL: `/workers/<id>/overview`. File: `web/components/skycot/worker/WorkerPersonalityCard.tsx:99-110, 117`.

```
const personality = (worker && PERSONALITIES[worker.name]) ?? DEFAULT_PERSONALITY;
```

Personalities are keyed by name and only `Mariah/Priya/Jamal` are populated. Maya (and every freshly-hired worker not named Mariah/Priya/Jamal) falls back to: traits "Reliable, Curious, Helpful"; greatAt "Working through the responsibilities you set / Asking when something is ambiguous / Surfacing patterns…". This breaks the investor-narrative claim "your AI teammate is a person, not a tool" — every hire IS the same person.

**Suggested fix**: derive personality from charter + vertical pack manifest (the C2 personality memory says "the character/charter system on the gateway will surface these in a follow-up brief" — that follow-up needs to land before the demo can show a hired worker's profile).

#### F11 [P1] — `/team` card LEVEL is a hash of the worker's name, not real

URL: `/team`. File: `web/components/skycot/TeammateCard.tsx:59-71`.

```
const LEVEL_BY_NAME: Record<string, number> = { Mariah: 5, Priya: 9, Jamal: 12, Nori: 15, Sam: 7, Rio: 4 };
function hashLevel(name) { /* hash mod 18 + 2 */ }
```

Maya the brand-new marketing intern hashes to LEVEL 16 — visible front-and-center on the trading card. The level pretends to be earned. Same theatre register as F4. Acceptable as dressing-side if the level pill is decorative, but the card's prominent gradient banner positions LEVEL 16 as a status signal.

**Suggested fix**: derive level from real action count + trust ratchet (the trust grade memory `project_workers_page_design_locked` mentions L0–L5 cohort grades — wire that in), or remove the LEVEL pill until real numbers exist.

### Audit / `/audit`

#### F12 [P0] — `/audit/action/<real-id>/replay` shows fabricated "Policy regression detected" report

URL: `/audit/action/019e5dbaec037fa2819eab3f27f3f41e/replay` (use any real audit row ID from `curl http://127.0.0.1:7400/api/v1/audit?range=today`). File: unknown — needs `rg "Policy regression detected" web/`.

Pop-out page renders "Sandbox — no side effects · Drifted-misaligned" banner + Output diff with text "Reconstructed original output for <id>." for both ORIGINAL and REPLAYED panels (with "Policy tightening would block this output today." highlighted in REPLAYED). Provenance diff says MODEL/PROMPT/SKILL/MEMORY same and POLICY `generic-policy v1.0 → v1.1`. **None of this content is real for the action** — it substitutes the ID into placeholder strings.

Carries forward from prior audit F67 (operator-rejected audit but the finding stands). For the demo's Scene 6 audit-drill-in this is theatre the camera will see.

**Reproduction**: hit `/audit`, click any row → click "Replay" or similar → land on this page. **Suggested fix**: either implement real replay (gateway endpoint), or make the page show "Replay coming in v1.x — view the action source instead" with a button back to the action's source view. Do NOT keep the fabricated diff.

#### F13 [P2] — `/audit` rows show worker `UN / unknown` because audit-side worker_id is empty string

URL: `/audit`. Source: gateway returns `worker: {id: "", name: "unknown"}` for the 3 real hipaa_compliance_check rows.

The audit page correctly renders this (badge "UN", no fake worker name), so this is gateway-side: the hipaa compliance checks aren't being attributed to a worker. For an investor demo this is visible — "your action was performed by unknown" is the wrong vocabulary. Could be a system-action (background compliance scan) that legitimately has no worker, in which case the SPA should render "System" not "unknown".

**Suggested fix**: gateway-side, set worker to `{id: "system", name: "System"}` for compliance-scan actions, or filter system-action rows out of the default audit view.

### Hire flow / `/hire`

#### F14 [P1] — `/hire` and `/onboarding/*` are TWO parallel 5-step wizards with content drift

URLs: `/onboarding/welcome` (first-run) and `/hire` (from `+ Hire` on `/team`). Both wrap `WizardEditorialShell`. Step 1 differs:
- `/onboarding/welcome`: "Build your AI workforce" + 3 trust pillars (Audit-first / Single-binary / Enterprise-grade).
- `/hire`: "Build your AI workforce" + "What do you need help with?" + textarea.

Step 5 of `/onboarding` is "Describe your first worker" with a textarea — same as `/hire` step 1. The first-run user gets the trust pillars pitch; the second-hire user gets straight to the task. This is fine UX-wise but the parallel-abstractions rule from `CLAUDE.md` flags this as a maintenance risk: any tweak to the wizard chrome has to be done in both places. Verify the deduplication is intentional.

**Suggested fix**: either unify (welcome pillars become a `firstRun?: boolean` toggle on the same wizard), or document why they're separate in both files' headers.

#### F15 [P1] — Onboarding `/onboarding/connect-provider` "Skip for now" leaves a mock-provider tenant

URL: `/onboarding/connect-provider`. File: `web/app/(setup)/onboarding/connect-provider/page.tsx`.

Skip path: `setProvider(false, true); router.push("/onboarding/pick-vertical")`. Honest copy: "Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings." Verified the flow proceeds without an Anthropic key.

This is correct demo behavior but the path doesn't surface that the worker is running on mocks anywhere later in the flow. Once you land on `/team` after the hire, there's no banner saying "this workspace uses a mock provider — switch to a real key in Settings". A user who skipped 5 minutes ago hires Maya, watches her stay idle forever, and doesn't know why. **Suggested fix**: add a persistent "Mock provider" pill in the app shell when `providerKeyConfigured=false && providerSkipped=true`.

### Polish / nits

#### F16 [P3] — Sidebar `Workers` nav highlights as "Team" because `/workers` 302s to `/team`

URL: `/workers`. File: `web/app/(app)/workers/page.tsx:25` (`router.replace("/team")` when workers exist).

Clicking the sidebar "Workers" link silently puts you on `/team` and highlights "Team" in the sidebar — confusing for a user navigating to the workers list. Either route both to the same canonical surface (drop the sidebar duplicate), or land the user on `/workers` with a real list view (the file comment says "a dedicated workers list with column-level filters is a separate Track-E brief" — that's still pending).

---

## Resolved since last audit (2026-05-25 morning audit)

Prior audit F37 (`/workers` empty-state, no seed) — partially resolved by F16 here: `/workers` redirects to `/team` which DOES show a worker (Maya, hired via demo-mode signup). The trading-card deck is now seeded with one real worker, not zero.

Prior audit F63 (`/audit` empty "Quiet day") — RESOLVED. Gateway has 3 real hipaa_compliance_check rows visible at `/audit` today.

Prior audit F1 (Approvals dashboard hardcoded KPIs) — NOT RE-VERIFIED in this audit (out of scope).

---

## Orientation score: 62 / 100

**Why not higher**: the four P0s touch the investor narrative's destination page (F5/F8), its first audit moment (F12), and the post-hire team frame (F4). The hire flow itself (signup → 5-step wizard → /team appearance) WORKS — the metaphor of "onboarding a teammate" is honored from the user's POV — but the destination is broken or theatrical. The investor seeing "I hired Maya" and clicking her profile gets a React error overlay; clicking an audit row gets a fabricated regression report. The story works until the camera pans to evidence.

**What would push it to 80+**: fix F5+F8 (worker detail renders), F12 (replay is honest), F4 (team hero reflects real activity). The dressing-side findings (F1/F2/F11) are operator-decision calls, not engineering blockers.

**What would push it to 90+**: real persona generation from vertical pack manifests (F10), real LEVEL computation (F11), unified `/onboarding` ↔ `/hire` wizard (F14), mock-provider banner in app shell (F15). At that point a brand-new user hires Maya, sees Maya's real personality, real charter, real (zero, growing) action count, and a clean audit trail.
