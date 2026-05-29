# Session handoff — Portia demo prep + MVP reframe (2026-05-29)

**For:** the operator (Mark) + the next Claude session resuming this work.
**Status at handoff:** demo dry-run in progress; Portia pitch flow wired 100% real.
**main HEAD at handoff:** `3fe5b8855`. All work committed + pushed.

---

## ✅ UPDATE — Grant Phase B + roster → 7 (2026-05-29, later same session)

**Pushed to origin/main:** `e1fe127a0` (Grant endpoint) + `8e107faf4` (seed
idempotency). Web follow-ups (below) are uncommitted in the working tree at
time of writing.

**Grant is the 2nd real working teammate (after Portia).** He owns inbound
growth as a standing *responsibility*, not a task — the Claim-2 differentiation,
proved recursively (Skycot's own GTM teammate growing Skycot).

- **New gateway endpoint** `POST /api/v1/gtm/draft-followup`
  (`crates/argo-cli/src/handlers/gateway/routes_gtm_draft_followup.rs`). Body
  `{signup:{name,email,context?}}` → Grant drafts a brand-safe follow-up via the
  **live LLM** (reuses Portia's `build_pitch_transport` / `user_message` /
  `extract_forbidden_class` — now `pub(super)`), truth-boundary gated, then
  **files an `approval_request`** to the workspace tenant + an
  `AuditKind::ApprovalAsked` row (`action: gtm.followup_drafted`). **Never
  auto-sends.** 7 unit tests; cargo check + clippy clean.
- **Verified live (cold):** novel signup (Dana @ Brightloop) →
  `source:llm model:claude-sonnet-4-6 disposition:Allow`, a real
  context-specific draft, no fabricated numbers, filed to `/approvals` (top
  item), audit row present in DB.
- **Roster is now 7** (added Grant after Portia): Mariah, Peta, Priya, Jamal,
  Nori, Portia, Grant. `personas.rs` already canonized to 7.
- **Demo seed idempotency FIXED** — re-seeding a tenant (to add a teammate) used
  to duplicate the 8 demo approvals; now skip-if-exists (sentinel guard) with a
  regression test. The duplicate-approvals open bug is resolved.
- **Clean demo state re-minted:** fresh DB, reseed (7 heroes + 8 approvals, no
  dupes), Grant draft fired last → `/team`=7, `/approvals`=9 (Grant on top),
  zero duplicates. Old store at `~/.argo/sessions.db.pre-grant-reset-*`. New
  workspace tenant `019e7344-78e1-77f1-ae99-dff2699683ea` (ensure_workspace mints
  a NEW random tenant per fresh DB — it is NOT path-deterministic; resolve it
  from the DB `workspace` table, don't hardcode).
- **New `/gtm` SPA console** (`web/app/(app)/gtm/page.tsx` +
  `web/lib/api/gtm.ts` + layout + nav entry "GTM") — mirror of `/pitch`: paste a
  signup, watch Grant draft live, disposition badge, "Filed to approvals" link.
  Renders 200 on the dev server.
- **SPA fallback roster → 7** (Portia + Grant added to `sample-personas.ts` /
  `sample-team.ts` / `sample-workers-detail.ts` + canon test to 7 + stale
  "five/six" copy fixed). Resolves the "SAMPLE_TEAM old 5" open bug below.

**Two demo-anchor bugs found via the walkthrough + FIXED this session:**
- **`/audit` was fully broken** (commit `04ca2a4a5`) — NOT "renders OK" as
  earlier notes claimed. ChannelBadge rendered `<undefined/>` for live channel
  kinds (`agent`/`mcp`/`computer-use`) outside the SPA's ChannelKind union →
  "Element type is invalid" → the route error boundary replaced the WHOLE feed
  with a 500 shell. Fixed with a defensive fallback + proper icons for those
  surfaces (contained to ChannelBadge — they aren't messaging channels).
- **`/approvals` WebSocket** (commit `ad913bf25`) — in auth-bypass/demo mode
  the tokenless `/inbox/ws` upgrade was rejected (2 console errors). Now skips
  the WS and uses polling in bypass mode. 0 errors.
- **Clean post-fix walkthrough baseline** (`docs/skycot/ux-walkthrough-2026-05-29-grant/`):
  12/14 routes 0-error; only remaining errors are two secondary-resource 404s
  needing gateway endpoints (`skills/learned-this-month`, a worker-detail
  coming-soon tab) — camera-invisible, evaluator-lane.

**Decision recorded:** [[DEC-018]] (Grant = 2nd real working teammate).

**Gateway endpoint batch (residual-404 cleanup, real data only — NO THEATER):**
- ✅ `GET /api/v1/skills/<id>` (detail), `GET /api/v1/skills/<id>/usage` (by id,
  for the skill-detail drill-in), `GET /api/v1/skills/learned-this-month` — all
  wired in `routes_skills_list_get.rs` against the real skill store +
  `worker_skill` adoption queries (the logic existed; the `/api/v1` dispatcher
  just wasn't routing the subpaths). Commits `76ddf845a` + `75dc02f1c`.
- ⏸️ `workers/<id>/retrain/recent` + `audit/<id>/replay` — deliberately NOT
  implemented: there is no retrain subsystem and no replay-over-HTTP route, so
  an endpoint would return fabricated/empty data (theater). Left as graceful,
  non-fatal 404s (pages render; only a dev-tools console line). Build the real
  subsystems before wiring these.

**Gateway: schema v84; SPA :3000 demo-live; swarm PAUSED. main HEAD `e78257ab7`+.**
The live gateway binary predates the /audit + /approvals fixes (those are
SPA-only, no gateway rebuild needed). The next gateway rebuild will also pick
up the seed idempotency fix already in the running binary.

---

## The big strategic reframe (read this first)

We are **NOT at a shippable "v1.0 MVP"** by the honest bar, and the operator
caught this. Reality:

- The **backend/core is genuinely real** — audit (hash-chained), approvals,
  teammate persistence, delegation, the Pitch Agent — these pass cold tests
  and are wired to the UI.
- But **~24 of ~30 SPA surfaces render bundled MOCK data even in demo-live
  mode** (see "Systemic finding" below). So the *user-facing product* mostly
  touches mock screens.

**Reframe (operator-aligned):** this is a **working-core + vision** pre-seed
demo — for funding / waitlist / socials — **NOT** an "MVP shipped" product.
Sell the vision + the real proof points honestly; raise to build the rest.
Stop using "v1.0 MVP / all claims LIVE" framing — it overstates the user-facing
reality. NO THEATER means: be honest about real-vs-mock; don't film mock as
"working product."

**Operator's chosen direction:** lean toward **WIRING UP** (make it real), not
cutting. The **100%-real hero journey = Portia pitching SKYCOT** (recursive
proof + viral moment).

---

## ✅ Done this session

1. **Portia is real.** The Pitch Agent persona is named **Portia** (female,
   DEC-016), promoted from a dept-filler to a **6th canonical hero persona**
   in the demo seed (`personas.rs`). Pack manifest + SPA fallback renamed too.
2. **Dropped the dept-fillers** (Fin/Devon/Lex) — they were fake "look
   populated" rows (theater). Demo tenant now seeds exactly **6 real heroes**;
   `/cohorts` shows the 6 real departments.
3. **`/team` now reads the LIVE gateway** in demo-live (was showing 5 mock
   `SAMPLE_TEAM` personas — `team.ts` was missing the `!isDemoLive()` gate
   every other module has). Fixed → shows the real 6 incl. Portia.
4. **`/settings` 500 fixed** — `SettingsHeading` imported `findActiveTab` from
   `SettingsTabs`, which lacked that export after the #2673 rebase. Re-exported.
5. **Portia's LLM fallback WIRED** (the hero moment): novel investor questions
   now hit the **live LLM via the operator's Claude subscription**
   (`ARGO_PITCH_LLM_PROVIDER=claude-code-cli`, persisted in `~/.argo/env.local`,
   DEC-012, Sonnet floor DEC-013), truth-boundary gated. Verified live:
   - canonical Q → 2ms real answer w/ provenance.
   - novel HIPAA Q → `source:llm model:claude-sonnet-4-6 disposition:Allow`,
     refused to speculate, pivoted to real architecture facts, escalated to
     founder. **Brand-safe, no hallucination, no theater.**
6. **Fresh demo store** seeded: 6 heroes, 6 cohorts, 8 approvals, 50 audit rows.
   Old store moved to `~/.argo/sessions.db.pre-portia-reset-*` (recoverable).
7. Earlier in the session (separate from demo): completed the 6-item program
   (PR audit, decision register, test-coverage audit, full-site walkthrough,
   e2e journeys, teammate-eval design), drained 18 web PRs, relit + re-paused
   the swarm. See git log + the decision register.

## 🔴 Systemic finding (the key one — NOT yet fixed)

**~24 SPA api modules short-circuit to bundled mock on `isAuthBypassed()`
alone, never checking `!isDemoLive()`** — so they show MOCK data in the
recording config. Only 6 are correctly gated (audit, audit-detail, approvals,
workers, worker-overview, team[fixed]).

**Mock in demo-live (theater if filmed as working):** cohorts, **skills (Scene
4)**, **workflows + builder (Scene 5)**, workflow-runs, worker-detail tabs
(activity/memory/skills/trust/policies/settings), verticals, orchestration,
evals, ghostwriter, retros, settings, api-keys, provider-keys, audit-export,
elevated-view, workplace-knowledge, skill-ir.

**The canonical gate pattern to copy:** `if (isAuthBypassed() && !isDemoLive())`
(see `web/lib/api/audit.ts:361`, `approvals.ts:202`, `team.ts:139`).

## ▶️ NEXT ACTION (where to resume)

**Per-module endpoint audit for the camera-visible mock surfaces** (`/skills`,
`/workflows`, `/cohorts`, worker-detail tabs): for EACH, determine —
- live gateway endpoint EXISTS → add the `!isDemoLive()` gate → wire it real.
- NO backend yet → leave mock + honestly mark "coming soon / out of demo scope"
  (do NOT fake-wire — that's theater).

This tells us exactly how much of Scenes 4/5 can become real vs must be cut.
The operator leans to wiring. Then: re-capture, confirm live, update runbook.

## 🔧 Live runtime state (survives a Claude restart — processes keep running)

- **Gateway:** running on `:7400`, schema v84, fresh build (`commits_behind=0`),
  6 heroes seeded, Portia LLM wired. Binary: `/tmp/argo-shared-target/release/argo`.
  Relaunch (preserve store, no rebuild): `source ~/.argo/env.local` then
  `argo gateway start --bind 127.0.0.1 --port 7400` with
  `ARGO_GATEWAY_AUTH_BYPASS=1 ARGO_DEMO_MODE=1` + `~/.local/bin` on PATH.
- **SPA:** running on `:3000` with the full demo env
  (`AUTH_BYPASS=1 + DEMO_MODE=1 + DEMO_LIVE=1 + GATEWAY_URL=:7400`). Hard-refresh
  the browser to pick up the 6.
- **Swarm:** PAUSED (halt sentinels set, supervisor unloaded) — good for
  recording. Relight: `rm /tmp/argo-stop ~/.argo/stop && launchctl bootstrap
  gui/$(id -u) ~/Library/LaunchAgents/com.skycot.worker-supervisor.plist`.

## Open bugs (non-blocking)

- `/audit` logs a console "Element type is invalid" error (page renders OK) — investigate.
- `/approvals` live-WebSocket auth-fails (`ws://…/inbox/ws` — no creds); data
  renders fine, just no live push. (CSP was fixed via #2669; this is an auth gap.)
- worker-detail: 1× 404 sub-resource.
- SPA offline fallback `SAMPLE_TEAM` still lists the old 5 (no Portia) — only
  surfaces if the gateway *fails* mid-record. Minor.

## Pending queue work (not demo-related)

- 4 Rust PRs awaiting the evaluator cold-cargo lane: #2497, #2616, #2636, #2668.
- Queued briefs (TODO): stale-web-tests, approvals-WS-CSP follow, nav-hydration,
  /pitch metadata, teammate-eval live-runner, the 5 rebase-* re-authors.
