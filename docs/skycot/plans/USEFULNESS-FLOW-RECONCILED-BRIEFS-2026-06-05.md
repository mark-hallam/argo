# Usefulness Flow — Reconciled brief pack (deduplicated deltas)

**Status:** STAGED — not enqueued. Authored 2026-06-05 against `origin/main 466da67b7`.
**Companion to:** `docs/skycot/plans/USEFULNESS-FLOW-IMPLEMENTATION-PLAN-2026-06-05.md` (the full design).
**Decisions:** [[DEC-036]] (autonomous scheduler), [[DEC-037]] (graduated execution), [[DEC-038]] (per-teammate cost).

## Read this first — why this pack exists

The original plan had 8 workstreams (WS1–WS8). A queue cross-check on 2026-06-05
found that **a large part of "Phase A" is already scoped or built**, so authoring
the plan as-written would have **duplicated and collided** with live work. This
pack is the reconciled result: **only the genuinely-new deltas**, each declaring
what already exists. Renamed **`uf-*`** (NOT `WS*`) to avoid colliding with the
active sprint's WS1–WS6.

### What already exists (do NOT rebuild)

| Capability | Already covered by | Status |
|---|---|---|
| `/genesis` SPA page + `POST /api/v1/genesis` SSE | `mission-genesis-spa-page-describe-and-watch` (#2326), `…-gateway-route-streaming-api` (#2309) | MERGED |
| Guardrails / access-scope / approval-threshold capture UI | `mission-genesis-access-controls-and-guardrails-spec-ui` | BLOCKED → about to unblock (dep #2326 merged) |
| Systems/wiki/Slack import, SOP/doc upload, voice intake, comprehension quiz | `mission-genesis-{integration-import,document-upload,voice-input,comprehension-verification-quiz-mode}` | BLOCKED → about to unblock |
| Real live-agent eval + scorecard scoring | `eval-teammate-live-agent-runner-and-scorecard` (#2668); backbone shipped `195ff2ccf` | IN_REVIEW |
| Auto-approval policy + budget + multi-window | `H-auto-approval` (#796–805), `claude-auto-approval-multiwindow-e2e` (#1672) | MERGED |
| Duty CRUD endpoints, routes_duties split, multi-worker handoff | `J13-gateway-duty-endpoints` (#1514), `#1617`, `#1654` | MERGED |
| `KpiSparkline` component | `E4-i-kpi-sparkline-primitives` (#1435) | MERGED |
| Real executors (MCP/computer-use/browser/api) | various | MERGED (just not reachable from a Genesis hire) |

### Sequencing rules (apply to EVERY brief here)

1. **After the tenant keystone.** Every brief assumes per-request tenant
   resolution. The sprint's `sprint-ws1-tenant-resolve-from-session-jwt-not-cwd`
   (cwd→JWT) is mid-build; UF work lands **after** it (or rebases onto it).
2. **After the genesis Phase-2 cascade** where noted (A1/A3 build on those panels;
   don't fork `web/components/skycot/genesis/`).
3. **Coordinate with the sprint** on three shared surfaces: empty-states (WS2 ↔
   A5), billing (WS4 ↔ A7), de-mock (WS3 ↔ A5/A7).
4. **Phase B briefs (B1–B3) are gated** on [[DEC-036]]/[[DEC-037]] go-live (default-OFF flags until verified).
5. At **enqueue time** the conductor splits each section below into its own
   `docs/build-plan/streams/<id>.md`, re-stamps the freshness header against
   then-current `origin/main`, and adds a `BUILD_STATUS.md` row under the right
   priority header. (This pack is staged so the 24/7 swarm doesn't grab it mid-sprint.)

---

# PHASE A — measurable & demonstrable (teammate stays draft-only)

## A1 · `uf-genesis-capture-cadence-exceptions-success`
**Owner:** claude · **Depends:** genesis Phase-2 SPA panels landed; tenant keystone.
**What's new vs existing:** the genesis Phase-2 tree captures guardrails, systems,
docs, voice, comprehension — but **nothing captures cadence/schedule, exceptions,
or success-criteria**. This adds those three as typed slots.
**Grep-before-write:** `rg 'pub struct CapabilitySpec' crates/argo-genesis` (extend,
don't fork — `spec.rs:143`); `rg 'RoleContract' crates/argo-genesis/src/role_contract.rs`.
**May-touch:** `crates/argo-genesis/src/spec.rs` (+`cadence`, `exceptions`,
`success_criteria: Vec<SuccessCriterion{name,kind,target,comparator}>` reusing the
manifest `KpiKind`/`KpiComparator` enums; additive `#[serde(default)]`);
`role_contract.rs`; `routes_genesis_stream_payload.rs:45` + TS mirror
`web/lib/api/genesis.ts:249` (surface in the review step); a capture panel in
`web/components/skycot/genesis/` consistent with the Phase-2 panels.
**Acceptance:** an intake that states a schedule, an exception, and a measurable
goal persists all three as typed fields visible in the `vertical_ready` review;
back-compat (old specs deserialize). Tests pin the round-trip.
**Effort:** ~1 brief (M).

## A2 · `uf-genesis-emit-real-kpis-and-cadence`
**Owner:** claude · **Depends:** A1.
**What's new:** stop emitting the hardcoded SLA skeleton for everyone; emit the
user's success-criteria as real `[[kpi]]` rows, and carry cadence for duty creation.
**Grep-before-write:** the manifest already supports this — `KpiDefinition`
(`crates/argo-vertical/src/manifest.rs:228`), unified read `all_kpis()` (`:324`).
**May-touch:** `crates/argo-genesis/src/materialize.rs` (replace the constant
`SuccessMetricsSection` at `:283` with a map from `success_criteria` →
`Vec<KpiDefinition>` at `:262`; default `gate=Warn` so dashboards score against
them; keep a sensible default only when the user gave none).
**Acceptance:** two teammates built from different success-criteria produce
**different** KPI targets in their `vertical.toml`; `all_kpis()` reads them back;
manifest round-trip (`argo_vertical::load_dir`) still validates.
**Effort:** ~1 brief (S–M).

## A3 · `uf-genesis-conversation-layer`  ✅ RESOLVED via [[DEC-039]]
**Owner:** claude · **Depends:** genesis Phase-2 panels; A1.
**Resolved ([[DEC-039]]):** ships as a new additive **"Talk it through" mode**
alongside the one-shot describe-and-watch flow (NOT a replacement); both entry
points feed the same `IntakeSpec → CapabilitySpec` materialization. Do not fork
the Phase-2 panels or the materializer.
**What's new:** a conversational intake that asks follow-ups and fills the A1 slots
+ responsibilities by dialogue, feeding the **same** materialization path (no fork).
**May-touch:** new `crates/argo-genesis/src/intake_machine.rs` + `intake_llm.rs`
(slot state machine + LLM extract/propose with a deterministic wizard fallback);
new `routes_genesis_intake.rs` (`POST /api/v1/genesis/intake`, SSE, registered in
`routes_dispatch_post.rs`); `genesis_intake` table (new `.sql` slot `106__`);
`web/components/skycot/genesis/IntakeChat.tsx` + `web/lib/api/genesis-intake.ts`.
**Acceptance:** a 6-turn conversation fills all required slots + emits a valid spec;
provider failure degrades to the wizard, never crashes; files stay <500 LOC.
**Effort:** ~3 briefs (L) — the largest delta. Can be deferred without blocking A2/A5.

## A4 · `uf-hire-gate-wire-live-eval`
**Owner:** claude/evaluator-lane · **Depends:** `eval-teammate-live-agent-runner-and-scorecard` (#2668) MERGED.
**What's new:** the hire gate still scores the tautological `ContractSubject`
(`routes_genesis_gate.rs:309`, ignores the task, substring-matches). Replace it with
the **real** live-agent runner from #2668 — the [[DEC-025]] #7 fast-follow
("deterministic floor first, LLM-graded rubric is the fast-follow"). Do NOT rebuild
the runner.
**Grep-before-write:** `rg 'EvalSubject' crates/argo-eval/src/runner.rs` (trait at
`:13`); reuse `argo_eval::teammate` + `scorecard.rs` from #2668.
**May-touch:** `crates/argo-cli/src/handlers/gateway/routes_genesis_gate.rs` (swap
`ContractSubject` → a subject that drives the #2668 runner against the seeds);
flag-gate for cost (run a capped subset live), document the cap (no silent truncation).
**Acceptance:** a deliberately-incompetent manifest **fails** the live floor; a
competent one passes; the gate no longer passes on keyword-parroting alone.
**Effort:** ~1 brief (M).

## A5 · `uf-scorecard-goal-attainment-surface`
**Owner:** claude · **Depends:** A2 (targets exist); tenant keystone. **Coordinate:** sprint WS2 empty-states.
**What's new:** dashboards measure generic activity and **drop** every domain/goal
metric (`kpi_table.rs:62` `NotComputableV1`; `routes_worker_overview_kpis.rs:196`).
Add **goal-vs-actual**: for KPIs with a countable audit source, emit
`target`+`attainment_pct`+`status`; add a scorecard surface. **Reuse** `KpiSparkline`
(#1435) and `scorecard.rs` scoring concepts (#2668) — don't reinvent.
**May-touch:** `routes_worker_overview_kpis.rs:186` `compute_kpis` (join manifest
`all_kpis()` targets with audit actuals; stop returning `None` for sourced metrics);
new `routes_worker_scorecard.rs` + a `scorecard ` arm at
`routes_dispatch_get_api_groups.rs:225`; `web/lib/api/scorecard.ts` (copy the
`web/lib/api/usage.ts` template) + a scorecard surface on worker-detail + a team
"goals hit this week" rollup. **NO THEATER:** goals with no countable source yet show
"target set — measurement not wired", never a fabricated number.
**Acceptance:** a teammate with a "respond < 60s" target + audit rows at 45s renders
"met / >100%"; a missed target renders "at risk"; scorecard route is tenant-scoped.
**Effort:** ~2 briefs (M–L).

## A6 · `uf-spend-worker-id-attribution`
**Owner:** claude · **Depends:** tenant keystone. **Reconcile:** the stale `claude-session-worker-id-schema` brief (authored, no ledger row — verify whether `session.worker_id` already exists FIRST).
**What's new:** `spend_ledger` has no `worker_id` (`migrations/v2.rs:34`) → cost can't
be split per teammate. Add it, additively.
**May-touch:** new additive migration (`spend_ledger` is owned by the Rust `vNN.rs`
system → next is `crates/argo-store/src/migrations/v90.rs`: `ALTER TABLE spend_ledger
ADD COLUMN worker_id BLOB`); `session.worker_id` (nullable, set at session creation —
duty-fired sessions have it from `DutyRunContext.worker_id`); thread `worker_id`
through `SpendCtx` (`helpers.rs:127`, built `turn_run_loop.rs:132`) →
`record_turn_spend` → `record_spend` (`spend.rs:11`).
**Acceptance:** a teammate turn writes a `spend_ledger` row carrying its `worker_id`;
null for system sessions doesn't break existing rollups; additive migration only.
**Effort:** ~1–2 briefs (M).

## A7 · `uf-cost-by-worker-and-spa-surface`
**Owner:** claude · **Depends:** A6. **Coordinate:** sprint WS4 billing.
**What's new:** the dollar-cost endpoints exist (`routes_costs_by_day.rs`,
`routes_post_costs.rs`) but have **zero `web/` callers**, are workspace-wide (not
tenant-scoped), and the turn loop never stamps `cost_usd` on the audit row.
**May-touch:** tenant-scope the `/costs/*` reads + new `/api/v1/tenant/costs/by-worker`
(GROUP BY worker_id); `web/lib/api/cost.ts` (copy `usage.ts`) + a dollar panel on
`/billing` and a cost chip on worker-detail/scorecard (the cost-vs-value pairing);
stamp `cost_usd` on the agent-turn audit row so `ActivityRow.tsx:109` populates for
normal work (today only forge/eval do).
**Acceptance:** the operator sees total $ spend, $ per teammate, and $ beside goal
attainment; tenant A can't read tenant B's costs.
**Effort:** ~1–2 briefs (M).

---

# PHASE B — autonomous & executing (gated by DEC-036 / DEC-037, default-OFF flags)

## B1 · `uf-scheduler-hire-roleduty-bridge`
**Owner:** claude · **Depends:** tenant keystone; [[DEC-036]].
**What's new:** hire writes table `duty` (`store_sql.rs:227`) but the scheduler reads
`role_duty` (`duty_scheduler_store.rs:116`) — no bridge, so a hire's cadence is
invisible to the scheduler. Make hire populate `role_duty` (the scheduler's source).
**May-touch:** `crates/argo-identity/src/store_hire.rs` / `routes_workers_hire.rs`
(after creating the worker, write cadence via `DutyStore::insert` — the same call
`routes_duties.rs:158` already uses); **reconcile the `tenant_id` TEXT (`duty`) vs
BLOB (`role_duty`) difference** explicitly.
**Acceptance:** hiring a teammate with a schedule creates a `role_duty` row the
scheduler can list; tenant-isolated.
**Effort:** ~1 brief (M).

## B2 · `uf-scheduler-auto-approval-gate`
**Owner:** claude · **Depends:** B1, A6 (per-worker spend snapshot); [[DEC-036]].
**What's new:** replace `ClosedGate` (`boot_spawns.rs:207`) with a real gate that
reuses the **merged** `auto_approval::evaluate` (`argo-identity/src/auto_approval.rs:140`).
**Grep-before-write:** `rg 'AutoApprovalPolicy|auto_approval::evaluate' crates` — wire
the existing evaluator, do NOT author a new policy engine.
**May-touch:** new `crates/argo-orchestrator/src/duty_scheduler_auto_gate.rs`
(`AutoApprovalDutyGate` impl `DutyGate`: resolve the worker's `AutoApprovalPolicy` +
fresh `SpendSnapshot` by `worker_id`, build a `PendingAction` from the duty's
`entry_action`, map `AutoApprovalDecision`→`FireDecision`; `Ask`→approvals inbox);
`boot_spawns.rs:207` flag select (`ARGO_DUTY_AUTOFIRE`, **default OFF**).
**Acceptance:** with the flag on + a generous policy, a fire produces a real
`run_turn`; with a $0 budget, it's denied and an approval item appears; tenant-isolated;
flag off = today's deny-all preserved.
**Effort:** ~2 briefs (M–L).

## B3 · `uf-execution-graduation-and-promotion`
**Owner:** claude · **Depends:** [[DEC-037]].
**What's new:** generated actions are all `executor="human"`; the real `McpExecutor`
in the worker-actions path is a not-implemented stub. Finish it + add the per-action
graduation/promotion path ([[DEC-037]]).
**Grep-before-write:** reuse `McpClientPool` from the runtime dispatch path
(`argo-agent-runner/src/dispatch.rs:228`) — do NOT author a parallel pool (the prior
cross-tenant-leak P1 came from exactly that).
**May-touch:** finish `McpExecutor::dispatch` (`worker_actions_executor*`); new
`POST /api/v1/workers/{id}/actions/{action}/executor` (rewrites that action's
`ExecutorDeclaration` human→tagged `{kind,server,tool}`, RBAC + probation
precondition); promotion audit rows; **approval stays** on sensitive verbs; the
dangerous-autonomous-capability fence (`hire_gate.rs:317`) stays.
**Acceptance:** a promoted MCP action performs a real side-effect against a mock MCP
server in tests; a non-promoted action still produces a human-handoff bundle;
promotion without probation is rejected; tenant + RBAC enforced.
**Effort:** ~2–3 briefs (L) — highest risk; per-action opt-in keeps blast radius small.

---

# PHASE C — generation depth (anytime after A2; quality)

## C1 · `uf-genesis-workflow-emit`
**Owner:** claude · **Depends:** A2. **Grep-before-write:** check `claude-genesis-refine-*` / `-iteration-cycle` briefs first.
**What's new:** no generated teammate emits `[workflows]`. Emit a minimal
`workflows/<name>.workflow.toml` per primary duty, matching the hand-built
`verticals/receptionist/workflows/*.workflow.toml`; validate against `argo-workflow`.
**Effort:** ~1–2 briefs (M).

## C2 · `uf-genesis-author-default-on`
**Owner:** claude · **Depends:** none. **What's new:** default the rich LLM-author
pass ON when a provider key is present (it exists, flag-off, fail-safe to template) —
the [[DEC-025]] full-synthesis intent. **May-touch:** `routes_genesis_author.rs:32`.
**Effort:** ~1 brief (S).

---

## Enqueue checklist (for the conductor, when the operator says go)

1. Confirm the tenant keystone (`sprint-ws1-*`) has landed (or rebase the UF brief onto it).
2. Split the chosen section(s) above into `docs/build-plan/streams/<uf-id>.md`,
   each with its own freshness header re-stamped against current `origin/main` +
   a `Validate before claim: bash scripts/check-brief-freshness.sh <uf-id>` line.
3. Add a `BUILD_STATUS.md` row per brief under the right priority header; respect
   the dependency order (A1→A2→A5; A6→A7; B1→B2; #2668→A4; DEC gates for B).
4. `git fetch && rebase` before pushing the ledger (shared checkout); `git add`
   explicit files only.
5. Update `FEATURE_MATRIX.md` rows as each ships (+ regen HTML), per the in-band ledger rule.
