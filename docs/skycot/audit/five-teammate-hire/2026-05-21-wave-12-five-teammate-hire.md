# Wave-12 Audit — Five-Teammate Hire Journey

- Auditor: Wave-12 audit agent #1
- Date: 2026-05-21
- Method: read-only static analysis of `~/code/argo/web/app/(hire)/hire/`, the supporting `web/lib/hire/*`, `web/lib/api/{genesis,workers,team,audit}.ts`, the gateway routes `crates/argo-cli/src/handlers/gateway/routes_{genesis,workers_hire,team_list,audit_feed}.rs`, and the worker / team / audit page surfaces under `web/app/(app)/{team,workers,audit}`. No runtime simulation; behavior inferred from code paths.
- Verdict (one line): the wizard plumbing supports 5 sequential hires cleanly, but persona distinctness collapses to "Mariah" when the gateway is offline (stub path) and three of the five demo names (Sam, Nori, plus any non-Mariah/Priya/Jamal) get the generic `DEFAULT_PERSONALITY` on the worker-detail page.

## Step plumbing — load-bearing files

| Route | File |
| --- | --- |
| Step 1 (Land) | `~/code/argo/web/app/(hire)/hire/page.tsx` + `web/components/skycot/hire/Step1Land.tsx` |
| Step 2..5 (Listen → Connect) | `~/code/argo/web/app/(hire)/hire/step/[step]/page.tsx` + `web/components/skycot/hire/Step{2..5}*.tsx` |
| Step 6 (Done / Celebrate) | `~/code/argo/web/app/(hire)/hire/done/[workerId]/page.tsx` |
| Draft state | `web/lib/hire/{use-hire-draft,draft-store,draft-types}.ts` |
| Genesis (classify + draft-worker) | `web/lib/api/genesis.ts` + `web/lib/hire/genesis-stub.ts` |
| Hire commit | `web/lib/api/workers.ts` (`hireWorker`) + gateway `routes_workers_hire.rs` |
| Team grid | `web/app/(app)/team/page.tsx` + `web/lib/api/team.ts` + `web/lib/sample-team.ts` |
| Worker detail | `web/app/(app)/workers/[workerId]/**` + `web/lib/sample-workers-detail.ts` |
| Audit feed | `web/app/(app)/audit/page.tsx` + `web/lib/api/audit.ts` + gateway `routes_audit_feed.rs` |

## Per-hire walk-through

### Hire 1 — sales SDR ("Priya screens leads + books interviews")

1. Wizard Step 1 — unique description: PASS. `Step1Land` accepts any free text ≥15 chars (`STEP1_MIN_CHARS` in `web/lib/hire/examples.ts`). The "sales-sdr" example pre-fill exists at `findExampleRoleById("sales-sdr")` for deep-links, but a typed description is also accepted.
2. Genesis call — distinct persona:
   - Live gateway path (`routes_genesis.rs::draft_worker_response`): the persona name comes from the operator's Step-2 answer for the question `id: "name"` (see `canonical_questions()` line 78). If the operator types "Priya", the response carries `suggested_name: "Priya"` and a `suggested_vertical` derived from `classify_charter()`. Distinct.
   - Stub path (gateway 404 → `web/lib/hire/genesis-stub.ts::stubWorkerDraft`): the stub ALWAYS returns `suggestedName: "Mariah"`, `suggestedVertical: "marketplace-seller"`, with marketplace-flavored example replies ("Is this still available?"). Not distinct. This is the v1 default in dev because `mission-gateway-genesis-classify-endpoint` / `mission-gateway-genesis-draft-worker-endpoint` paths are gated behind a running gateway.
3. Worker detail page — unique avatar/initials/role: PARTIAL. Initials are derived from the name (`TeammateCard::initialsFor` → "PR" for Priya). The `WorkerPersonalityCard` (`web/components/skycot/worker/WorkerPersonalityCard.tsx`) has a hand-authored `PERSONALITIES["Priya"]` entry — distinct traits, escalation triggers, signature behaviors. Resolution depends on `fetchWorker(workerId)` finding the record; the stub-id branch (`stub-<uuid>`) is NOT in `SAMPLE_WORKERS_DETAIL`, so worker-detail throws `WorkerNotFoundError` in dev-no-gateway mode. Live gateway path persists the worker via `IdentityStore::create_worker_from_draft`.
4. Team grid: PASS once the worker row exists. `TeammateCard` reads name + tagline + vertical from the live team list.
5. Audit row: emitted at `routes_workers_hire.rs::emit_hire_audit` as an `AuditKind::HttpRequest` row with payload `{ endpoint: "workers.hire", persona_name: "Priya", worker_id, ... }`. The audit feed mapper (`routes_audit_feed.rs::action_type`) falls back from `payload.action_type` to `payload.endpoint` so the row surfaces with `actionType: "workers.hire"`. PARTIAL — see "Audit-row clarity" below.

