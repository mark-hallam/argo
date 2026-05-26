# Scene 1 + 2 — Claim 1 (Teammate, not tool) + Claim 2 (Delegation is the unit)

**Audit date**: 2026-05-26 21:52 AEST
**Scope**: `/workers`, `/team`, `/workers/{id}/*` (all 7 tabs) for the 5 demo personas — Mariah / Peta / Priya / Jamal / Nori.
**Method**: Claude Preview MCP walk (Chromium) + read-only code inspection. Preview renderer became unstable mid-run from continuous NextAuth `SessionProvider` ClientFetchError loops (see F2); the second half of the walk relied on direct route + component reads against the same fixtures the demo will render.

---

## TL;DR

20 findings: **4 P0 (demo-blocker), 8 P1 (visible — fix before recording), 6 P2 (polish), 2 P3 (nit)**.

The two biggest demo-blockers are **(F1)** the `/workers` route is a stub that redirects to `/team` — so anyone who narrates "click Workers in the sidebar" lands on the Team page with a `MOCK-OFFLINE` debug pill front-and-centre, and **(F4)** Peta + Nori have **no personality data** on either the `/team` card or the `/workers/{id}/overview` page (both are missing from `TRAITS_BY_NAME` and `PERSONALITIES`), so 2 of the 5 demo workers fall through to a generic "Reliable / Curious / Helpful" trait set and a copy-paste "great-at" list — directly contradicting Claim 1 ("teammate, not tool"). **(F3)** there is no "Run now" affordance anywhere on the worker card, the duties card, or the overview tab, so Scene 2's "click Run now → audit row lands" is unscripted. **(F5)** Claim 2 (Delegation) is invisible in the UI even though the runtime supports it: no fixture audit row populates `delegationChain`, so the `DelegationChainCard` never mounts.

The C2 holographic register is intact and looks correct (gradient banner, big level numbers, MA/PE/PR initials, trait chips). KPIs and "today / tomorrow" lines render. The biggest opportunity to make the demo land is: **add personality data for Peta + Nori, rename `/workers` to redirect to `/team` server-side (kill the empty-state stub), populate one `delegationChain` in the sample audit fixture, and add a Run now CTA to the duties card.**

---

## Headline blockers (P0)

1. **F1** — `/workers` is a stub; nav-click lands on `/team` with a `MOCK-OFFLINE` debug pill visible.
2. **F3** — No "Run now" anywhere on worker card / overview / duties card. Scene 2's headline interaction can't be scripted.
3. **F4** — Peta + Nori have no personality block on `/team` card OR `/workers/{id}/overview`. 2 of 5 demo workers feel generic.
4. **F5** — Claim 2 (Delegation is the unit) is invisible in the UI: no sample audit fixture populates `delegationChain`, so `DelegationChainCard` never renders.

---

## Findings

### F1 [P0] — `/workers` is a stub that bounces to `/team`; the canonical roster page shows a `MOCK-OFFLINE` debug pill

The sidebar nav has BOTH a "Team" link (→ `/team`) AND a "Workers" link (→ `/workers`). The Workers page is a client-side stub: if the tenant has any workers, `useEffect` calls `router.replace("/team")`. If the runbook says "click Workers in the sidebar" the camera sees a redirect flash + lands on `/team` with the `MOCK-OFFLINE` data-source pill visible right next to the page title.

The `MOCK-OFFLINE` pill is correct telemetry (we are in `mock-bypass` mode) but for a recorded demo it reads like a dev-mode leak.

- File: `web/app/(app)/workers/page.tsx:19-46`
- File: `web/components/skycot/nav-items.tsx:24-25`
- Repro:
  1. Boot `pnpm --dir web dev` with `NEXT_PUBLIC_ARGO_DEMO_MODE=1`.
  2. Click sidebar "Workers".
  3. Watch the URL flip to `/team` and observe the `MOCK-OFFLINE` pill next to the "Team" heading.
- Fix: either (a) remove the `/workers` sidebar entry entirely and let `/team` be the only nav; or (b) hide the `MOCK-OFFLINE` pill when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`. Probably both.

### F2 [P0] — NextAuth `SessionProvider` spams `ClientFetchError` on every navigation in demo-bypass mode

Continuous browser-console error spew while walking the app:

```
ClientFetchError: There was a problem with the server configuration. Check the server logs for more information.
  at SessionProvider.useEffect [as _getSession]
