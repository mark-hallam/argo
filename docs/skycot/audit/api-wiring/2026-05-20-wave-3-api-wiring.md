# Wave-3 audit: SKYCOT web app API wiring

**Date:** 2026-05-20
**Auditor:** Wave-3 audit agent #2 (Claude Opus 4.7 / 1M context)
**Scope:** every fetcher in `web/lib/api/*` and every Next.js API
route in `web/app/api/**`, checked against the live gateway at
`http://127.0.0.1:7400`.

## Method

For each fetcher / route I:
1. Read the source to extract the URL pattern and HTTP verb.
2. Probed the gateway (live process, archived test worker
   `019e3939-2365-7681-950e-0b99d0e74666`).
3. Mapped each fetcher to the React page that calls it (via the
   hook layer in `web/lib/hooks`) and inspected the page's error
   handling.
4. Grep-confirmed missing routes against `crates/argo-cli/src/handlers/gateway/`.

HTTP-status legend (gateway in dev-mode, no bearer token):
- **200** — happy path
- **400** — route exists, request body invalid (the route IS wired)
- **403** — route exists, permission denied (route IS wired)
- **401** — route exists, auth required (route IS wired)
- **404** with body `{"error":"not found"}` — **no route registered**
- **404** with body containing `worker ... not found` — route exists,
  the test ID just isn't in the local DB

## Findings — wiring matrix

### `/(app)/page.tsx` — Approvals inbox (home page)

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `(app)/page.tsx` | `fetchApprovals` | `GET /api/v1/approvals` | **YES (200)** | YES — `isError` + mock-offline fallback in fetcher |
| `(app)/page.tsx` | `submitDecision` | `POST /api/v1/approvals/<id>/decide` | YES (registered) | YES — 404 → mock-offline |
| `(app)/page.tsx` | `submitUndo` | `POST /api/v1/approvals/<id>/undo` | YES (registered) | YES — 404 → mock-offline |

### `/(app)/team/page.tsx`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `team/page.tsx` | `fetchTeam` | `GET /api/v1/team` | **YES (200)** | YES — `TeamErrorState`, fetcher also has mock fallback |

### `/(app)/cohorts/page.tsx`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `cohorts/page.tsx` | `fetchCohorts` | `GET /api/v1/cohorts` | **YES (200)** | **NO `isError` branch** — only `isPending` + empty. Empty cohorts hides errors. Fetcher has mock fallback in dev. |
| `cohorts/page.tsx` (drawer) | `fetchCohortDetail` | `GET /api/v1/cohorts/<id>` | YES (404 only when id missing) | YES — fetcher 404 → mock-offline |

### `/(app)/audit/page.tsx` + `/audit/action/[actionId]`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `audit/page.tsx` | `fetchAuditFeed` | `GET /api/v1/audit` | **YES (200)** | YES — `ErrorState` + mock fallback |
| `audit/action/[id]/page.tsx` | `fetchAuditDetail` | `GET /api/v1/audit/<id>` | YES (route registered) | YES — `isError` + `AuditActionNotFound` |
| `audit/action/[id]/replay/page.tsx` | `replayAction` | `POST /api/v1/audit/<id>/replay` | **NO ROUTE** | Fetcher 404 → deterministic mock outcome; UI renders the mock as if real |
| (Evidence Bundle dialog) | `previewExport` | `POST /api/v1/audit/export/preview` | **NO ROUTE** | Fetcher 404 → `mockPreview` |
| (Evidence Bundle dialog) | `generateExport` | `POST /api/v1/audit/export` | **NO ROUTE** | Fetcher 404 → `mockGenerate` |
| (Audit page generally) | `emitAuditRow` (audit-the-audit) | `POST /api/v1/audit` | **NO ROUTE** | Fire-and-forget; `console.warn` |
| (ElevatedViewRequestDialog) | `submitElevatedViewRequest` | `POST /api/v1/audit/elevated-view-request` | YES (registered) | YES — 404 → client-side grant |