Score: 6/10 live-gateway, 2/10 stub-fallback (collapses to Mariah).

### Hire 2 — support specialist ("Jamal: tickets")

1. Step 1 description: PASS. Maps to the `support-tier1` example.
2. Genesis: live → distinct ("Jamal", vertical = `customer-support` if classifier matches the keyword set in `argo-genesis::classify_charter`). Stub → "Mariah".
3. Worker detail: `PERSONALITIES["Jamal"]` exists — distinct trait set (`["Calm under pressure", "Empathic", "Quick", "Honest"]`). `SAMPLE_WORKERS_DETAIL["wkr_01J0JAMAL"]` exists but only matches the seeded id, not a freshly-hired stub-uuid.
4. Team grid: PASS.
5. Audit row: emitted with `persona_name: "Jamal"`; worker.name in the feed JSON falls back from `payload.worker_name` (NOT emitted) to `payload.worker_id` to literal `"unknown"`. The hire row will display the worker.id, not "Jamal". CLARITY ISSUE.

Score: 6/10 live, 2/10 stub.

### Hire 3 — marketplace lister ("Mariah")

1. Step 1 description: PASS. The `marketplace-seller` example is the canonical match.
2. Genesis: this is the ONE case where the stub matches the operator's intent — both paths return Mariah-shaped output. Live path may diverge if `classify_charter` returns a non-marketplace vertical for the user's specific wording.
3. Worker detail: `PERSONALITIES["Mariah"]` exists — distinct trait set.
4. Team grid: PASS.
5. Audit row: same fall-back chain as above.

Score: 8/10 live, 8/10 stub (the stub happens to be Mariah-shaped).

### Hire 4 — finance / AP ("Sam: invoices")

1. Step 1 description: PASS. No exact example template; closest is `tax-prep-intake`. The free-text path is supported.
2. Genesis: live → distinct ("Sam", vertical likely `ops` or `unknown`). Stub → "Mariah" (wrong).
3. Worker detail: `PERSONALITIES["Sam"]` is NOT in the map. Sam gets `DEFAULT_PERSONALITY` (`["Reliable", "Curious", "Helpful"]`) — same as any other unmapped name. The personality card's "great-at / escalate-when / signature behaviors" copy is generic. `TRAITS_BY_NAME["Sam"]` IS present in `TeammateCard.tsx` (line 55: `["Vigilant", "Steady", "Methodical"]`) — so the team-grid card looks distinct, but the worker-detail page does not.
4. Team grid: PASS.
5. Audit row: as above.

Score: 5/10 live (worker-detail personality is generic), 1/10 stub.

### Hire 5 — marketing ("Nori: posts/CRM updates")

1. Step 1 description: PASS.
2. Genesis: live → distinct ("Nori", vertical `marketing`). Stub → "Mariah".
3. Worker detail: `PERSONALITIES["Nori"]` is NOT in the map. Same `DEFAULT_PERSONALITY` blob as Sam. `TRAITS_BY_NAME["Nori"]` is present (`["Creative", "On-brand", "Concise", "Curious"]`) — team grid distinct; worker-detail not.
4. Team grid: PASS.
5. Audit row: as above.

Score: 5/10 live, 1/10 stub.

## Cross-hire concerns

### A. Backend ↔ frontend `WorkerDraft` shape mismatch (P1)

`routes_genesis.rs::WorkerDraft` (lines 42-52) serializes `persona: String` (a slug from `name_to_persona_slug`). `web/lib/hire/draft-types.ts::WorkerDraft.persona` expects `{ tagline, avatarKind, voiceProfile }`. The frontend's `isWorkerDraft` validator (`web/lib/api/genesis.ts::isPersona` line 135) REJECTS the gateway response — the response will be classified as malformed → falls back to `stubWorkerDraft()` → returns "Mariah" every time. This means the live-gateway path effectively never serves distinct personas to the wizard; every hire collapses to the Mariah stub regardless of the operator's input. The schema mismatch is the load-bearing bug.