```

`<SessionProvider>` is mounted globally at `web/components/skycot/AuthProvider.tsx:7` and polls `/api/auth/session`. In demo-bypass mode (which IS the demo runtime) there is no minted `__Host-session` cookie, so the polling fails on a loop. During my session the renderer eventually became unresponsive — preview_eval started timing out after ~60 navigations.

If the operator opens devtools during recording (or screen-shares the console pane), this is the first thing they'll see. Worse: it likely contributes to the SPA being janky on the demo machine.

- File: `web/components/skycot/AuthProvider.tsx:1-9`
- Repro: open `/` with `NEXT_PUBLIC_ARGO_DEMO_MODE=1`, no session cookie. Read `console`.
- Fix: skip `<SessionProvider>` when `NEXT_PUBLIC_ARGO_DEMO_MODE === "1"`, or render a stub SessionContext that never fetches.

### F3 [P0] — No "Run now" affordance on the worker card, duties card, or overview tab

The operator's expectation of Scene 2 ("click Run now → audit row lands within 5s") has no surface that fires it. I searched the entire worker-detail tree:

- `TeammateCardContextMenu` (the `⋮` button on the card) only offers `Pause` / `Resume` per `web/components/skycot/TeammateCardContextMenu.tsx:122-141`.
- `DutiesCard` (the Responsibilities card on `/workers/{id}/overview`) has Toggle, Edit, and Add — no Run-now per `web/components/skycot/worker/DutiesCard.tsx:158-196`.
- Grep `Run now|RunNow|run_now|run_duty` across `web/` returns zero matches.

There is no path from "look at Mariah's schedule" → "trigger one of them" → "see the resulting audit row". The cron `as messages arrive` / `daily 06:30` / `weekdays 09:00` strings are display-only.

- Repro: `/workers/018f67d0-0000-7000-8000-000000000001/overview`; inspect all buttons on the Responsibilities card; observe none of them trigger a workflow run.
- Fix: add a "Run now →" button on each `DutyRow` that calls the gateway's existing workflow-execute route and routes to `/audit/action/<id>` with the freshly-minted action id.

### F4 [P0] — Peta + Nori have NO personality block (great-at / escalate-when / signature) and Peta has NO traits on the team card

Two places hardcode per-worker personality:

1. `web/components/skycot/TeammateCard.tsx:51-58` — `TRAITS_BY_NAME` covers Mariah, Priya, Jamal, Nori, Sam, Rio. **Peta is missing.** Fallback is `[]`, so the team card's TRAITS section silently disappears.
2. `web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` — `PERSONALITIES` covers Mariah, Priya, Jamal. **Peta AND Nori are missing.** Both fall through to `DEFAULT_PERSONALITY` (line 98-111): trait list `["Reliable", "Curious", "Helpful"]`, great-at "Working through the responsibilities you set / Asking when something is ambiguous / Surfacing patterns across recent runs", escalate-when "Confidence in the next action is below 80% / A risky side-effect would happen if they proceed / Multiple plausible interpretations exist".

So 2 of the 5 demo workers feel like the system prompt of a generic agent, not a colleague. This is the single most direct hit on Claim 1.

Additionally, **Nori's traits diverge between the two files** — `sample-personas.ts:119` says `["On-voice", "Considered", "Schedule-respecting", "Reach-tracking"]` while `TeammateCard.tsx:55` says `["Creative", "On-brand", "Concise", "Curious"]`. The Audit feed will use the persona-spine values; the Team card uses TeammateCard's values; the two never agree.

- Repro: navigate to `/workers/018f67d0-0000-7000-8000-000000000007/overview` (Peta) and `/workers/018f67d0-0000-7000-8000-000000000004/overview` (Nori). Observe the same generic content.
- Fix: backfill `PERSONALITIES["Peta"]` + `PERSONALITIES["Nori"]` + `TRAITS_BY_NAME["Peta"]`. Reconcile Nori's traits to the persona-spine canonical set.

### F5 [P0] — Claim 2 (Delegation) is invisible in the UI — no sample audit row populates `delegationChain`

The SPA fully models cross-worker handoff: `DelegationStep` + `DelegationRole` types live in `web/lib/api/audit-detail.ts:24-36`, and `DelegationChainCard` mounts on the audit drill-in page when `chain.length > 1`. But:

- `grep delegationChain web/lib/` returns zero matches outside the type definitions.
- `grep delegation|handoff|originator|delegated_to web/lib/sample-audit*` returns zero hits.

So even though CLAUDE.md says "the runtime is now carrying the active handoff chain through `ToolContext.current_handoff_id`", the demo SPA never sees one. Click into any audit row from the demo seed — the `DelegationChainCard` is short-circuited (`web/app/(app)/audit/action/[actionId]/page.tsx:150`).

If Claim 2 is meant to land in the demo, one of the seeded audit actions must carry a 2-step `delegationChain` (e.g. `Mariah → Jamal` for the buyer-question scenario, or `Peta → Patrick` for the status-escalation).

- File: `web/lib/sample-audit-detail.ts` (no delegation fields)
- File: `web/components/skycot/audit/DelegationChainCard.tsx` (renderer ready but never receives data)
- Fix: add a `delegationChain` block to at least one entry in `SAMPLE_AUDIT_DETAILS` showing one originator + one delegated_to step. Bonus: surface a "↳ delegated from Mariah" badge on the corresponding row in `/audit` listings so the operator knows where to click.

---

### F6 [P1] — Browser tab title is the raw UUID, not the worker name

Visiting `/workers/018f67d0-0000-7000-8000-000000000001/overview` sets `document.title = "018f67d0-0000-7000-8000-000000000001 · Overview"`. Same for every other tab (skills, trust, settings, etc.). If the operator alt-tabs or the camera catches the browser chrome, the URL bar AND tab title both leak the UUID.

- File: `web/app/(app)/workers/[workerId]/overview/layout.tsx:9-11`
- File: `web/app/(app)/workers/[workerId]/[tab]/layout.tsx:14-22` (and 6 other per-tab layouts share the same pattern)
- Fix: `generateMetadata` should `fetchWorker(workerId)` and use `worker.name`; fall back to the UUID only on lookup failure.

### F7 [P1] — Edit Responsibility row fires a "ships in E4-h" stub toast (not a real editor)

Clicking the body of a Responsibility row on the Overview tab calls `onEditDuty(dutyId)` which the parent wires to `setAnnouncement("Edit schedule ships in E4-h")`. There is no editor mounted. The same is true for the toggle — it visually flips but `onToggleDuty` is `console.warn` only.

This means **the operator literally cannot edit the schedules from the UI**. The "Add responsibility" CTA correctly opens `AddResponsibilityDialog` (real), but Add is the only mutation that works.

- File: `web/components/skycot/worker/WorkerOverviewContent.tsx:78-92`
- Fix: either wire Edit + Toggle to real mutations (the gateway endpoint exists at `/api/v1/workers/<id>/duties`) or visually disable the toggle + edit click target in demo mode so the operator doesn't accidentally click them on camera.

### F8 [P1] — Team page header reads "0 working right now" while the same page subtitle says "5 teammates working" and the live status pills say "Waiting on you"

Within one viewport on `/team`:

- Top banner: "0 working right now · 935 actions this week · saving you ~60 hours"
- Stats strip: WORKING **0** / ACTIONS/WK **935** / HOURS SAVED **60**
- Section subtitle: "5 teammates working"
- Card status pills: 4× "Waiting on you" + 1× (Jamal) "Working" or "Idle" depending on the minute

Three different counts in the same view. The "5 teammates working" copy in the section subtitle is the worst offender — it's clearly meant to say "5 teammates" or "5 teammates in your workspace", not "working". On camera the operator will be narrating one number while reading another.

- File: `web/app/(app)/team/page.tsx:255-260` (subtitle string)
- File: source of "0 working right now" — the workforce-stats banner; likely `web/components/skycot/WorkforceStats*` or similar
- Fix: pick one count semantics ("5 teammates" / "1 active" / "4 waiting on you") and use it everywhere.

### F9 [P1] — `MOCK-OFFLINE` data-source pill is rendered next to the "Team" heading in demo mode

Right beside the H1 "Team" the page renders a small pill: `MOCK-OFFLINE`. This is the value of `FetchTeamResult.source` from `web/lib/api/team.ts:36-37`. It's correct dev telemetry (we're in `mock-bypass`) but the demo camera will read it and ask "what does MOCK-OFFLINE mean — is the system broken?"

- File: the source pill renderer (search for `data-source-pill` / `dataSource` in `web/app/(app)/team/page.tsx`)
- Fix: suppress the source pill when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`; show it only in dev-no-demo mode.