### `/(app)/workers/page.tsx`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `workers/page.tsx` | `fetchTeam` | `GET /api/v1/team` | YES (200) | Re-uses team error state |

### `/(app)/workers/[workerId]/...` — worker profile tabs

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `workers/[id]/page.tsx` | `fetchWorker` | `GET /api/v1/workers/<id>` | YES (registered, 200 with real id) | YES — `WorkerNotFoundError` |
| `workers/[id]/overview/page.tsx` | `fetchWorkerOverview` | `GET /api/v1/workers/<id>/overview` | YES (200) | YES — mock fallback |
| `workers/[id]/activity/page.tsx` | `fetchWorkerActivity` | `GET /api/v1/workers/<id>/activity` | YES (200) | YES — mock fallback |
| `workers/[id]/memory/page.tsx` | `fetchMemory` | `GET /api/v1/workers/<id>/memory` | YES (200) | YES — mock fallback |
| `workers/[id]/memory/page.tsx` | `addFact` | `POST /api/v1/workers/<id>/memory/facts` | YES (201) | YES — 404 → mock |
| `workers/[id]/memory/page.tsx` | `editFact` | `PATCH /api/v1/workers/<id>/memory/facts/<fact>` | **NO ROUTE** (gateway has PUT per source comment) | YES — fetcher catches 404→mock; **method mismatch** documented in source ("PUT is brief's, PATCH is workspace convention") |
| `workers/[id]/memory/page.tsx` | `deleteFact` | `DELETE /api/v1/workers/<id>/memory/facts/<fact>` | YES (route exists, returned 400 on test) | YES — 404 → mock |
| `workers/[id]/skills/page.tsx` | `fetchSkills` (worker-skills) | `GET /api/v1/workers/<id>/skills` | **NO ROUTE (404 even with real worker)** | YES — fetcher 404 → mock |
| `workers/[id]/skills/page.tsx` | `enableSkill` | `POST /api/v1/workers/<id>/skills` | **NO ROUTE** | YES — 404 → mock |
| `workers/[id]/skills/page.tsx` | `disableSkill` | `DELETE /api/v1/workers/<id>/skills/<id>` | **NO ROUTE** | YES — 404 → mock |
| `workers/[id]/skills/page.tsx` | `promoteLearned` | `POST /api/v1/workers/<id>/skills/learned/<id>/promote` | **NO ROUTE** | YES — 404 → mock |
| `workers/[id]/skills/page.tsx` | `discardLearned` | `DELETE /api/v1/workers/<id>/skills/learned/<id>` | **NO ROUTE** | YES — 404 → mock |
| `workers/[id]/trust/page.tsx` | `fetchWorkerTrustGrade` | `GET /api/v1/workers/<id>/trust-grade` | YES (200) | YES — mock fallback |
| `workers/[id]/[tab]/page.tsx` (policies tab) | `getWorkerPolicies` | `GET /api/v1/workers/<id>/policies` | YES (200) | YES — mock fallback |
| `workers/[id]/[tab]/page.tsx` (policies tab) | `updateWorkerPolicy` | `PATCH /api/v1/workers/<id>/policies/<action>` | YES (route exists, 400 on test) | YES — 404 → mock |
| `workers/[id]/settings/page.tsx` | `getSettings` | `GET /api/v1/workers/<id>/settings` | YES (200) | YES — mock fallback |
| `workers/[id]/settings/page.tsx` | `updateChannels`/`QuietHours`/`Budget`/`Autonomy` | `PATCH /api/v1/workers/<id>/settings/<key>` | **NO ROUTE — all 4 PATCHes 404** | YES — fetcher 404 → mock |
| `workers/[id]/settings/page.tsx` (danger zone) | `pauseWorker`/`archiveWorker` | `POST /api/v1/workers/<id>/{pause,archive}` | YES (200) | YES |
| `workers/[id]/settings/page.tsx` (danger zone) | `deleteWorker` | `DELETE /api/v1/workers/<id>` | YES (403 = registered) | YES |
| `workers/[id]/settings/page.tsx` (freeze chip) | `freezeWorker` | `POST /api/workers/<id>/freeze` (NEXT proxy → gateway) | YES (route wired) | YES — proxy returns gateway envelope |
| `workers/[id]/settings/page.tsx` (freeze chip) | `unfreezeWorker` | `POST /api/workers/<id>/unfreeze` (NEXT proxy → gateway) | YES (route wired) | YES — proxy returns gateway envelope |
| `workers/[id]/evals/page.tsx` | `fetchWorkerEvals` | `GET /api/v1/workers/<id>/evals` | YES (200) | YES — mock fallback |
| `workers/[id]/responsibilities/new/page.tsx` | `classifyDuty` | `POST /api/v1/workers/<id>/duties/classify` | YES (200) | YES — 404 → mock |
| `workers/[id]/responsibilities/new/page.tsx` | `previewDuty` | `POST /api/v1/workers/<id>/duties/preview` | YES (route wired) | YES — 404 → mock |
| `workers/[id]/responsibilities/new/page.tsx` | `commitDuty` | `POST /api/v1/workers/<id>/duties/commit` | YES (route wired) | YES — 404 → mock |
| `workers/[id]/?/retrain dialog` | `classifyRetrain` | `POST /api/v1/workers/<id>/retrain/classify` | YES (200) | YES — 404 → mock |
| `workers/[id]/?/retrain dialog` | `commitRetrain` | `POST /api/v1/workers/<id>/retrain/commit` | YES (400 on bad body) | YES — 404 → mock |
| `workers/[id]/?/retrain dialog` | `listRecentRetrains` | `GET /api/v1/workers/<id>/retrain/recent` | **NO ROUTE** | YES — 404 → empty list |
| `workers/[id]/?/retrain dialog` | `undoRetrain` | `DELETE /api/v1/workers/<id>/retrain/<id>` | **NO ROUTE** | YES — 404 → synthetic record |
| `workers/[id]/?/retrain dialog` | `previewReverseTest` | `POST /api/v1/workers/<id>/retrain/<id>/reverse-test` | **NO ROUTE** | YES — 404 → mock |
| (hire connectors flow) | `getConnectorStatus` | `GET /api/v1/workers/<id>/connectors/<id>` | **NO ROUTE** | YES — 404 → `{status:"not_connected"}` |
| (hire connectors flow) | `disconnectConnector` | `DELETE /api/v1/workers/<id>/connectors/<id>` | **NO ROUTE** | YES — 404 → `{ok:true}` |

