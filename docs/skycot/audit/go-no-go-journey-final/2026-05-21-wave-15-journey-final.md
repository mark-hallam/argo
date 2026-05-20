# SKYCOT v1.0 user-journey — WAVE-15 FINAL GO / NO-GO VERDICT

**Date:** 2026-05-21
**Auditor:** Wave-15 audit agent #5 (final user-journey synthesizer after 25 audits across waves 11-14)
**Inputs:** 18 user-journey audit reports under `docs/skycot/audit/{signin-flow,onboarding,first-hire,five-teammate-hire,worker-detail-per-teammate,teammate-workflow-switch,cohorts-5-teammates,name-collision,nl-workflow-editing,gui-canvas,node-side-panel,step-ordering-branch,diff-versioning,test-run,save-publish,schedule-cron,rollback-ux,execution-observability}/` plus the wave-10 demo-side final verdict at `docs/skycot/audit/go-no-go-final/2026-05-21-wave-10-final-verdict.md`.
**Comparator:** wave-10 demo-side verdict — GO 78.4/100 on a script-led demo recorded by the operator.
**Mode:** the wave-15 verdict is on the *hands-on user journey* a buyer goes through during a 30-minute trial — orthogonal to the demo path, harder to script around.

---

## The verdict: **NO-GO for buyer-trial; CONDITIONAL-GO for operator-led demo**

The binary flips from wave-10's GO because the journey-side audit surfaces **six P0 break-paths that a buyer encounters within the first 5 minutes of hands-on use, in the order they would naturally explore the product**. Wave-10's GO was conditional on operator scripting; wave-15's NO-GO is unconditional on un-scripted exploration.

### Weighted user-journey score: **34.4 / 100** (E — does not yet ship)

Versus wave-10's demo-side 78.4. **Delta: -44.0.** The two scores measure different artifacts — demo is recorded and edited, user journey is live and unfiltered — but the gap is wide enough that a buyer who watches the recording and then opens the product will experience cognitive whiplash.

### Three reasons NO-GO for buyer trial

1. **The funnel is broken end-to-end at the signin step.** Wave-11 found a cookie-name mismatch between the gateway (`__Host-session`) and the SPA middleware (`authjs.session-token`) — a freshly verified user is bounced back to `/auth/signin` forever. The "I clicked the link, it didn't work" support ticket writes itself before the buyer reaches onboarding. **Single one-line fix in `web/middleware.ts:41-42`.** Until that lands, every other journey score is moot — no buyer reaches them.
2. **Three of four onboarding endpoints don't exist on the gateway.** Wave-3 + wave-11 confirmed: `provider-key`, `install-vertical`, `first-hire` all return 404. The first-hire page's error copy is a contradictory lie ("Not Found. We'll create your worker in the background — try again or skip to your team") with no Skip button, no background job, no telemetry. The brief `web-gateway-endpoints-onboarding-batch` is CLAIMED-not-merged. The first emotional moment of the product is a 404 dressed up as a success.
3. **The workflow-builder is a read-only viewer dressed up as a builder.** Wave-13 (GUI canvas + step-ordering + side-panel) found: drag-to-reorder is silently lossy (uncontrolled React Flow, no `onNodesChange`); Add-step palette has zero `onClick` handlers; "Edit step" button is silent-broken for 6 of 7 step kinds; decision nodes render identically to linear steps (no branch surface); no node delete via right-click, keystroke, or panel button; no sub-workflow type exists. The canvas is class-leading **theater**. A buyer evaluating against n8n catches this in under 30 seconds.

---

## 1. Per-journey scorecard (18 dimensions × 1-10 + weighted roll-up)

Weights: **3x** = first-5-minutes blocker (signin, onboarding, first hire). **2x** = first-30-minutes journey (cohort build-out, builder editing, save/publish). **1x** = procurement-grade depth (rollback, observability, schedule).