### F10 [P1] — Peta gets a hash-generated Level (showed as 18) — not a curated value like the other 4

`LEVEL_BY_NAME` in `TeammateCard.tsx:60-67` covers Mariah(5), Priya(9), Jamal(12), Nori(15), Sam(7), Rio(4). Peta falls through to `hashLevel("Peta") → 18`. So in the demo, the 5 teammates' levels are 5 / 18 / 9 / 12 / N — Peta is the highest level on the team purely by hash coincidence. If the runbook narrates "the system trusts each worker more as they prove themselves" this looks weird (Peta is the newest persona, added 2026-05-19 per memory).

- File: `web/components/skycot/TeammateCard.tsx:60-67`
- Fix: add a deliberate `Peta: 4` (or 6 — the operator picks) so Peta reads as "newest hire, level 4" — fits the M22 brief framing.

### F11 [P1] — `/audit?worker=<id>` link from the Activity tab uses the raw UUID

The "Open in Audit timeline →" link at the bottom of the Activity tab is `/audit?worker=018f67d0-0000-7000-8000-000000000001&range=7d` per `web/app/(app)/workers/[workerId]/activity/page.tsx:69`. If the operator copies this URL into a Slack message or screen-shares the address bar, the long UUID is visible.

- Repro: navigate to `/workers/<id>/activity`, hover "Open in Audit timeline →" link.
- Fix: use a worker-name slug as the audit filter param: `/audit?worker=mariah`. Add a small lookup table or rely on the persona slug.