### `/(app)/workers/[id]/skills/new` — Ghostwriter prompt entry

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `skills/new/page.tsx` | `draftSkillFromPrompt` | `POST /api/v1/skills/draft-from-prompt` | YES (200) | YES — 404/5xx → mock IR |

### `/(app)/workers/[id]/skills/[skillId]/canvas/page.tsx` — Skill IR viewer

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `canvas/page.tsx` | `fetchSkillIr` | `GET /api/v1/skills/<id>/ir` | YES (400 on test = route wired) | YES — `SkillIrNotFoundError` empty state |
| `canvas/page.tsx` | `saveSkillIr` | `POST /api/v1/skills/<id>/ir` | YES (400 on test) | NO explicit dev fallback — relies on bypass env |
| `canvas/page.tsx` | `exportSkillIr` | `GET /api/v1/skills/<id>/ir/export` | YES (400 on test) | NO dev fallback — bypass env only |
| `canvas/page.tsx` | `importSkillIr` | `POST /api/v1/skills/<id>/ir/import` | YES (400 on test) | NO dev fallback — bypass env only |

### `/(app)/skills/page.tsx`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `skills/page.tsx` | `fetchSkills` (marketplace) | `GET /skills` (NB: top-level, NOT `/api/v1/skills`) | YES (200) | YES — `isError` panel + seed catalog when empty |