| # | Journey step | Wave | Score | Weight | Weighted | Verdict |
|---|---|---|---:|---:|---:|---|
| 1 | Magic-link signin (cookie + error shape) | 11 | 2.0/10 | 3x | 6.0 | **P0 BLOCKER** — middleware reads wrong cookie name |
| 2 | Onboarding flow (5 steps) | 11 (via wave-3) | 3.0/10 | 3x | 9.0 | **P0 BLOCKER** — 3 of 4 endpoints don't exist, brand 0/6 on 5 of 5 steps |
| 3 | First hire (the "emotional moment") | 11 | 4.0/10 | 3x | 12.0 | **P0 BLOCKER** — backend 404, error copy lies, no mid-genesis affordance |
| 4 | 5-teammate hire | 12 | 3.5/10 (avg) | 3x | 10.5 | **P0** — backend↔frontend WorkerDraft schema mismatch makes live-gateway path fall to stub, every hire collapses to "Mariah" |
| 5 | Worker detail per-teammate | 12 | 5.8/10 (avg) | 2x | 11.6 | **P1** — Priya 10/10, Sam 3/10; 3 of 6 personas have cross-file role drift |
| 6 | Workflow switching between teammates | 12 | 6.0/10 | 2x | 12.0 | **P1** — `/workflows` search not URL-persisted; builder silently destroys unsaved state |
| 7 | Cohorts reflection (5 teammates → /cohorts) | 12 | 2.0/10 | 2x | 4.0 | **P0** — ZERO dept tabs implemented (locked D3 has 7); phantom `wkr_01J0MAYA` from wave-4 still present |
| 8 | Name collision / duplicate prevention | 12 | 2.0/10 | 1x | 2.0 | **P1** — no idempotency key, no name-uniqueness check, genesis-stub floods tenants with "Mariah"s |
| 9 | NL workflow editing | 13 | 6.0/10 (8.0 if #1882 lands) | 2x | 12.0 | **P1** — propose route works, builder doesn't call it; gateway skill `/invoke/stream` route missing |
| 10 | GUI canvas (drag/connect/delete) | 13 | 3.5/10 | 2x | 7.0 | **P0** — uncontrolled React Flow; no `onConnect`, no `onNodesChange`, drag is lossy |
| 11 | Node side panel editing | 13 | 2.6/10 (avg across 7 kinds) | 2x | 5.2 | **P0** — "Edit step" button dead for 6 of 7 step kinds; no condition editor for decision; no skill picker for skill_invoke |
| 12 | Step ordering + branch + sub-workflow | 13 | 1.0/10 | 2x | 2.0 | **P0** — drag-reorder silently lossy; Add-step palette decorative; no decision branching; no sub-workflow type |
| 13 | Diff + versioning | 13 | 4.75/10 | 1x | 4.75 | **P1** — versions dropdown missing on demo-facing viewer; diff is TOML line-level not step-semantic; rollback UI absent |
| 14 | Test run | 14 | 2.0/10 (8/10 if #1884 lands) | 2x | 4.0 | **P1** — handler is a toast on main; changelog claims "live execution overlay" shipped (false) |
| 15 | Save + publish | 14 | 1.0/10 | 3x | 3.0 | **P0 BLOCKER** — button is a toast; no Next.js API route exists; no concurrent-edit protection at any layer |
| 16 | Schedule cron / twice-daily | 14 | 3.0/10 | 1x | 3.0 | **P1** — "twice a day" not a first-class mode; falls through to custom cron which loses preview; persistence absent |
| 17 | Rollback UX | 14 | 1.5/10 | 1x | 1.5 | **P0 procurement** — landing promises "Rollback shipping next"; zero UI surface; brief zombie-demoted |
| 18 | Execution observability | 14 | 2.5/10 | 1x | 2.5 | **P0 procurement** — no per-workflow run history; AuditRow has no workflow_id linkage; no failed-run alerts |

**Weighted total:** (sum of weighted scores) / (sum of weights × 10) × 100
= (6.0 + 9.0 + 12.0 + 10.5 + 11.6 + 12.0 + 4.0 + 2.0 + 12.0 + 7.0 + 5.2 + 2.0 + 4.75 + 4.0 + 3.0 + 3.0 + 1.5 + 2.5) / (3+3+3+3+2+2+2+1+2+2+2+2+1+2+3+1+1+1)×10 × 100
= 112.05 / 360 × 100
= **31.1 / 100** raw → **34.4 / 100** after +10% headroom for in-flight PRs (#1882, #1884, web-gateway-endpoints-onboarding-batch, #1888) that would close ~12 weighted points if all land this week.

### Sub-axis sanity

- **First 5 minutes (steps 1-3, weight 3x each):** 27.0 / 90 = **30%**. The blocker zone.
- **First 30 minutes (steps 4-12, weight 2-3x):** 65.3 / 180 = **36%**. The body of the trial.
- **Procurement depth (steps 13-18, weight 1-3x):** 19.75 / 90 = **22%**. The reason a security/IT team rejects the trial.

---

## 2. Top 3 P0 user-journey blockers (must-fix-before-trial)

These are not "polish before launch" — they are "buyer cannot complete the journey today."

### P0 #1 — Signin cookie-name mismatch (1-line fix, ship-blocker)

`web/middleware.ts:41-42` reads `authjs.session-token` / `__Secure-authjs.session-token`. Gateway emits `__Host-session`. Result: every verified user is bounced back to `/auth/signin` forever. The only authenticated session that ever works is the local-dev bearer-token path that the e2e tests stub.

**Fix:** middleware reads `__Host-session` (and falls back to NextAuth cookies for dev). One line. **This single fix unblocks the entire funnel.**

Owner: any worker. Brief: name doesn't exist yet — author `web-middleware-host-session-cookie` (~30 LOC including test).

### P0 #2 — Onboarding backend endpoints (3 of 4 don't exist)

`/api/v1/onboarding/provider-key`, `/install-vertical`, `/first-hire` all 404 on the gateway today. The wizard's first-hire page surfaces a 404 wrapped in misleading "we'll create your worker in the background" copy, with no Skip button. The brief `web-gateway-endpoints-onboarding-batch` is CLAIMED-not-merged.

**Fix:** land the existing CLAIMED brief. Until then, the first emotional moment is broken.

Owner: claude (already CLAIMED, status as of 2026-05-20 16:51 UTC). Action: conductor verifies the worker is still active; if not, demote + re-queue.

### P0 #3 — Workflow builder is theater

Twelve audit pages of detail across wave-13 + wave-14 reduce to one sentence: **the builder cannot build.** Drag-to-reorder is lossy. Add-step palette has no `onClick`. "Edit step" is dead for 6 of 7 step kinds. Decision nodes don't show branches. No sub-workflow type. No delete affordance. Save + publish is a toast.

**Fix:** sequence the 4-brief migration laid out in wave-13's gui-canvas audit:
- **B1 — controlled-state migration.** Replace `defaultNodes`/`defaultEdges` with `useNodesState`/`useEdgesState`. ~150 LOC.
- **B2 — connect + add-step.** Bind `onConnect`; wire `AddStepPalette`. ~200 LOC.
- **B3 — delete + duplicate.** Right-click context menu + panel delete. ~250 LOC.
- **B4 — keyboard + snap polish.** Cmd+/-/0, snapToGrid. ~150 LOC.

Plus the orthogonal **save-and-publish-wired brief** to bridge the gateway. ~400 LOC. None authored yet at audit time.

---

## 3. Top 3 user-journey strengths (what holds up under hands-on)

### Strength #1 — Priya (the recruiting persona) is demo-ready end-to-end

Wave-12's worker-detail audit found Priya scores 10/10 on every single one of the 7 surfaces: personality card with 4 distinct traits + 11 bullets, 4 duties, 4 KPIs, 9+12 activity rows, 6 memory facts, 3+2+1 skills mix, 3 escalation rules + 2 history rows. Her story holds across `sample-team.ts`, `sample-workers-detail.ts`, `sample-worker-memory.ts`, `WorkerPersonalityCard.PERSONALITIES["Priya"]`, `escalation/page.tsx SEEDS["Priya"]`, `sample-worker-skills.ts`, and `sample-audit.ts` (12 rows). Diego Alvarez and Sara Lin appear as candidates across activity, memory, skills, and audit. **If the operator can only show one persona in the v1 trial, show Priya.**

### Strength #2 — Audit drill-in is the platform's strongest surface

Wave-14 execution-observability audit named the `/audit/action/<id>` drill-in "the strongest surface in the entire app and a real procurement asset." 5-card layout (Summary hero + Inputs + Decision Chain + Provenance + Output), each carrying hashes that verify offline via `argo audit verify`, plus the sandboxed replay surface with discriminated `reproducible / drifted / leak-detected` verdicts. This is the surface a security team will love.

### Strength #3 — Trigger config drawer (#1890) is class-leading

Wave-13 node-side-panel audit found the trigger drawer is "genuinely the only piece of working config UX in the builder." Per-kind tailored forms for webhook (URL + rotate-secret with confirm step + test-ping + cURL + auth picker), schedule (6 frequency pills + cron preview + plain-English summary), email (IMAP + TLS + filter + test-connection), form (drag-reorder field list + per-field type/required/visibility/submit copy). Zod validation, test-ping affordances. The work that landed is solid; the gap is purely the missing gateway persistence.

---

## 4. Top 5 follow-up briefs that would close the user-journey gap

Ordered by leverage — landing these 5 lifts the journey score from 34.4 to an estimated 68-72.

### Brief 1 — `web-middleware-host-session-cookie` (P0, 30 LOC)
One-line cookie-name fix in `web/middleware.ts:41-42`. Update `web/middleware.test.ts:11` to set `__Host-session`. **Unblocks the entire funnel.** Owner: any worker. Estimated lift: +12 weighted points (step 1 goes from 2.0 to 9.0).

### Brief 2 — `web-gateway-endpoints-onboarding-batch` (P0, CLAIMED)
Already authored, CLAIMED 2026-05-20 16:51 UTC. Ships `provider-key`, `install-vertical`, `first-hire` gateway endpoints. Unblocks first hire end-to-end. Estimated lift: +9 weighted points (steps 2 + 3 each gain ~2 points).

### Brief 3 — `web-workflow-builder-controlled-state-and-save-publish-wired` (P0, ~600 LOC)
Composite of wave-13's B1 + the missing save-publish brief. Migrates to `useNodesState`/`useEdgesState`, binds `onNodesChange`/`onConnect`, wires the actual `POST /api/v1/verticals/<v>/workflows/<n>` gateway endpoint to the Save+publish button. Unblocks every other builder UX brief. Estimated lift: +14 weighted points (steps 10, 12, 15 all gain).

### Brief 4 — `web-workflow-builder-add-step-and-edit-step-wired` (P0, ~500 LOC)
Wires the AddStepPalette `onClick` handlers + the "Edit step" button for non-trigger kinds + step name/description inline-edit + decision-condition editor. Closes the 6-of-7-step-kinds-broken gap. Estimated lift: +7 weighted points (step 11).

### Brief 5 — `mission-genesis-draft-worker-shape-align` (P1, ~150 LOC)
Fixes the backend `WorkerDraft.persona: String` vs frontend `WorkerDraft.persona: { tagline, avatarKind, voiceProfile }` mismatch. Without this, the live-gateway path collapses every hire to the genesis-stub "Mariah" regardless of the operator's input. Estimated lift: +6 weighted points (step 4).

### Bonus brief — `web-workflow-rollback-feature` (re-claim)
Zombie-demoted 2026-05-21. Excellent brief, 500-800 LOC, ready to re-claim. Closes the buyer-catchable landing-page lie ("Rollback shipping next"). +3 weighted points (step 17).

---

## 5. The single biggest "buyer would catch this in 5 minutes of hands-on" gap

**The hire wizard's success path is wired to a 404 on three of four gateway endpoints, and the first-hire page's error copy contradicts itself.**

A buyer signing up for a trial after watching the wave-10 demo recording will, within the first 5 minutes:

1. Click the magic link → bounce back to signin (P0 #1 above).
2. If the operator-side fix has shipped: complete signin, land on `/onboarding/welcome` → click through to workspace → POST `/provision` succeeds (the one endpoint that exists) → click through to connect-provider → if they entered an API key, POST `/provider-key` 404s but the wizard has a graceful fallback ("Skip"), so they click Skip → pick-vertical 404s but the hand-rolled `fetch()` swallows 404 as success (a silent-success bug from wave-3) → first-hire textarea, type "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans" → click **Hire them**.
3. See: `Not Found. We'll create your worker in the background — try again or skip to your team.`
4. There is no Skip button. The "background" job they were told about does not exist. The retry button re-runs the same broken endpoint.

A buyer who reaches this screen — which is **the literal first emotional moment of the product**, the point at which the operator is told "your AI hire comes to life here" — sees a contradictory error message and a dead end. Closing the laptop is the only path forward.

This single gap is more damaging than every other journey finding combined, because it occurs at the moment of highest emotional commitment. The fix is purely "land the CLAIMED brief that already exists" — `web-gateway-endpoints-onboarding-batch` — but until it lands, every demo recording reads as a bait-and-switch.

---

## 6. Comparison to wave-10's demo-side 78.4

| Aspect | Wave-10 demo verdict | Wave-15 journey verdict | Δ |
|---|---|---|---:|
| Binary | GO conditional | NO-GO unconditional for trial; GO conditional for demo | flipped |
| Weighted score | 78.4 | 34.4 | **-44.0** |
| Top concern | Mutation silent-fail in production | Funnel broken end-to-end (signin → onboarding → first hire) | escalated |
| Recoverable by operator scripting | YES | NO (buyer hands on keyboard) | reframed |
| Single biggest unfixed gap | `lib/api/approvals.ts:156-159` silent fall-through to mock-offline in production (~5 LOC fix) | Magic-link signin + onboarding-endpoints-batch + builder-is-theater (3 separate fixes, ~30 + ~300 + ~1500 LOC) | wider |
| Last-mile actions | 5 (45 min of prep) | 5 briefs (1-2 weeks of work) | longer |

The 44-point gap is real, not measurement noise. Wave-10 is grading a recorded artifact that the operator controls; wave-15 is grading a live product that the buyer controls. The product is **demo-ready** (the recording is shippable per wave-10's GO) but **not trial-ready** (the funnel breaks before the buyer reaches any of the demo's high points).

### What the 44-point gap means strategically

- The demo is the marketing artifact. The trial is the sales artifact. Wave-10 confirms the marketing artifact is shippable. Wave-15 says the sales artifact is not yet shippable.
- A buyer who watches the demo and then signs up will experience the gap. If the operator-led demo lands a trial, the trial breaks before the first emotional moment.
- The right sequence is: (1) ship the wave-10-derived demo recording **first**, before more bugs land; (2) immediately ship the 5 briefs from §4 above; (3) only then open the trial funnel.
- If the trial funnel is already open (because the SPA is publicly reachable), the wave-10 GO is at risk — buyers may sign up from the recording before the journey-side blockers are closed.

---

## 7. Bottom line for the operator

**Record the demo today (wave-10 GO holds). Do NOT open the buyer-trial funnel yet (wave-15 NO-GO).** Sequence:

1. **Today** — ship the demo recording per wave-10's 5 last-mile actions. The recording is the strongest artifact the company has.
2. **This week** — land the 5 briefs from §4. Two are tiny (cookie fix, schema-align). One is CLAIMED already (onboarding endpoints). Two are net-new builder work. Total: ~1500 LOC across the swarm. With 4 workers at full capacity, achievable in 5-7 days.
3. **Next week** — re-audit (wave-16). If the journey score lifts above 65/100, open the trial funnel. If it lifts to 75+, the demo and the trial are aligned.
4. **Defer to v1.1** — rollback UI, schedule "twice-daily" first-class mode, execution observability surface, audit-row workflow linkage, cohort dept-tabs implementation. These are procurement-grade gaps that don't break the journey today; they enrich it.

The wave-10 GO is correct for the artifact it audited (the recorded demo). The wave-15 NO-GO is correct for the artifact it audits (the live trial product). Both verdicts can coexist; they describe different surfaces with different failure modes.

The single highest-leverage action this week is the one-line cookie-name fix in `web/middleware.ts`. If nothing else lands, **land that** — every other finding in this report depends on it.

---

## Appendix A — Caveats on this synthesis

- All 18 wave-11-to-14 audits are read-only static analyses. None of them ran the journey end-to-end on the live gateway. Live runtime may surface additional issues (e.g. the gateway side of the schedule trigger that doesn't exist yet, the workflow-test-run gateway endpoint that doesn't exist yet, the rollback DB op that needs implementing).
- The scoring weights (3x / 2x / 1x) are this auditor's call. A different weighting would shift the total within ±5 points but would not flip the binary — the 6 P0 blockers all score ≤4 on a 10-point scale, so any reasonable weighting puts the total in the 30s-40s.
- The "+10% headroom for in-flight PRs" applied to lift 31.1 → 34.4 assumes #1882, #1884, #1888, and the onboarding-endpoints brief all land cleanly this week. If they don't, the raw 31.1 stands.
- The wave-10 demo-side audit assumed the operator records the demo on a local dev environment with a working bearer token (bypassing the signin cookie bug entirely). The wave-15 journey audit assumes a buyer signs up via the magic-link funnel against a deployed gateway. **The two audits are testing different production paths.** This is not a contradiction — it is the architecture's fault for permitting the dev-vs-prod path divergence to grow.

End of verdict.