### F12 [P1] — Empty-state copy in the Activity tab is gendered ("she's working / she hasn't taken any actions")

`web/app/(app)/workers/[workerId]/activity/page.tsx:203` hardcodes the pronoun "she" in the empty-state. Mariah/Priya/Nori/Peta are she, **Jamal is he**. If a viewer drills into Jamal during a sparse range, the copy will say "Jamal hasn't taken any actions yet. Once she's working…".

- File: `web/app/(app)/workers/[workerId]/activity/page.tsx:201-206`
- Fix: use `they` or restructure to "No activity yet for {workerName}. Activity will appear here once they're working."

### F13 [P1] — "Workers" sidebar entry conflicts with "Team" entry (duplicate roster nav)

Sidebar has both:
- `Team` → `/team` (the real roster)
- `Workers` → `/workers` (the stub from F1)

This duplicates the same destination and means clicking "Workers" forces an extra redirect. It also confuses the operator's mental model — "what's the difference?" — and forced me to spend the first 20 minutes of this audit chasing that question.

- File: `web/components/skycot/nav-items.tsx:24-25`
- Fix: drop the `/workers` nav entry; let `/team` be the only roster surface in the sidebar. `/workers/{id}/...` deep-links keep working because `Workers` was never the intended landing — `Team` always was.

---

### F14 [P2] — Worker-card "Open profile" link goes directly to `/overview`, NOT to `/workers/{id}` (which would redirect)

This is actually CORRECT behavior (`web/components/skycot/TeammateCard.tsx:122`) — but the in-code comment is the only place it's documented. Worth surfacing as a deliberate polish call so future briefs don't "fix" it by reverting to `/workers/{id}`.

### F15 [P2] — `web/components/skycot/worker/WorkerPersonalityCard.tsx:117` falls through to `DEFAULT_PERSONALITY` silently — no console warning

When `PERSONALITIES[worker.name]` misses (which happens for Peta + Nori today, see F4), the component renders `DEFAULT_PERSONALITY` without any warning. A dev-mode `console.warn("Missing personality for", worker.name)` would have caught the Peta/Nori gap in CI long ago.