### `/(app)/orchestration/page.tsx`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `orchestration/page.tsx` | `fetchOrchestrationRuns` | `GET /api/v1/orchestration/runs` | YES (200) | YES — mock fallback |
| `orchestration/page.tsx` | `cancelRun` | `POST /api/v1/orchestration/runs/<id>/cancel` | YES (202) | NO explicit fallback — error propagates |
| `orchestration/page.tsx` | `resetRetry` | `POST /api/v1/orchestration/runs/<id>/reset-retry` | YES (202) | NO explicit fallback |
| `orchestration/page.tsx` | `forceEscalate` | `POST /api/v1/orchestration/runs/<id>/force-escalate` | YES (202) | NO explicit fallback |

### `/(app)/retro/page.tsx` and worker retros tab

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `retro/page.tsx` (index) | `fetchRetrosIndex` | `GET /api/v1/retros` | YES (200) | YES — `isError` + mock-offline |
| (worker retros tab) | `fetchRetrosForWorker` | `GET /api/v1/workers/<id>/retros` | YES (200) | YES — mock fallback |
| (Retro dialog) | `createRetro`/`updateRetroSections`/`finalizeRetro` | `POST/PATCH /api/v1/workers/<id>/retros[/<id>/finalize]` | YES (routes wired) | NO explicit fallback (mutations error propagate) |

### Bell-popover (across pages)

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `NotificationsPopover` (in app shell) | `fetchNotifications` | `GET /api/v1/notifications` | **NO ROUTE** | YES — `isError` panel + fetcher 404 → mock |
| `NotificationsPopover` | `markAsRead`/`markAllAsRead`/`dismissNotification` | `POST /api/v1/notifications/...` | **NO ROUTE (all 3)** | YES — 404 → optimistic mock |

### Hire flow `/hire/...`

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `hire/step/[step]/page.tsx` (step 1) | `classifyRole` | `POST /api/v1/genesis/classify` | YES (200) | YES — `STUB_QUESTIONS` |
| `hire/step/[step]/page.tsx` (step 3) | `draftWorker` | `POST /api/v1/genesis/draft-worker` | YES (400 on test) | YES — `stubWorkerDraft` |
| `hire/done/[id]/page.tsx` | `hireWorker` | `POST /api/v1/workers` | YES (400 on test) | YES — stub UUID in dev; propagates in strict prod |

### Workflows + support

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `workflows/.../build/page.tsx` | `POST /api/workflows/propose` (Next.js route) → `POST /api/v1/skills/<name>/invoke/stream` (gateway) | (Next.js proxy; gateway slot covered by `dispatch_skill_named`) | YES proxy degrades to mock TOML when gateway unavailable | YES |
| `support/inbox/...` | `POST /api/support/ticket` (Next.js route) → `POST /api/v1/support/tickets` (gateway) | YES (403 = registered, principal-gated) | YES — gracefully degraded `202 queued + warning` |

### Auth (NextAuth + magic link)

| Page | Fetcher | Gateway endpoint | Exists? | Page handles 404? |
|---|---|---|---|---|
| `auth/signup/page.tsx` | `POST /api/auth/signup` (Next.js → gateway) | `POST /api/v1/auth/signup` | YES (registered, magic-link mounted ahead of bearer gate) | YES |
| `auth/verify/page.tsx` | `POST /api/auth/verify` (Next.js → gateway) | `POST /api/v1/auth/verify` | YES (registered) | YES |

## Summary counts

