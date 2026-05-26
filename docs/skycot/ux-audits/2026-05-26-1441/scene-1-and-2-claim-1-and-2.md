# Scenes 1 + 2 — Claims 1 + 2 (Teammate-not-tool, Delegation-is-the-unit)

**Date:** 2026-05-26 14:41 AEST
**Scope:** `/workers` deck + `/workers/{id}` tabs (all tabs) + Schedules / Duties tab + "Run now" affordance + audit-row landing.
**Tester:** e2e-ux-journey-test-agent (preview MCP partial, then code-read + curl since the preview Chromium hung on a navigation race between the dev server on :3000 and the preview's own :62927 instance).
**Lens:** function-vs-dressing per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — POC/investor demo, not public release.
**Prior baselines:** `docs/skycot/ux-audits/2026-05-26-0820/scene-1-and-2-claim-1-and-2.md` (15 findings, 52/100) and `docs/skycot/ux-audits/2026-05-26-1028/scene-1-and-2-claim-1-and-2.md` (14 findings).
**Servers used:** gateway `http://127.0.0.1:7400` (schema_version 83, uptime ~14m), Next dev on :3000, preview-MCP instance on :62927 (both serve the same code).

---

## TL;DR

**4 P0, 5 P1, 3 P2, 1 P3 — 13 findings total.** State vs the 10:28 audit: **all five headline blockers still present**; the seed gap is the load-bearing one. The live gateway returns **one worker named "Maya"** (a marketing-intern role created via the hire endpoint, not the demo seed). The five-worker deck the runbook narrates (Mariah / Peta / Priya / Jamal / Nori) only exists in the SPA mock fallback, and even that mock is mis-shaped: `SAMPLE_PERSONA_IDS` lists Mariah / Priya / Jamal / Nori / Sam / Rio — Peta is defined as a persona at `web/lib/sample-personas.ts:124` but absent from the ID list, so even the mock-bypass path renders the wrong roster. Scene 2 has two independent blockers: there is no Schedules / Duties tab on the worker tab strip (tabs are Overview / Activity / Memory / Skills / Trust / Escalation / Settings), and although a `DutiesCard` does mount inside Overview, it has no Run-now affordance — its callbacks announce "Edit schedule ships in E4-h" / "Add responsibility dialog ships in E4-h" (stubs). The audit feed contains only `audit.detail.viewed` / `audit.replay` rows where `worker.name = "unknown"` — there is no real worker turn to drill into, so Claim 2's "delegation is the unit" talk-track has no provenance to camera-on.

**The one-line summary for the operator:** the function-side gap is the demo-seed CLI never landed against this gateway tenant. Until `argo demo seed --tenant <demo>` runs and produces a real worker turn (with `delegated_by` / `handoff_id` populated in the audit row's provenance JSON), Scenes 1 and 2 are unrecordable as scripted.

---

## Headline blockers (top 4 P0s)

| # | Severity | Finding | Scene impact |
|---|---|---|---|
| F1 | P0 | Gateway returns **1 worker "Maya"**; SAMPLE_PERSONA_IDS includes Sam/Rio instead of Peta. | Scene 1 deck has 1 card, not 5. Scene 2 has nobody named Mariah. |
| F2 | P0 | **No Schedules / Duties tab.** DutiesCard renders inside Overview, no Run-now button anywhere on the worker profile. | Scene 2 step 1 (click Schedules) impossible; step 2 (Run-now) impossible. |
| F3 | P0 | Gateway `/api/v1/workers/<maya-id>/overview` returns `duties: [], recent_activity: []`, all `permissions.can_edit_*: false`. | DutiesCard would render its empty state on camera; can't toggle/edit even if a button existed. |
| F4 | P0 | Audit feed has **no real worker actions** — only `audit.detail.viewed` rows with `worker = {id:"", name:"unknown"}`. Drill-in returns 8 decision-chain stages but every body is `""` and every detail field is `null`. | Claim 2 ("delegation is the unit") has no provenance for the camera. Audit-row landing for Scene 2 finale lands on empty-template theatre. |

---

## All findings

### F1 [P0] — Gateway returns 1 worker "Maya"; SAMPLE_PERSONA_IDS still ships Sam/Rio instead of Peta

`curl -s http://127.0.0.1:7400/api/v1/team`:
```json
{"total_count":1,"workers":[{"id":"019e5dbb-c03d-79a0-a9a4-ee60bb3c8218","name":"Maya","role":{"tagline":"Marketing intern that drafts and asks before publishing.","vertical":"marketing-intern"},"status":"idle","headline":{"bullets":["No activity yet."],"tomorrow":null}}]}
```
Same response via the SPA proxy at `:62927/api/v1/team` — the SPA reads gateway truth in real time.

Mock-fallback escape hatch *also* fails: `web/lib/sample-personas.ts:181-188` ships `SAMPLE_PERSONA_IDS = [MARIAH, PRIYA, JAMAL, NORI, SAM, RIO]`. **Peta is defined** as a persona at line 124 but excluded from the ID list. The runbook + landing-page narration says "Mariah, Peta, Priya, Jamal, or Nori" — neither the gateway nor the mock fixture deliver that set. (The canonical demo-seed crate `crates/argo-demo-seed/src/personas.rs:39-76` IS correct — it has Mariah/Peta/Priya/Jamal/Nori.)

Reproduction:
- `curl -s http://127.0.0.1:7400/api/v1/team` → total_count=1
- `rg 'name: "Peta"' web/lib/sample-personas.ts` → line 124 (defined)
- `rg 'SAMPLE_PERSONA_IDS' web/lib/sample-personas.ts -A 8` → no `PETA.id`
- Open http://localhost:3000/workers → `WorkersPage` calls `useTeam()`, sees 1 worker, `router.replace('/team')`, lands on the team page with 1 "MA" card.

Suggested fix: run `argo demo seed --tenant <demo>` to seed the canonical 5 personas into the gateway. Until that's wired into the dev-server boot, fix the mock-bypass fallback: drop SAM and RIO from `SAMPLE_PERSONA_IDS` and insert PETA in the canonical slot.

**Files:** `web/lib/sample-personas.ts:181-188`, `crates/argo-demo-seed/src/personas.rs`.

---

### F2 [P0] — No "Schedules" / "Duties" tab on worker profile; no Run-now affordance anywhere

Tab strip per `web/components/skycot/WorkerTabs.tsx:19-27`:
```
overview · activity · memory · skills · trust · escalation · settings
```
A `DutiesCard` titled "Responsibilities" renders inside the Overview tab (mounted by `web/components/skycot/worker/WorkerOverviewContent.tsx:146`). Per-row affordances in `web/components/skycot/worker/DutiesCard.tsx`:
- enable/disable toggle (`duty-toggle-<id>`)
- click-to-edit (fires `onEditDuty`)
- "+ Add responsibility" footer button

**No "Run now" / Play button.** Grep across `web/app/(app)/workers/` for `"Run now"`: zero matches. The only "Run now" in the codebase is on `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:290`. So the Scene-2 talk-track ("click Run now on Mariah's daily standup duty → audit row lands in 5s") is unimplementable on the worker profile; the operator would have to navigate /workflows/<vertical>/<name> and Run-now there — different scene.

Worse, the existing DutiesCard callbacks in `WorkerOverviewContent.tsx:70-77` are explicit stubs:
```ts
const onAddDuty = useCallback(() => setAnnouncement("Add responsibility dialog ships in E4-h"), []);
const onEditDuty = useCallback(() => setAnnouncement("Edit schedule ships in E4-h"), []);
```
And the toggle just `console.warn`s `"[E4-b] duty '<id>' toggled to <bool> — mutation lands with E4-h"`. So even on the surface that exists, every interaction is a no-op.

Reproduction:
- `rg 'Run now' web/app/\(app\)/workers/ web/components/skycot/worker/` → no matches
- `cat web/components/skycot/WorkerTabs.tsx` → tab list has no schedules/duties
- Open `/workers/<id>/responsibilities` → 404 (only `/workers/<id>/responsibilities/new` exists)

Suggested fix: either (a) author a `schedules` tab with a real Run-now button wired to a workflow.invoke endpoint, or (b) rewrite the runbook Scene 2 to navigate /workflows/<vertical>/<workflow> and Run-now from there. (b) is much cheaper for the v1.0 MP4.

**Files:** `web/components/skycot/WorkerTabs.tsx:19-39`, `web/components/skycot/worker/DutiesCard.tsx`, `web/components/skycot/worker/WorkerOverviewContent.tsx:70-88`.

---

### F3 [P0] — Maya has zero duties + locked-down permissions; DutiesCard renders the empty state

`curl http://127.0.0.1:7400/api/v1/workers/019e5dbb-c03d-79a0-a9a4-ee60bb3c8218/overview`:
```json
{
  "charter": {"text":"Drafts blog posts based on briefs and asks for approval before publishing.","version":1},
  "duties": [],
  "kpis": [{"key":"actions_taken_today","value":"0"},{"key":"pending_approvals","value":"0"},...],
  "permissions": {"can_edit_charter":false,"can_edit_duties":false,"can_pause_resume":false,...},
  "recent_activity": []
}
```
Every list is empty and every permission is `false`. So even if the runbook navigated to the (nonexistent) Schedules tab, Maya wouldn't have anything to schedule and the operator couldn't add one inline. The Overview shows three sparkline rows all `[0,0,0,0,0,0,0]`.

This is "function-side" by the operator's rule: the camera lands on real-but-empty data, not a fabricated card. So it's not P0 because it's THEATRE — it's P0 because the demo journey can't reach a populated Overview without seeding.

Reproduction: as above.

Suggested fix: same as F1 — seed the gateway with duties + recent activity per persona. The demo-seed crate already authored the personas; it needs to also insert ≥1 duty per worker and ≥1 audit row per worker turn so the Overview lights up.

**Files:** `crates/argo-demo-seed/src/personas.rs`, `crates/argo-demo-seed/src/plan.rs`.

---

### F4 [P0] — No worker actions in the audit feed; drill-in returns an 8-stage decision chain with empty bodies

`curl -s 'http://127.0.0.1:7400/api/v1/audit?limit=10'` returns ten rows. Every single one is:
- `action_type` ∈ {`audit.detail.viewed`, `audit.replay`}
- `worker = {"id":"", "name":"unknown"}`
- `channel = "web"`, `verb = "http.request"`

These are the SPA itself navigating the audit page, not worker turns. Drilling into the first row (`/api/v1/audit/019e61d28dc77db0a38693ff74d99fe7`) returns a structurally complete decision chain:
```json
{"decision_chain":[
  {"n":1,"title":"Memory loaded","body":"","details":{"snapshot_id":null}},
  {"n":2,"title":"Skill chosen","body":"","details":{"skill_id":null,"skill_trust":null,"skill_version":null}},
  ...
  {"n":8,"title":"Outcome","body":"","details":{"downstream":null,"status":null}}
]}
```
Eight stages, every `body: ""`, every `details.*: null`. `provenance.signature = ""`, `provenance.model.id = ""`, `provenance.skill.id = ""`. The drill-in modal would render eight rows of stage names with no content underneath — looks broken on camera, but isn't "fabricated" (the structure is real, the substance is just missing because no real worker turn has happened).

Claim 2 ("delegation is the unit") asks the demo to show a delegation chain — `delegated_by`, `handoff_id`, sub-task fanout. None of those fields are populated on the only audit rows we have. There is nothing to drill into.

Reproduction:
- `curl http://127.0.0.1:7400/api/v1/audit?limit=10 | jq '.items[] | {action_type, worker}'` → 10× `{action_type: "audit.detail.viewed", worker: {id:"", name:"unknown"}}`
- `curl http://127.0.0.1:7400/api/v1/audit/<id>` → 8 empty stages

Suggested fix: see F1+F3 — seed needs real worker turns. Each turn needs to anchor the hash chain, populate `skill.id`, model + handoff metadata, decision-chain bodies. The drill-in UI is in place; it's data-starved.

**Files:** `crates/argo-demo-seed/src/lib.rs` (Pass A/Pass B paths); `web/components/skycot/audit/DecisionChainCard.tsx` (renderer).

---

### F5 [P1] — `/workers` and `/team` are both in the nav-bar and resolve to the same page

`web/components/skycot/nav-items.tsx:24-25`:
```
{ href: "/team", label: "Team", icon: <Users /> },
{ href: "/workers", label: "Workers", icon: <Workflow /> },
```
Both nav entries are visible. `/workers` (`web/app/(app)/workers/page.tsx:19-46`) is a thin redirect: `useEffect → router.replace('/team')` if `workers.length > 0`, otherwise renders `<HireStarterPrompts />`. On the live gateway with 1 worker (Maya), clicking either nav row lands on `/team`. To the buyer this looks like two surfaces that disagree (Team / Workers) but actually one redirects into the other. The runbook script says "click Workers" (matches Scene-1 narration) — fine functionally, but the navbar should not have a duplicate.

Reproduction: open the SPA, mouse to the left nav, observe "Team" and "Workers" stacked. Click Workers, watch the URL flip to `/team` and the active highlight move.

Suggested fix: hide `/workers` from `NAV_ITEMS` (it stays accessible by URL for empty-tenant first-touch). Or invert: make `/workers` the canonical route, route `/team` to it.

**Files:** `web/components/skycot/nav-items.tsx:22-30`, `web/app/(app)/workers/page.tsx:19-46`.

---

### F6 [P1] — `PERSONALITIES` map keyed on worker.name has no entry for Peta, Nori, Maya — they all fall to the generic "Reliable / Curious / Helpful" default

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96`: `PERSONALITIES` only has Mariah, Priya, Jamal. Nori is in the SAMPLE_PERSONA_IDS list but not in this map. Peta isn't in either. So on the Worker-detail page (the C2-personality card the design-lock memory `project_worker_detail_design_locked` calls for), only Mariah / Priya / Jamal show distinctive traits, great-at, escalate-when, and signature behaviors. Everyone else gets DEFAULT_PERSONALITY at line 98:
```ts
traits: ["Reliable", "Curious", "Helpful"]
greatAt: ["Working through the responsibilities you set", "Asking when something is ambiguous", "Surfacing patterns across recent runs"]
escalateWhen: [...]
signature: ["Always confirms before a public or irreversible action"]
```
This is generic boilerplate — visible on camera if the runbook clicks into anyone other than Mariah/Priya/Jamal.

The same gap exists in `TeammateCard.tsx:51-67`: `TRAITS_BY_NAME` has Mariah/Priya/Jamal/Nori/Sam/Rio — **no Peta**. The deck-card trait chip row would be empty for Peta. `LEVEL_BY_NAME` has Mariah=5, Priya=9, Jamal=12, Nori=15, Sam=7, Rio=4 — Peta falls to `hashLevel()` (some random `(name.length * 31) % 18 + 2` value), inconsistent with everyone else's hand-tuned level.

Reproduction:
- `rg 'PERSONALITIES = ' web/components/skycot/worker/WorkerPersonalityCard.tsx -A 80` → 3 entries (Mariah/Priya/Jamal)
- `rg 'TRAITS_BY_NAME' web/components/skycot/TeammateCard.tsx -A 10` → 6 entries, no Peta
- `rg 'LEVEL_BY_NAME' web/components/skycot/TeammateCard.tsx -A 10` → 6 entries, no Peta

Suggested fix: add Peta to `PERSONALITIES`, `TRAITS_BY_NAME`, and `LEVEL_BY_NAME`; add Nori to `PERSONALITIES`. Treat these three maps as ONE source of truth (probably co-locate with `sample-personas.ts`).

**Files:** `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-111`, `web/components/skycot/TeammateCard.tsx:51-67`.

---

### F7 [P1] — `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` (set in launch.json) is not the var the middleware reads (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`)

`.claude/launch.json` line 9: `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`.
`web/middleware.ts:39`: reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`.
`web/lib/api/team.ts:50`: reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`.

The bypass works only because `NEXT_PUBLIC_ARGO_DEMO_MODE=1` is also set, which the middleware honors. But the team-API's `isAuthBypassed()` returns false, so the SPA always hits the gateway instead of using the SAMPLE_TEAM fallback that includes the 5 named personas. **This is the load-bearing bug that hides the 5-card deck even when SAMPLE_TEAM would be correct.**

Reproduction:
- `rg 'NEXT_PUBLIC.*_AUTH_BYPASS' web/` → all references use `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS`
- `cat .claude/launch.json | jq '.configurations[0]'` → sets `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` (no ARGO prefix)
- `curl :62927/api/v1/team` → real gateway response, not SAMPLE_TEAM

Suggested fix: rename the env var in `.claude/launch.json` to match (`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`). One-line change. After this, the SPA renders the 5-worker SAMPLE_TEAM by default, which would also patch around F1 for the recorded demo (still need to fix Peta-missing-from-ID-list).

**Files:** `.claude/launch.json:9`.

---

### F8 [P1] — A `/team` empty-banner reads "Demo mode — showing sample data. Sign in to see your own workspace" and is on by default

Visible at the top of the rendered /team page during the preview-MCP screenshot, with a close (X) button. This is the demo-mode disclosure banner (set by `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`). The copy explicitly says "showing sample data" — when the camera rolls, this banner undercuts the "real teammates" register the runbook is trying to establish. The narrator says "here are your five teammates" while the banner says "actually it's sample data".

Reproduction: load /team in the preview, screenshot the top of the page.

Suggested fix: either dismiss before record (operator workflow note), or auto-dismiss in demo-mode after 2s, or rewrite the banner to a more discreet badge (e.g., a tiny "demo" pill in the page corner instead of a full-width banner).

**Files:** find `<DemoBanner>` component (referenced in `web/lib/skycot/demo-mode.ts:13-14`).

---

### F9 [P2] — Empty `headline.bullets = ["No activity yet."]` on Maya bleeds into the team-card "today / tomorrow" copy

`/api/v1/team` returns `headline: {bullets: ["No activity yet."], tomorrow: null}` for Maya. The TeammateCard's today/tomorrow block renders that string under the level banner. Even after demo-seeding, this empty-state copy is the per-worker fallback when there's no real activity for the day — a fresh-hire worker would have the same. It's acceptable copy ("No activity yet." is honest), but it makes the card feel like the worker is broken or hasn't been wired.

Reproduction: `curl :7400/api/v1/team | jq '.workers[0].headline'`.

Suggested fix: when `bullets` is the single placeholder string "No activity yet.", render a warmer empty state ("just hired, getting set up...") — same component, different copy branch.

**Files:** `crates/argo-acp/src/handlers/team.rs` (or wherever the headline is composed); `web/components/skycot/TeammateCard.tsx`.

---

### F10 [P2] — Worker tabs do not visually expose Activity/Memory/Trust/Escalation/Settings cleanly on /workers/{id} default

The default route is `/workers/<id>` (no sub-segment) — `WorkerLayoutShell` renders Overview as the default. The 7 sub-tab links (`overview · activity · memory · skills · trust · escalation · settings`) all share the same minimal h-10 chip styling. On camera at 1080p they'll likely show as a single horizontal strip of small text; the "Skills" tab is easily confused with the global "/skills" nav. There's no visual hint that these are per-worker tabs vs global nav.

Reproduction: navigate to `/workers/<maya-id>` and observe the tab strip below the WorkerHero.

Suggested fix: add a 1-line scoping label above the strip ("This worker's profile · 7 sections") or visually nest the tabs inside the hero card.

**Files:** `web/components/skycot/WorkerTabs.tsx:75-101`.

---

### F11 [P2] — `Worker.permissions = {can_edit_*: false, can_pause_resume: false, ...}` likely gates the entire interaction surface on the demo recording

Maya's overview returns `permissions.can_edit_charter: false`, `can_edit_duties: false`, `can_pause_resume: false`, `can_archive: false`, `can_delete: false`, `can_view_audit: false`, `can_view_settings: false`. So even when content seeds, every interactive control on the Overview will be disabled. The FreezeButton (per `web/app/(app)/workers/[workerId]/FreezeButton.tsx`) is gated by `canFreeze` from the layout server-fetch — likely also false in this state. Camera rolls, narrator says "watch us pause her", button is greyed.

This is a permissions-model finding — the demo-mode tenant should grant the recording user the full delegator role (or the gateway should grant operator-by-default permissions in demo mode).

Reproduction: `curl :7400/api/v1/workers/<id>/overview | jq '.permissions'`.

Suggested fix: when `ARGO_DEMO_MODE=1` is set on gateway boot, override per-worker permissions to all-true for the demo seed tenant. Add an audit row noting the override so it's not hidden.

**Files:** `crates/argo-acp/src/handlers/workers.rs` (look for `compute_permissions`).

---

### F12 [P3] — Preview-MCP nav-race: window.location.replace bounces between port :3000 and :62927 because two Next dev instances serve the same code

I tried to drive the preview MCP browser through the journey, but my early `window.location.replace('/workers')` calls bounced between `http://localhost:3000` (the long-lived dev server) and the preview's own `http://127.0.0.1:62927`. Eventually the preview Chromium hung on a snapshot call. Not a SKYCOT bug — it's a tooling collision in my test harness. Worth noting because future runs of this audit on the same machine will hit the same race.

Workaround: stop the long-lived dev server before invoking preview-MCP, or use absolute URLs on `:62927` and never relative paths.

---

### F13 [P1] — Workers nav-bar entry routes to a redirect; on first paint there's a `<Loader2>` spinner shown for ~200-800ms before the redirect to /team fires

`web/app/(app)/workers/page.tsx:25-35`: the page first paints a loading spinner (loop testid `workers-loading`), THEN the `useEffect` checks team data, THEN calls `router.replace('/team')`. On the recorded demo, clicking the "Workers" nav item will show a spinner for a beat before the URL changes — looks janky vs a direct route. Easy to mistake on camera for "slow page".

Reproduction: hard-refresh `/workers` with a populated team — observe the brief spinner before the URL flips.

Suggested fix: server-side redirect via `redirect('/team')` from a server component, or middleware rewrite. Removes the spinner.

**Files:** `web/app/(app)/workers/page.tsx:19-35`.

---

## Resolved since prior audit (2026-05-26-1028)

Cross-checked against the prior baseline's 14 findings. **None of the P0s have moved.** F1/F2/F3/F4 in this report are the same five P0s the prior baseline called out (consolidated to four — F5 from the prior report, the "no delegation chain visible in audit drill-in," is folded into F4 of this report). The F5–F11 P1/P2/P3 findings overlap heavily with the prior baseline's F6–F14. No regressions were introduced this tick.

The prior baseline's new-this-run finding ("`/api/v1/workers/<id>` fails with `net::ERR_FAILED` from cross-origin browsers") was not reproducible from curl on either port (200 from both). If the operator sees that on the real browser, it's worth a separate browser-only repro pass.

---

## Orientation score

**Scenes 1 + 2 ship-readiness: 38 / 100.**

The component scaffolding is good — locked C2-holographic card design is implemented (`TeammateCard`), locked C2-personality detail card is implemented (`WorkerPersonalityCard`), tab strip + Overview composition is in place. **The whole gap is data + a missing Schedules tab + 1 missing button.** Run the demo-seed CLI against the live gateway (it has the right personas baked in), fix the four lookup tables to include Peta (sample-personas.ts ID list, WorkerPersonalityCard map, TeammateCard trait map, TeammateCard level map), and either ship a Schedules tab with Run-now OR rewrite Scene 2 to launch from /workflows — and the scene becomes recordable. The 6 hrs of work here is data + ~30 lines of UI + a runbook edit, not a deep architectural fix.

Until that happens, the v1.0 MP4 cannot record Scenes 1 + 2 as the runbook scripts them.