### B. Stub-id worker resolution (P1)

`hireWorker()` falls back to `{ workerId: "stub-<uuid>", success: true }` in dev when the gateway 404s (`web/lib/api/workers.ts:147-154,254-258`). The Step-5 → done route then navigates to `/workers/stub-<uuid>/overview`. `fetchWorker` 404s → `findSampleWorker("stub-<uuid>")` returns undefined → `WorkerNotFoundError` → the worker-detail page renders `WorkerNotFound`. The user just hired a teammate and the link to their profile breaks immediately.

### C. Celebration page says "first teammate" on hire 5

`~/code/argo/web/app/(hire)/hire/done/[workerId]/page.tsx:93-95` hardcodes the hero copy: `"You just hired your first teammate."`. There is no branch on team size. Hires 2 through 5 will see the same "first teammate" celebration. Low-cost fix; high-impact polish miss.

### D. Audit-row clarity (P2)

`routes_workers_hire.rs::emit_hire_audit` (line 265) emits payload keys `{ endpoint: "workers.hire", persona_name, worker_id, ... }`. The audit-feed mapper (`routes_audit_feed.rs::audit_row_json` line 225) reads `worker_name` (not `persona_name`) and `verb` (not emitted by the hire path; falls back to `row.kind = "http.request"`). Net result of 5 hires in the audit feed: 5 rows with `actionType: "workers.hire"`, `verb: "http.request"`, `worker.name: "<worker-id-uuid>"`. Distinguishable as separate events; unreadable as 5 named-teammate hires. The fix is two payload-key renames (`persona_name` → `worker_name`, add `verb: "hired"`) at the emit site.

### E. PERSONALITIES coverage gap

`web/components/skycot/worker/WorkerPersonalityCard.tsx:35-96` only contains entries for Mariah / Priya / Jamal. Nori, Sam, and Rio (and any future worker) get the generic `DEFAULT_PERSONALITY` (line 98-111). For the 5-teammate journey, 2 of 5 (Sam, Nori) hit the fallback. The fix is additive — fill in PERSONALITIES for Nori and Sam (the `SAMPLE_WORKERS_DETAIL` set already provides their archetypes).

### F. /team grid post-5

The grid surface itself handles N workers cleanly. `useTeam` calls `GET /api/v1/team`; the gateway `routes_team_list` enforces `MAX_WORKERS=200` and emits a `total_count` field with truncation signaling. 5 hires is well within bounds. Sort/filter chips work. The hero strip (`TeamHero` in `web/app/(app)/team/page.tsx:421-462`) reads `total === 0 | 1 | else` so the "5 teammates, one team." headline activates correctly past hire 2. PASS.

### G. Draft-store lifecycle between hires

`web/lib/hire/use-hire-draft.ts` writes to one localStorage key (`HIRE_DRAFT_STORAGE_KEY = "skycot.hireDraft"`). The done page calls `draft.clear()` (line 60 of done page) before the 1.5s redirect, so the next hire starts from a fresh `step: 1` state. The 7-day TTL guards stale drafts. Sequential 5-hire flow is safe. PASS.

### H. Audit page mock-fallback masks live hires

In dev (no gateway), `web/lib/api/audit.ts::fetchAuditFeed` falls back to `SAMPLE_AUDIT_ROWS` (`web/lib/sample-audit.ts`) — 50+ pre-seeded action rows for Priya / Sam / Mariah / Jamal / Nori / Rio. Zero of those rows are `worker_hired`-flavored. The audit page in dev-no-gateway will show no evidence of any of the 5 hires the operator just performed. Live gateway path emits the rows correctly (modulo clarity issue D). For the demo MP4 path the operator needs the gateway running.

## Are the 5 distinct in seed data?

Yes, in `SAMPLE_TEAM` (`web/lib/sample-team.ts:29-94`) and `SAMPLE_WORKERS_DETAIL` (`web/lib/sample-workers-detail.ts:30-118`) — all five (Mariah, Priya, Jamal, Nori, Sam) have distinct names, taglines, verticals, departments, teams, kpiToday, headlines, status, last-active. `TRAITS_BY_NAME` (`TeammateCard.tsx:50-56`) covers all five with distinct trait lists. The personality card (`WorkerPersonalityCard.tsx`) covers only three (Mariah, Priya, Jamal); Nori and Sam fall through to `DEFAULT_PERSONALITY`.