- Fix: add a dev-only warn when the personality map misses.

### F16 [P2] — `/workers/{id}/[tab]` placeholder page exists for tabs that ALSO have a real route — dead code drift

`web/app/(app)/workers/[workerId]/[tab]/page.tsx` is the dynamic stub that renders "Overview — coming in E4-b" etc. But the seven sibling static routes (overview/activity/memory/skills/trust/escalation/settings) all exist and are real. The dynamic stub is unreachable. Next.js prefers the static segment, so the placeholder text in `TAB_BRIEFS` (which references long-finished briefs like `E4-b`) is purely vestigial.

- File: `web/app/(app)/workers/[workerId]/[tab]/page.tsx`
- Fix: delete `[tab]/page.tsx` and `[tab]/layout.tsx`. The dead-code-drift rule in CLAUDE.md applies here.

### F17 [P2] — DutiesCard title is "Responsibilities" but the user-facing vocab per memory is "Schedules"

Per the operator vocab memory (`project_vocabulary_duty_vs_schedule`): UI label is **Schedule**, code is **Duty**. The card heading says "Responsibilities" (`web/components/skycot/worker/DutiesCard.tsx:89`), which is a third synonym that matches neither.

That said — "Responsibilities" reads warmer than "Schedules" in this context, and the memory is silent on whether "Responsibilities" is allowed. Flag for operator decision, don't auto-rename.

### F18 [P2] — Status pill semantics inconsistent: "Waiting on you" is used both for `waiting` and (implicitly) when `pendingApprovalCount > 0`

The fixture overlay logic at `web/lib/sample-team.ts:81-92` overrides `baselineStatus` to `waiting` when there's a pending approval, then displays "Waiting on you". On `/team` four cards say "Waiting on you" because four workers have pending approvals — but the user is shown 5 *pending* approvals in `/approvals`. One worker (Mariah) has 2 approvals; she still shows one badge. The visual signal is fine — the semantic is just "this teammate needs you" — but a small badge with the actual pending count would be a bigger Claim-1 hit ("I have 3 approvals waiting for you" feels colleague-like).

### F19 [P2] — Settings tab — destructive actions not surfaced

`/workers/{id}/settings` is a 150-line page. Operator question: "Can operator add/edit?" — Add/edit responsibilities works (F7 notes the gaps), but I didn't verify the freeze + restart + retire flows because the preview renderer died before I could click them. Worth a follow-up sweep specifically for `/settings`.

---

### F20 [P3] — `5 teammates, one team.` headline uses Audiowide (per brand-consistency pass), which renders the period as a small dot next to "team."

Aesthetics nit only — Audiowide treats the period as monospaced + same-height as caps, so it looks a touch off in screenshots. Probably fine on the demo screen.

### F21 [P3] — `/api/team` (no v1) returns the full SPA HTML shell, not a 404

`curl http://127.0.0.1:3002/api/team` returns a full HTML page (Next.js's default 404 catchall serves the SPA). Browsers shouldn't hit this — the SPA proxies through `/api/v1/...` — but if a curious dev hits the wrong path, the response is misleading. Low priority.

---

## Resolved since last audit

No prior audit at `docs/skycot/ux-audits/`. (`docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` exists but per CLAUDE.md is operator-rejected and explicitly not to be used as a source.)

## Orientation score

**62 / 100.**

The visual register is good (C2 holographic locks landed cleanly, brand-consistency pass is visible). The 5-worker roster reads as colleagues at-a-glance, level numbers are legible, status pills are clear. But the demo-blocker count (4 P0s) is high, and three of them are existential to the two claims this scene is *about*: no Run-now (Claim 1's "give them work to do"), Peta/Nori-as-defaults (Claim 1's "they're colleagues"), and zero visible delegation in the audit (Claim 2's entire premise). One worker out of five having no traits + two workers having no personality block is a >40% failure rate on the most camera-visible Claim-1 signal.

The good news: every one of the four P0s is a 5–30 minute fix, and three of them are pure fixture work (Peta/Nori personality + traits, a sample delegationChain row, hiding the MOCK-OFFLINE pill). Adding Run-now to DutiesCard is a small bit of real wiring against the existing gateway workflow-execute endpoint. None of these need new architecture.
