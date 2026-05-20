# Wave-15 Audit #2 — Worker-Detail Tabs: Memory + Trust Ratchet + Freeze Flow

**Date:** 2026-05-21
**Auditor:** Wave-15 audit agent #2 (read-only)
**Scope:** `/workers/[id]/{memory,skills,trust,escalation,settings/policies,activity,overview,responsibilities}` plus the freeze button at the layout level.
**Mode:** Read-only review of `web/` (SPA) and `crates/argo-cli/src/handlers/gateway/` (gateway endpoints), cross-referenced against wave-1 + wave-8 prior findings.

---

## TL;DR verdict

**Overall worker-detail score: 6.5 / 10.**

The worker-detail surface is structurally complete (every tab has a real page, every page has typed hooks against a query-cache, every primary mutation has an optimistic-update path). The Memory tab is the strongest — its full CRUD path lights up at both ends (gateway endpoints emit dedicated audit rows; SPA cache is updated optimistically with rollback). Trust grade is read-only and grown-up. Escalation tab is brand-new (#1904) but is **client-side mock data only** — no gateway endpoint behind the trust-over-time chart or the escalation history table.

**The two biggest cracks:**

1. **There is no per-worker `/skills` gateway endpoint at all.** The SPA's `worker-skills.ts` reads from `/api/v1/workers/<id>/skills` and POSTs to `/api/v1/workers/<id>/skills`, `…/skills/learned/<id>/promote`, etc., but **none of those routes exist in `crates/argo-cli/src/handlers/gateway/`** (no `routes_worker_skills.rs` file). Every dev request 404s and falls through to in-memory mocks that vanish on hard refresh. The "shipped via #1682" skill-library worker-load is the *runtime* hook, not the *UI* read/write surface.
2. **Wave-8 freeze-propagation gap is unaddressed.** PR #1878 (the FreezeBanner work) shipped the banner on every worker-profile tab — that single surface is correct and consistent. But the cross-surface propagation Wave-8 flagged (Approvals inbox, Team grid, Cohorts, Notifications bell, Audit feed, hero status pill) is **still broken on today's main**. `WorkerStatus` (`web/lib/types/worker.ts:15-20`) still has no `"frozen"` variant. `ApprovalCard.tsx` still grep-clean of "frozen". `sample-audit.ts` / `types/audit.ts` still have no `worker_frozen` / `worker_unfrozen` / `worker.memory.fact.created` / `worker.policy_changed` action kinds — the audit rows the gateway DOES emit are silently dropped at the SPA's audit feed.

---

## Per-tab score

| Tab | Score | UI handlers wired? | Gateway endpoint? | Optimistic + rollback? | Audit row emitted? |
| --- | ----- | ------------------ | ----------------- | ---------------------- | ------------------ |
| **Memory** | 8.5/10 | Yes (Add/Edit/Delete via `useWorkerMemoryActions`) | Yes (GET/POST/PUT/DELETE in `routes_worker_memory.rs`) | Yes (optimistic insert + rollback on error in `use-worker-memory.ts:47-93`) | Yes (`worker.memory.fact.{created,edited,deleted}` via `emit_audit`) |
| **Skills** | 4/10 | Yes (Disable / Try / Promote / Discard via `useSkillsActions`) | **NO — no `routes_worker_skills.rs` file exists.** SPA always hits 404 → falls to in-memory mocks | Yes (optimistic-move via `applyOptimistic`) | **NO — mock-only path emits no audit row** |
| **Trust** | 7/10 (read-only by design) | N/A (no edit affordance — brief intentionally locked) | Yes (GET `/api/v1/workers/<id>/trust-grade`) | N/A | Yes on view (`emit_audit` in `routes_worker_trust_grade.rs:89`) |
| **Escalation (trust-chart)** | 5/10 | N/A (read-only) | **NO — `findSampleTrustHistory` is client-side mock; history table is hardcoded `SEEDS` in `escalation/page.tsx:64-173`** | N/A | N/A |
| **Settings → Policies** | 7/10 | Yes (Edit policy via `useUpdateWorkerPolicy`) | Yes (PATCH `/api/v1/workers/<id>/policies/<action>`) | Yes (optimistic via `setQueryData`) | Yes (`HttpRequest` + `endpoint: worker.policy_changed`) |
| **Activity** | 6/10 | N/A (read-only feed) | Yes (overview / audit feed) | N/A | Reads existing audit rows |

### Other tabs (not in primary scope)

| Tab | Score | Notes |
| --- | ----- | ----- |
| **Overview** | 7/10 | KPIs + headline; mock-fallback chain |
| **Responsibilities** | n/a | Only `/new` subroute exists; main page is `[tab]` fallback |
| **Hero action menu (Pause/Disable/Archive on profile)** | 2/10 | `handleHeroAction` in `WorkerLayoutShell.tsx:111-117` is a `console.warn` — actions land in "E3-b + E4-f" (still unshipped) |
| **Freeze/Unfreeze button** | 7/10 (own surface) / 2/10 (cross-surface) | Button + dialog + reason field + gateway emit are all correct; **everything else is still broken per Wave-8** |

---

## Persistence verdict

| Scenario | Persists across reload? | Notes |
| -------- | ----------------------- | ----- |
| Memory fact add/edit/delete (gateway up) | YES | `routes_worker_memory.rs` writes via `argo-memory` append-only store with soft-delete tombstone |
| Memory fact mutation (dev, no gateway) | **NO — module-scoped Map mutates in-tab; lost on hard refresh** | `mockAddFact` in `sample-worker-memory.ts:203-225` mutates `MEMORY_BY_ID` which re-evaluates on full page reload |
| Skills toggle (gateway up) | **N/A — endpoint doesn't exist** | Routes return 404 in all environments |
| Skills toggle (dev) | NO — in-tab only; lost on hard refresh (same module-Map pattern) | |
| Policy change (gateway up) | YES | `routes_worker_policies.rs:144` writes `.skycot/worker-policies/<worker_id>.toml` |
| Trust grade | YES (read-only — derived from `worker.trust_grade u8` in identity store) | No edit path by design |
| Freeze / unfreeze | YES | `worker_freeze` store call writes a freeze record; idempotent unfreeze |

---

## Audit-row emission table

| Action | Gateway emits AuditKind? | SPA audit feed shows it? |
| ------ | ------------------------ | ------------------------ |
| Memory fact create | `HttpRequest` + `worker.memory.fact.created` | **NO — kind not in `web/lib/types/audit.ts` taxonomy** |
| Memory fact edit | `HttpRequest` + `worker.memory.fact.edited` | NO (same reason) |
| Memory fact delete | `HttpRequest` + `worker.memory.fact.deleted` | NO (same reason) |
| Memory list (read) | `worker.memory.viewed` | NO |
| Skill toggle | None (no gateway endpoint exists) | N/A |
| Trust-grade view | Yes (per `emit_audit`) | NO |
| Policy change | `HttpRequest` + `worker.policy_changed` | NO |
| Worker freeze | `HttpRequest` + `worker.frozen` (success) / `RbacDeny` (denied) | NO |
| Worker unfreeze | `HttpRequest` + `worker.unfrozen` | NO |

**Implication: the gateway is faithfully emitting an audit row for every material worker-detail action, and the audit chain self-hashes them — but the operator never sees them in the `/audit` feed UI because the SPA's `action_type` taxonomy doesn't recognise the verbs.** This is the highest-impact finding in the audit. The receipts exist; the inbox can't read them.

---

## Freeze propagation status (Wave-8 follow-up)

Wave-8 (2026-05-21) flagged 8 surfaces; 1 correct, 7 not-handled. Today's status:

| Surface | Wave-8 verdict | Today (2026-05-21 late) | Change? |
| ------- | -------------- | ----------------------- | ------- |
| `/workers/[id]/*` layout | Correct | Correct (banner + button + every-tab visibility) | unchanged |
| `/team` grid (TeammateCard) | Not handled | Still not handled — `WorkerStatus` has no `"frozen"` variant | unchanged |
| `/cohorts` | Not handled | Still not handled | unchanged |
| `/audit` feed | Action type not modelled | Still not modelled — no `worker_frozen` in `lib/types/audit.ts` | unchanged |
| `/` Approvals inbox | **P1 — operator can approve a no-op on a frozen worker's pending action** | **Still broken — `ApprovalCard.tsx` grep-clean of "frozen"** | unchanged |
| `/workflows` | Not handled | Still not handled | unchanged |
| Notifications bell | No `worker-frozen` kind | Still no kind | unchanged |
| Worker hero status pill | Partial overlap (pill might show "Working" while banner says frozen) | Same — `WorkerHero` has no freeze-aware branch | unchanged |

**Verdict: zero progress on Wave-8 cross-surface freeze propagation in the intervening 24 hours.** The freeze flow remains a one-surface feature.

---

## Top 3 worker-detail gaps

### Gap 1 — Skills tab is a paper UI

**Severity: P1.**

The Skills tab (`/workers/[id]/skills`) is fully wired in the SPA — three sections, optimistic mutations, confirm modals, query-cache contracts. **But there is no `/api/v1/workers/<id>/skills` endpoint at the gateway** (no `routes_worker_skills.rs`, no entry in `routes_dispatch_get_*` or `routes_dispatch_post_*`). The disable/try/promote/discard actions appear to work for one browser-tab session via the in-memory mock Map, and disappear on hard refresh. A live demo to a CFO who toggles a skill, refreshes, and sees their change vanish is a credibility-erosion event.

**Fix shape:** Author `mission-gateway-worker-skills-endpoints` covering:
- `GET /api/v1/workers/<id>/skills` (active + library + learned for one worker)
- `POST /api/v1/workers/<id>/skills` (enable / try)
- `DELETE /api/v1/workers/<id>/skills/<skill_id>` (disable)
- `POST /api/v1/workers/<id>/skills/learned/<pattern_id>/promote`
- `DELETE /api/v1/workers/<id>/skills/learned/<pattern_id>` (discard)
- Each mutation emits a typed audit row (`SkillApproved` exists; new `WorkerSkillEnabled` / `WorkerSkillDisabled` / `LearnedPatternDiscarded` variants needed — or reuse `HttpRequest+endpoint`)

### Gap 2 — Audit-row taxonomy diverges between gateway and SPA

**Severity: P1 (this is a trust-receipts gap, not just a UX gap).**

The gateway writes typed audit rows for every material worker-detail action (memory fact CRUD, policy change, freeze, unfreeze, trust-grade view) and the audit-chain verifier self-hashes them. **The SPA's `/audit` feed reads from a sample-data file that never declares these action kinds.** The `action_type` taxonomy in `web/lib/types/audit.ts` is the source of truth for what the audit-feed UI knows how to render; if a kind isn't there, it's silently dropped.

This means the gateway's marketing claim that "every action is audit-logged" is technically true — but the operator's audit-feed UI cannot see ~80% of the worker-detail events. The Memory tab's "Reason recorded in audit" copy on the Add Fact dialog is honest about the database; dishonest about the UI.

**Fix shape:** Author `mission-audit-taxonomy-worker-detail-kinds` covering:
- Add to `web/lib/types/audit.ts`: `worker.memory.fact.created`, `worker.memory.fact.edited`, `worker.memory.fact.deleted`, `worker.policy_changed`, `worker.frozen`, `worker.unfrozen`, plus their renderers in `/audit` feed.
- Cross-check against `crates/argo-audit/src/event.rs::AuditKind` so the SPA taxonomy is the projection of the gateway's, not a parallel surface.
- Add `worker_frozen` / `worker_unfrozen` rows to `sample-audit.ts` so the demo renders them under "Recent Activity".

### Gap 3 — Trust-grade is a static observatory, not a ratchet

**Severity: P2.**

The wave-15 brief hints at a "trust ratchet promise" — the idea that as a worker proves itself, its trust grade ratchets up, and the operator sees that movement. Today:

- The trust grade is **observable** (`/workers/[id]/trust` shows the current rung + criteria for the next rung).
- The trust-grade history is **mocked client-side** (`escalation/page.tsx` uses `findSampleTrustHistory` from `lib/sample-worker-trust-history.ts`; the gateway has no `/api/v1/workers/<id>/trust-history` endpoint).
- The trust grade itself is **not derived** from observable signals — it's a `u8` field on the `worker` row in the identity store with no automatic promotion logic. The trust-grade response includes `next_grade_criteria` (current_value/target_value/met) but no scheduler/evaluator promotes the grade when criteria are met. The disclaimer text at `trust/page.tsx:54-60` even admits this: "Trust grade is derived from observed behavior, not operator-tunable. The criteria above describe what would unlock the next rung once the worker meets them" — but nothing actually unlocks it.
- There is no audit-row emission on trust-grade *change* (`routes_worker_trust_grade.rs` emits a `viewed` row, not a `changed` row, because the grade never changes from this endpoint).

So the "trust ratchet" is currently a marketing promise without a turning mechanism. The operator sees a static grade and a chart of mock history.

**Fix shape:** Author `mission-trust-ratchet-promotion-engine` covering:
- A scheduler that periodically evaluates `next_grade_criteria` against real signals and promotes/demotes the worker's `trust_grade` field when all criteria are met / a regression is detected.
- A new `WorkerTrustGradeChanged` audit kind with payload `{worker_id, old_grade, new_grade, signals_snapshot}`.
- A `/api/v1/workers/<id>/trust-history` endpoint reading from the audit chain instead of mock data, so the escalation tab's chart reflects reality.
- A notification kind `trust-grade-promoted` so the operator gets a celebratory bell-ping when their worker earns a rung.

---

## Honourable mentions (smaller gaps not in the top 3)

- **Hero action menu still no-ops.** `WorkerLayoutShell.tsx:111-117` is a `console.warn` for Pause/Disable/Archive actions on the worker hero — pending "E3-b + E4-f" briefs that have not landed. Settings-tab "Danger zone" does carry real `onPause` / `onArchive` / `onDelete` handlers, so the menu is duplicate UX, not the only path — but it's still a broken affordance on the page chrome.
- **Memory mock fallback is module-scoped in-tab persistence.** `mockAddFact` mutates the `MEMORY_BY_ID` module-level object, so within one open tab the CRUD round-trips correctly; full reload re-evaluates the module and discards the mutation. Acceptable for dev mode if expectations are set; misleading otherwise. The same shape applies to skills mutations.
- **Trust criteria are not real.** `routes_worker_trust_grade.rs:42-46` notes the brief asked for `action_count`, `success_rate`, `refused_count`, `time_since_first_action` — the implementation surfaces `trust_grade_value`, `age_days`, `last_active_days_ago`, `has_been_used`. Those are observable but they are the wrong signals — they describe the worker, not the worker's *behaviour*. A worker that has been around 30 days with zero successful actions and three policy refusals would rank identically with a worker that has 1000 successful actions and zero refusals.
- **Frozen does not block edits to memory facts / policies / settings on the worker-profile page.** The freeze record blocks new sessions at the runtime, but the SPA does not disable the Add Fact button or the Save Policy button when `freezeState !== null`. An operator can edit a frozen worker's memory and policy; whether the gateway enforces "frozen workers are read-only at the API" was not verified (path not exercised in this audit).

---

## Reference: file locations

- Pages: `~/code/argo/web/app/(app)/workers/[workerId]/{memory,skills,trust,escalation,settings}/page.tsx`
- Hero shell: `~/code/argo/web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx`
- Freeze surface: `~/code/argo/web/app/(app)/workers/[workerId]/{FreezeButton,FreezeBanner,fetchFreezeState}.{tsx,ts}`
- SPA hooks: `~/code/argo/web/lib/hooks/use-worker-{memory,skills,trust,policies,settings}.ts`
- SPA API clients: `~/code/argo/web/lib/api/worker-{memory,skills,trust,policies,settings}.ts` + `workers.ts`
- Proxy routes: `~/code/argo/web/app/api/workers/[id]/{freeze,unfreeze}/route.ts`
- Gateway handlers: `~/code/argo/crates/argo-cli/src/handlers/gateway/routes_worker_{memory,trust_grade,policies,freeze}.rs` — **no `routes_worker_skills.rs`**
- Audit emission: `~/code/argo/crates/argo-audit/src/event.rs::AuditKind`
- Worker types (status enum without "frozen"): `~/code/argo/web/lib/types/worker.ts:15-20`
- Wave-8 prior finding (still open): `~/code/argo/docs/skycot/audit/worker-frozen/2026-05-21-wave-8-worker-frozen-behavior.md`

---

## Verdict

The worker-detail surface is **structurally honest but contractually disconnected**: the pages exist, the hooks exist, the optimistic-update plumbing exists, the gateway endpoints mostly exist, and the audit chain emits typed rows. But two contracts diverge — (a) the SPA's Skills tab speaks to a gateway endpoint that was never built, and (b) the audit-row taxonomy known to the SPA is a strict subset of the audit-row kinds the gateway emits. Both make the platform feel less audit-receipts-grade than it actually is.

**Recommended next actions, in priority order:**

1. `mission-gateway-worker-skills-endpoints` — make the Skills tab real (P1, gateway side).
2. `mission-audit-taxonomy-worker-detail-kinds` — promote the gateway's typed audit kinds into the SPA's renderer (P1, frontend side).
3. `mission-frozen-worker-cross-surface-propagation` — Wave-8's open follow-up; add `"frozen"` to `WorkerStatus` and propagate to Approvals + Team + Audit + Notifications (P1).
4. `mission-trust-ratchet-promotion-engine` — make the trust grade actually ratchet (P2 — strategic).
5. `mission-worker-hero-action-menu-wire` — close the no-op `handleHeroAction` warn-only path (P3 — cosmetic, settings page covers the same actions).