## /team grid post-5

PASS at the grid layer. Hero hits the "N teammates, one team." branch. KPI strip computes `workers.length * 187` actions/wk and `workers.length * 12` hours-saved — derived multipliers, not real counts (acceptable v1 placeholder). Status filter, search, chip rail all functional for N=5.

## /audit row clarity

PARTIAL — fails the spirit of the test even though 5 separate events ARE emitted. The 5 rows are technically distinguishable (5 distinct `id`s, 5 distinct `worker_id`s, 5 distinct timestamps). They are practically unreadable because: (a) `actionType` collapses to `"workers.hire"` for all 5 (no per-hire differentiation), (b) `verb` is `"http.request"` not "hired Priya", (c) `worker.name` is the UUID. Fix is a 3-line payload patch in `emit_hire_audit`.

## Per-hire score summary

| Hire | Live-gateway score | Stub-fallback score | Notes |
| --- | --- | --- | --- |
| 1 — Priya (sales) | 6/10 | 2/10 | PERSONALITIES OK; collapses to Mariah on stub |
| 2 — Jamal (support) | 6/10 | 2/10 | PERSONALITIES OK; audit clarity issue |
| 3 — Mariah (marketplace) | 8/10 | 8/10 | Best-case; stub happens to match |
| 4 — Sam (finance/AP) | 5/10 | 1/10 | DEFAULT_PERSONALITY on detail page |
| 5 — Nori (marketing) | 5/10 | 1/10 | DEFAULT_PERSONALITY on detail page |

Live-gateway weighted score: ~6/10. Stub (dev-no-gateway) score: ~3/10 average — collapse to Mariah is the dominant failure mode.

## Recommended follow-ups (briefs to author)

1. **`mission-genesis-draft-worker-shape-align`** (P1) — the backend `WorkerDraft.persona: String` vs frontend `WorkerDraft.persona: { tagline, avatarKind, voiceProfile }` mismatch means the live-gateway path never returns to the wizard. Either reshape the gateway response (add tagline/avatarKind/voiceProfile fields populated from the manifest) or loosen the frontend validator to accept string persona slugs and synthesize a tagline client-side.
2. **`mission-hire-stub-id-worker-resolution`** (P1) — when `hireWorker` returns a `stub-<uuid>`, either (a) write the worker to a session-scoped in-memory store so `/workers/stub-<uuid>` can render it, or (b) reroute the celebration page to `/team` instead of the worker profile when the id is stub-shaped.
3. **`mission-hire-celebration-copy-not-first`** (P3) — branch the done-page hero copy on team-size: "first teammate" for 1, "second teammate" or "another teammate" thereafter.
4. **`mission-audit-hire-row-readable`** (P2) — in `emit_hire_audit` rename `persona_name` → `worker_name`, add `verb: format!("hired {}", suggested_name)`, add `action_type: "worker_hired"`. Three field changes; clarity goes from unreadable to plain-English.
5. **`mission-worker-detail-personality-coverage`** (P3) — fill in `PERSONALITIES["Nori"]`, `PERSONALITIES["Sam"]`, `PERSONALITIES["Rio"]` so the worker-detail card matches the visual distinctness already present in the team grid trait chips.

## Acceptance against the brief

| Audit question | Result |
| --- | --- |
| Wizard accepts unique description per hire? | YES — `Step1Land` textarea has no template-id lock; all 5 free-text inputs accepted. |
| Genesis call generates distinct persona (traits, skills)? | LIVE: yes if the schema mismatch is fixed. STUB: no — always Mariah. |
| `/workers/[new-id]` resolves with unique avatar/initials/role? | LIVE: yes (after a worker row commits via `IdentityStore`). DEV-STUB: no — stub-id returns `WorkerNotFoundError`. |
| `/team` grid shows all 5 after? | YES at the surface layer. Depends on the gateway list endpoint returning the committed workers. |
| `/audit` reflects 5 separate `worker_hired` events? | TECHNICALLY YES (5 rows), CLARITY NO — they appear as `actionType: workers.hire`, `verb: http.request`, `worker.name: <uuid>`. Needs payload-key rename. |