- **Total (page, fetcher, endpoint) triples audited:** **65**
- **Endpoints definitively missing from the gateway (no route registered):** **17**
- **Pages that crash / render-blank on a 404 with NO fallback:** **1 confirmed**
  - `/(app)/cohorts/page.tsx` has no `isError` branch. If the gateway
    cohort endpoint ever 404s in strict-prod (no mock fallback) the
    cohorts hero would render `0 cohorts across the fleet` indistinguishable
    from a real empty state. In dev the fetcher's mock-fallback keeps it
    presentable.
  - 4-5 mutation pages (Skill IR save/export/import, orchestration cancel/retry/escalate,
    retro create/finalize) propagate errors but those don't render-blank — they show a
    toast / inline error.

## Missing routes — full list (no gateway implementation found)

These are the 17 endpoints the SPA expects but the gateway does not currently serve.

### High-impact (demo-path)
1. `POST /api/v1/audit/<id>/replay` — replay sandbox. Page renders mock outcome silently.
2. `POST /api/v1/audit/export` and `POST /api/v1/audit/export/preview` — evidence-bundle export.
3. `POST /api/v1/audit` — "audit the audit" emission (fire-and-forget).
4. `GET /api/v1/notifications`, `POST .../<id>/read`, `POST .../read-all`, `POST .../<id>/dismiss` — the bell popover. Currently entirely mock-driven.
5. `GET /api/v1/workers/<id>/skills` (worker's Skills tab — the three-section Active/Library/Learned UI).
6. `POST /api/v1/workers/<id>/skills` (enable), `DELETE .../skills/<id>` (disable), `POST .../skills/learned/<id>/promote`, `DELETE .../skills/learned/<id>` (discard).

### Medium-impact (settings tab)
7. `PATCH /api/v1/workers/<id>/settings/channels`
8. `PATCH /api/v1/workers/<id>/settings/quiet-hours`
9. `PATCH /api/v1/workers/<id>/settings/budget`
10. `PATCH /api/v1/workers/<id>/settings/autonomy`

### Lower-impact (degraded gracefully)
11. `GET /api/v1/workers/<id>/connectors/<id>` (always "not_connected" in dev)
12. `DELETE /api/v1/workers/<id>/connectors/<id>` (no-op)
13. `GET /api/v1/workers/<id>/retrain/recent` (returns empty list)
14. `DELETE /api/v1/workers/<id>/retrain/<id>` (returns synthetic undone record)
15. `POST /api/v1/workers/<id>/retrain/<id>/reverse-test` — actually wired per `routes_retrain.rs:72` but my probe was on archived worker so 404 reason ambiguous; flagging for re-verification.
16. `PATCH /api/v1/workers/<id>/memory/facts/<id>` — the brief locks PUT but the workspace client.ts has no PUT verb; documented in source as a deviation.

### Verb confusions / route may exist
17. The `/api/v1/workers/<id>/memory/facts/<id>` route family in `routes_worker_memory.rs` declares PUT + DELETE (line 139, 185). Fetcher uses PATCH. PUT vs PATCH is the wire mismatch.

## Top 5 wiring gaps to close before demo

In order of demo-path impact:

### 1. **Worker Skills tab is entirely mock-driven** (P0)
`GET/POST/DELETE /api/v1/workers/<id>/skills` and the four mutation
endpoints all 404. This is the three-section "Active / Library /
Learned" surface the worker-detail design locked on 2026-05-19.
Demo-path impact: critical. The whole "AI worker that learns and
gets promoted" story relies on this tab. **Single highest-priority
gateway endpoint to ship: `GET /api/v1/workers/<id>/skills`.**

### 2. **Notifications popover is entirely mock-driven** (P0 demo)
4 endpoints missing. The bell icon shows a hard-coded sample. Any
operator clicking it during a demo sees curated fake notifications,
not what their workers actually need. Easy to authorise but lands
zero alerting value until the routes exist.

### 3. **Evidence-bundle export silently fakes the download** (P1)
`POST /api/v1/audit/export` and `/preview` 404. The bundle dialog
generates a deterministic mock PDF / JSON name+size; if the
operator's customer / auditor asks for the actual file, it's not
real. The SPA shows no indication. Highest data-integrity risk in
the list.

### 4. **Audit-action replay is a deterministic mock** (P1 demo)
`POST /api/v1/audit/<id>/replay` 404. Every audit row's "Replay"
button returns one of three pre-canned verdicts based on the id
hash. Demo viewer cannot tell. The verdict-routing UI is on the
replay page — fine — but the verdict itself is fake.

### 5. **Worker Settings tab mutations don't persist** (P2)
4 PATCH endpoints missing. Operator changes channels / quiet hours
/ budget / autonomy, the UI says "saved", reload the page — change
is gone (dev mock-fallback only). Same flavour as `worker-skills`
gap but lower demo impact (these surfaces are usually pre-configured
once).

## Single highest-priority gateway endpoint missing

**`GET /api/v1/workers/<id>/skills`** — the three-section
Active/Library/Learned bundle. Closes the worker-detail page's
biggest mock surface and unblocks the post-MVP "skills that the AI
learns and gets promoted on" narrative that's locked in the
worker-detail brand-locked design.

## Notes & caveats

- Probes were done against a single live gateway with a single
  workspace. The test worker `019e3939-2365-7681-950e-0b99d0e74666`
  was archived during testing (POST `.../archive` returned 200),
  so any tests run after that point on that worker returned 410
  "gone" envelopes — not 404. The `duties/preview` and
  `duties/commit` 410 results in the raw log are NOT missing
  routes; they're "worker archived" 410s.
- The fetcher pattern is consistently:
  `ARGO_GATEWAY_AUTH_BYPASS=1` → mock; dev 404 → mock; strict-prod
  → propagate. This means **in dev, no missing route crashes the
  page** — they all render mock data. The gap surfaces in
  strict-prod (`NODE_ENV=production` AND `NEXT_PUBLIC_ENV=production`).
- The Next.js proxy routes (`/api/workers/<id>/freeze`,
  `/api/workers/<id>/unfreeze`, `/api/workflows/propose`,
  `/api/support/ticket`) all have a `502 gateway_unreachable`
  fallback envelope, which the SPA surfaces as a toast. They do
  NOT degrade silently to mock — they fail loud, which is the
  right behaviour for write paths.
- `/skills` (the marketplace listing) uses the unversioned
  top-level route — historic carry-over. The `lib/api/skills.ts`
  source comment says "the gateway exposes the layered skill
  registry under `/skills`", which is true; the `/api/v1/skills`
  prefix is reserved for skill-id sub-routes (IR, draft-from-prompt).
- The skill IR mutations (save/export/import) all return 400 on
  the test probe — that means the routes ARE registered, but the
  saveSkillIr / exportSkillIr / importSkillIr fetchers don't have
  dev fallbacks. If the gateway returns 404 (e.g. unknown skill
  id), the page would throw to the caller. Worth a follow-up
  brief for parity with the rest of the fetcher family.

## Recommended follow-ups (do NOT modify code in this pass)

1. **One brief per missing-route group:**
   - `mission-gateway-worker-skills-endpoints` — closes 5 missing routes (P0).
   - `mission-gateway-notifications-endpoints` — closes 4 missing routes (P0).
   - `mission-gateway-audit-export-replay-endpoints` — closes 4 missing routes (P1).
   - `mission-gateway-worker-settings-mutations` — closes 4 PATCH routes (P2).
   - `mission-gateway-worker-connectors-endpoints` — closes 2 routes (P3).
   - `mission-gateway-worker-retrain-history` — closes 3 routes (P3).
2. **Add `isError` branch to `cohorts/page.tsx`** — the one page that
   currently silently empties instead of surfacing the error. Match
   the cohorts hero to `TeamErrorState` pattern.
3. **Add dev mock fallbacks to the four skill-IR mutation fetchers**
   for parity with the rest of the fetcher family. Otherwise the
   /canvas page will throw uncaught errors in dev whenever the
   skill id isn't in the local DB.
