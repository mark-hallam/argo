# Skycot "Usefulness Flow" — Full Implementation Plan

**Status:** PROPOSAL (nothing here is implemented). Authored 2026-06-05.
**Source audit:** ground-truth code audit of the discovery→spec→build→test→deploy→dashboard→cost flow (this session).
**Owner decision required before Phase B** (see §3 Decision Gates).

> Plain-language note (for the operator, who is non-technical): a *migration* = a
> versioned change to the database shape; *endpoint* = a web address the app calls
> to get/save data; *manifest* = a teammate's config file (`vertical.toml`); *SPA* =
> the single-page web app in `web/`; *gateway* = the Rust server the app talks to;
> *executor* = the thing that actually performs an action (a human, an MCP tool, a
> computer-use session); *KPI* = a headline performance number; *audit row* = a
> sealed log entry. Every dev term is expanded on first use.

---

## 1. The goal (the through-line we are making real)

Turn Skycot from *"type a role → get a draft-only assistant"* into the loop the
operator actually wants:

> **Interview me → capture my needs as a spec → build one+ measurable teammates →
> test them for real → put them to work (safely, on my schedule) → show me work-vs-goals
> → show me the cost.**

Today the *foundations* are real (generation, audit trail, approvals, billing
backend, real executors all exist) but **the links that make it useful and
measurable are broken or missing.** This plan closes exactly those links.

---

## 2. The four verified gaps (recap, with code evidence)

| # | Gap | Verified evidence |
|---|-----|-------------------|
| G1 | **No discovery interview** — one textbox, one shot, gathers none of cadence/exceptions/success/guardrails/systems | `web/app/(app)/genesis/page.tsx` + `HeroInput.tsx:152`; `routes_genesis_stream.rs:114`; the only "questions" endpoint returns 6 hardcoded constants `routes_genesis.rs:76` and isn't wired to the live page |
| G2 | **No autonomous run** — scheduler denies every fire AND hire writes a different table than the scheduler reads | `boot_spawns.rs:207` (`ClosedGate`) + `duty_scheduler_gate.rs:70`; hire→`duty` (`store_sql.rs:227`) vs scheduler→`role_duty` (`duty_scheduler_store.rs:116`), no bridge |
| G3 | **Success measured as generic activity, not against goals you set** | `kpi_table.rs:62` (`NotComputableV1` drops all domain metrics); `routes_worker_overview_kpis.rs:196`; no goal/target field anywhere |
| G4 | **Cost is invisible in the app and can't be sliced per teammate** | `spend_ledger` has no `worker_id` (`migrations/v2.rs:34`); zero callers of `/costs/*` in `web/`; billing page shows action-count not dollars (`usage_endpoint.rs`) |

Two further quality gaps that ride along: **the pre-hire "test" is a keyword check, not a competence exam** (`routes_genesis_gate.rs:309` `ContractSubject` ignores the task), and **generated teammates are template-boilerplate by default with no workflows** (`materialize_writer.rs:29` placeholder; LLM-author pass flag-off; no `[workflows]` emitted).

---

## 3. Operator decision gates (resolve before Phase B)

These are **operator-locked calls**, not engineering choices. The plan is built so
**Phase A needs none of them** (it makes the product measurable + demonstrable while
the teammate stays draft-only). Phase B needs all three.

- **DG-1 — Autonomous scheduled execution.** Conflicts with the in-code fail-safe
  "deny every fire until the live auto-approval gate is wired and verified"
  (`boot_spawns.rs:205`). Decision: *do we turn the scheduler on, gated by the real
  per-action auto-approval policy that already exists* (`argo_identity::auto_approval::evaluate`)?
- **DG-2 — Real side-effect execution (override/evolve DEC-025 #6).** Today every
  generated action is `executor="human"` (draft-only) by locked decision. Decision:
  *do we allow a graduated teammate to actually send/post/act via MCP/computer-use,
  keeping approval-gating on sensitive verbs?*
- **DG-3 — Spend attribution shape.** Adding `worker_id` to spend requires a small
  schema change to a hot table. Decision: *confirm we want per-teammate cost
  (recommended) vs tenant/session-only (status quo).* Low controversy; included for
  completeness because it touches a billing-critical table.

If approved, each becomes a `DEC-NNN` ADR in `docs/skycot/decisions/` in the same
turn (per CLAUDE.md decision-register rule).

---

## 4. Phasing, dependency graph, critical path

**Phase A — "Make it measurable & demonstrable"** (teammate stays draft-only; no
new safety surface). Delivers the full useful+measurable story you can demo.
→ WS1 (interview) → WS2 (spec capture) → WS4a (real test) → WS7 (goal dashboards) ; WS8 (cost) in parallel.

**Phase B — "Make it autonomous & executing"** (gated by DG-1/DG-2).
→ WS5 (scheduler bridge + real gate) → WS6 (execution graduation).

**Phase C — "Make the generation deep"** (quality, can run anytime after WS2).
→ WS3 (workflow generation + LLM-author default-on + forge rename).

```
            ┌── WS8 cost (worker_id → spend → /costs/by-worker → SPA)  ──┐
            │                                                            │
WS1 ──► WS2 ─┼─► WS4a real test ─────────────────────────────► (Phase A demo)
 intake  spec │                                                 │
            └─► WS7 goal dashboards ───────────────────────────┘
                                                                
WS2 ──► WS3 generation depth (parallel, Phase C)

WS2 + DG-1 ──► WS5 scheduler ──► WS6 execution graduation (Phase B, needs DG-2)
   (worker_id from WS8 also feeds WS5's auto-approval spend lookup)
```

**Critical path to "useful + measurable + demonstrable":** WS1 → WS2 → WS7 (with WS8
and WS4a landing alongside). That is Phase A. Everything that makes the *demo* prove
usefulness is in Phase A; autonomy (Phase B) is the follow-on that makes it run
unattended.

---

## 5. Workstreams (detailed)

Each workstream lists: **Goal · Current (evidence) · Target · Steps (exact files) ·
Data model · Tests · Acceptance · Effort · Risks · Invariants**. Effort is in
*briefs* (the swarm's <500-LOC work units) + rough complexity; treat as estimates.

Platform invariants every brief must hold (from `CLAUDE.md`): single binary; Rust
1.75; **no file >500 LOC, no fn >80 LOC, no impl block >200 LOC**; **no
`unwrap`/`expect` outside `#[cfg(test)]`**; **additive migrations only**; multi-tenant
(`tenant_id` plumbed through every primitive); capability-based security; vertical
behaviour lives in templates not core forks; **update `FEATURE_MATRIX.md` + regen
HTML in the same commit**; tests for every feature.

---

### WS1 — Discovery interview (close G1)

**Goal.** Replace the single textbox + one-shot generate with a real multi-turn
conversation that elicits, into structured slots: **responsibilities, cadence,
exceptions, success criteria, guardrails, systems/interfaces, approval channels.**

**Current (evidence).** `routes_genesis_stream.rs:114` runs a fixed non-conversational
pipeline once; `HeroInput.tsx:152` is one `<textarea>`; the unused
`/api/v1/genesis/classify` returns 6 constant questions (`routes_genesis.rs:76`). An
*unused* deterministic dialogue already exists: `argo-genesis` `CharterDialogue` /
`charter_dialogue.rs` (6 fixed slots) — we extend its spirit, not reuse it as-is.

**Target.** A slot-filling intake: an LLM proposes the next best question and extracts
the user's answer into typed slots; a deterministic wizard is the fail-safe fallback
(reusing/extending `canonical_questions`). Conversation ends when all *required* slots
are filled + confirmed, producing a new typed `IntakeSpec`.

**Steps.**
1. **New type `IntakeSpec`** in `crates/argo-genesis/src/intake.rs` (new file, <300 LOC).
   Fields (all `Serialize/Deserialize`):
   - `responsibilities: Vec<String>`
   - `cadence: Vec<CadenceSlot>` where `CadenceSlot { description: String, cron: String, entry_action: String }`
   - `exceptions: Vec<String>`
   - `success_criteria: Vec<SuccessCriterion>` where `SuccessCriterion { name: String, kind: KpiKind, target: f64, comparator: KpiComparator }` (reuse the manifest enums from `crates/argo-vertical/src/manifest.rs:256–282`)
   - `guardrails: Vec<String>` (→ becomes `RoleContract.will_not`)
   - `systems: Vec<SystemRef>` where `SystemRef { name: String, mcp_server: Option<String> }`
   - `approval_channels: Vec<String>` (slack-dm/email/sms/discord/whatsapp)
   - `charter: String`, `persona: String`
2. **Intake state machine** `crates/argo-genesis/src/intake_machine.rs`: given the
   conversation so far + filled slots, decide `NextStep::{Ask(question), Confirm(IntakeSpec), Done(IntakeSpec)}`.
   Deterministic ordering of required slots; pure, unit-testable.
3. **LLM question/extract layer** `crates/argo-genesis/src/intake_llm.rs`: two bounded
   calls per turn via the existing `argo_provider::Transport` (same path as
   `routes_genesis_stream_llm.rs:264` `complete_once`): (a) *extract* the latest user
   message into slot deltas (JSON), (b) *propose* the next question. **Fail-safe:** any
   transport/parse error → fall back to the deterministic wizard question for the next
   unfilled slot. Token-capped like the existing classifier.
4. **Multi-turn gateway route.** New `routes_genesis_intake.rs` exposing
   `POST /api/v1/genesis/intake` (a turn: `{conversation_id, user_message}` →
   `{assistant_question | spec_preview | done, filled_slots}`). Server-Sent-Events
   (SSE = a one-way stream of progress lines) reused from the existing stream infra.
   Register in `routes_dispatch_post.rs` (the manual prefix-match chain) per the
   `(status_line, json_body)` contract.
5. **Persist conversation** in a new `genesis_intake` table (slot JSON + transcript),
   tenant-scoped. (Migration: `.sql` slot file `crates/argo-store/migrations/106__genesis_intake.sql` — match the slot-file system that owns `93__role_duties`/`105__tenant_usage`; additive `CREATE TABLE IF NOT EXISTS`.)
6. **SPA conversational UI.** Replace the genesis hero with a chat surface
   `web/app/(app)/genesis/page.tsx` + a new `web/components/skycot/genesis/IntakeChat.tsx`
   (message list + input + live "what we've captured" side panel showing the 7 slot
   groups filling in). New api module `web/lib/api/genesis-intake.ts` (copy the
   `web/lib/api/usage.ts` template; stream via the existing SSE client).
7. **Materialize bridge.** `IntakeSpec::into_capability_spec()` populating the
   `CapabilitySpec` fields that are blank today (`spec.rs:201` `responsibilities:
   Vec::new()` becomes real; `success_metrics` from `success_criteria`; new
   cadence/systems carried for WS2/WS5).

**Data model.** New `IntakeSpec` (Rust) + `genesis_intake` table. `CapabilitySpec`
extended (additive fields, `#[serde(default)]` for back-compat).

**Tests.** Unit: state machine slot-ordering + completion; extract-layer fallback on
provider error. Integration: a scripted 6-turn conversation fills all required slots
and emits a valid `IntakeSpec`. Contract test: route returns the streaming envelope.

**Acceptance.** A user can have a back-and-forth that ends with all 7 slot groups
captured and visible; killing the provider mid-interview degrades to the wizard, never
crashes.

**Effort.** ~6–7 briefs (types, machine, llm layer, route, migration, SPA chat, tests). Largest WS.

**Risks.** Conversation quality depends on the LLM author flag being on (env
`ARGO_GENESIS_LLM_AUTHOR` + a provider key); keep the deterministic wizard genuinely
usable so a no-LLM demo still works.

**Invariants.** Files split to stay <500 LOC (intake/machine/llm separate). Tenant on
`genesis_intake`. No new parallel "spec" type beyond `IntakeSpec` (grep-before-write:
reuse `CapabilitySpec`/`RoleContract`/`KpiDefinition`).

---

### WS2 — Spec capture: stop hardcoding, emit real targets + cadence (close the rest of G1, feed G2/G3)

**Goal.** The teammate that gets generated must carry **the user's** success criteria
(as real `[[kpi]]` rows with targets) and **the user's** cadence (as real duty rows),
not the hardcoded SLA skeleton.

**Current (evidence).** `materialize.rs:283` hardcodes `deflection_target_pct=70,
median_response_seconds=60, csat_threshold=0.85` for everyone; cadence is invented
post-hoc as staggered crons `routes_onboarding_genesis.rs:42`; generated `[[kpi]]` are
`gate=Inform` placeholders (`materialize.rs:262`).

**Target.** `materialize` consumes `IntakeSpec.success_criteria` → emits
`KpiDefinition` rows (`name/kind/threshold/comparator/gate`) with the user's targets,
defaulting `gate=Warn` (visible, non-blocking) so dashboards can score against them.
`IntakeSpec.cadence` → emits duty rows at hire (WS5 consumes them).

**Steps.**
1. `crates/argo-genesis/src/materialize.rs`: replace the constant `SuccessMetricsSection`
   skeleton with a mapping from `IntakeSpec.success_criteria` → `Vec<KpiDefinition>`
   (the manifest already supports this; `all_kpis()` at `manifest.rs:324` is the read
   path). Keep a *sensible* default only when the user gave none.
2. Carry `cadence` through `CapabilitySpec` → into the hire payload so WS5 can write
   duty rows. (Until WS5, store it on the manifest as informational.)
3. Emit `guardrails` → `RoleContract.will_not` and `systems` → `RoleContract.draft_integrations`
   (both fields already exist, `role_contract.rs:87,92`).
4. Update the genesis `vertical_ready` SSE payload (`routes_genesis_stream_payload.rs:45`)
   + TS mirror (`web/lib/api/genesis.ts:249`) to surface the captured KPIs/cadence in
   the review step so the user confirms real numbers.

**Data model.** No new tables; richer manifest content (reuses `KpiDefinition`).

**Tests.** Given an `IntakeSpec` with 3 success criteria, the generated `vertical.toml`
contains 3 matching `[[kpi]]` with the right thresholds/comparators; `all_kpis()` reads
them back.

**Acceptance.** Two teammates built from different success criteria produce *different*
KPI targets in their manifests (no shared skeleton).

**Effort.** ~3–4 briefs.

**Risks.** Manifest round-trip validation (`argo_vertical::load_dir`) must still pass —
add the KPIs within the existing `deny_unknown_fields` schema.

**Invariants.** Additive only; reuse `KpiDefinition`; no new KPI type.

---

### WS3 — Generation depth: workflows + author-on + forge rename (Phase C; quality)

**Goal.** Make generated teammates read like hand-authored hires and include the
step-by-step process they follow.

**Current (evidence).** No `[workflows]` in any of the 67 generated dirs; LLM-author
pass flag-off + degrade-to-template (`routes_genesis_author.rs:32`); placeholder prose
(`materialize_writer.rs:29`); `argo-forge` is a stubbed *skill optimizer* mis-named as
the codegen engine (`forge.rs:80`).

**Target.** (a) Emit a minimal workflow per primary duty (entry_action → linear
checklist) as `workflows/<name>.workflow.toml`, matching the hand-built example
`verticals/receptionist/workflows/caller_greeting_check.workflow.toml`. (b) Default the
author pass ON when a provider key is present, with the existing fail-safe. (c) Rename/
relabel `argo-forge` docs so it isn't confused with generation (cheap doc-only).

**Steps.** Workflow emitter in `crates/argo-genesis/src/` (new `workflow_emit.rs`);
author-default change in `routes_genesis_author.rs`; doc fix + `FEATURE_MATRIX.md` row.

**Effort.** ~3–4 briefs. **Risks.** Workflow schema must validate against `argo-workflow`.
**Invariants.** Vertical-as-template (workflows are template artifacts — correct home).

---

### WS4a — Real competence test (close the "keyword check" gap)

**Goal.** The pre-hire gate must actually make the candidate teammate *attempt the
task* and judge the real output, not check that its description contains keywords.

**Current (evidence).** `routes_genesis_gate.rs:309` `ContractSubject::run` ignores the
task and returns static contract text; metric is `SubstringContains` (`task.rs:15`).
The eval *runner* is real and generic (`run_eval`, `runner.rs:47`); only the *subject*
is fake.

**Target.** New `LiveTurnSubject` implementing the existing trait
`argo_eval::EvalSubject` (`runner.rs:13`: `async fn run(&self, task: &EvalTask) ->
ArgoResult<String>`). Its `run` spins up an ephemeral session for the candidate
manifest and drives a real model turn via `argo_agent_runner::run_turn` (`turn.rs:90`),
returning the assistant text. Add an **LLM-judge metric** using the existing
`argo-judge` crate so scoring is semantic, not substring.

**Steps.**
1. `crates/argo-cli/src/handlers/gateway/genesis_live_subject.rs` (new): `LiveTurnSubject
   { store, cwd, manifest }` impl `EvalSubject`; `run` creates a throwaway dry-run
   session bound to the candidate manifest, calls `run_turn(&store,&cwd,session_id,
   Some(task.input),false,MAX_TOOL_ROUNDS)`, returns `TurnResult.text`.
2. Extend `EvalMetric` (`crates/argo-eval/src/task.rs:15`) with `JudgeRubric { rubric:
   Vec<String> }` scored by `argo-judge` (a real model grading "did the answer satisfy
   the rubric?"). Keep substring metrics for cheap cases.
3. Wire the gate (`routes_genesis_gate.rs`) to run the seeds through `LiveTurnSubject`
   for the *behavioural* floor, keeping the contract/safety/SOP checks as today. Make
   the live floor flag-gated (cost control) but ON for the hire path.

**Data model.** None.

**Tests.** A deliberately-empty manifest fails the live floor; a competent one passes.
Judge-metric unit test against canned good/bad answers.

**Acceptance.** Hire gate fails a teammate that *can't actually do the task*, not just
one with an empty resume.

**Effort.** ~3 briefs. **Risks.** Cost + latency of real turns at hire time → cap
seeds (e.g. 3 of 10 run live), document the cap (no silent truncation). **Invariants.**
Reuse `EvalSubject`/`run_eval`/`argo-judge`; no parallel eval runner.

---

### WS5 — Autonomous run: bridge + real gate (close G2; Phase B, needs DG-1)

**Goal.** A hired teammate's schedule actually reaches the scheduler, and the scheduler
fires it under the **real** per-action auto-approval policy instead of blanket-deny.

**Current (evidence).** Two defects: (A) `boot_spawns.rs:207` wires `ClosedGate`
(denies every fire, `duty_scheduler_gate.rs:70`); (B) hire writes `duty`
(`store_sql.rs:227`, columns: id/worker_id/role_id/tenant_id[TEXT]/cron/entry_action/
description/created_at) while the scheduler reads `role_duty` (`duty_scheduler_store.rs:116`,
columns incl. tenant_id[BLOB]/enabled/paused_until/last_fired_at) — no bridge. The real
evaluator already exists: `argo_identity::auto_approval::evaluate(policy, action, spend,
now) -> AutoApprovalDecision` (`auto_approval.rs:140`).

**Target.** (A) On hire, populate `role_duty` (the scheduler's table) from the captured
cadence; (B) replace `ClosedGate` with `AutoApprovalDutyGate` that maps the existing
evaluator's decision to `FireDecision`.

**Steps.**
1. **Bridge (defect B).** In the hire path (`crates/argo-identity/src/store_hire.rs` /
   `routes_workers_hire.rs`), after creating the worker, write the cadence to
   `role_duty` via `DutyStore::insert` (the same call `routes_duties.rs:158` already
   uses). Decide single-source-of-truth: **recommend `role_duty` is the scheduler's
   source**; keep `duty` for identity/back-compat or migrate reads. Reconcile the
   `tenant_id` TEXT-vs-BLOB difference explicitly in the insert.
2. **Real gate (defect A, DG-1).** New `crates/argo-orchestrator/src/duty_scheduler_auto_gate.rs`:
   `AutoApprovalDutyGate` impl `DutyGate`. Its `evaluate(&self, duty: &DutyRecord)`:
   look up the worker's `AutoApprovalPolicy` (the `worker.auto_approval_json` column,
   `12__worker_identity.sql:25`) by `duty.worker_id`; build a `SpendSnapshot`
   (`WorkerBudget::snapshot`, needs per-worker spend — **depends on WS8 worker_id**);
   build `PendingAction { risk_level, requires_approval, estimated_usd_micros,
   action_type: duty.entry_action }`; call `auto_approval::evaluate(...)`; map
   `AutoApprove→AutoApprove`, `Ask→DeniedGate` (route to approvals inbox instead of
   firing), `Block→DeniedGate`.
3. **Flip the wiring behind a flag.** `boot_spawns.rs:207`: `let gate: Arc<dyn DutyGate>
   = if env("ARGO_DUTY_AUTOFIRE")=="1" { Arc::new(AutoApprovalDutyGate::new(store)) }
   else { Arc::new(ClosedGate) };` — defaults OFF (preserves today's fail-safe), ON only
   after DG-1.
4. **Approvals integration.** When the gate says `Ask`, create an approval item so the
   user can one-click approve a scheduled fire (reuse the approvals route).

**Data model.** No new table (reuse `role_duty`); depends on WS8 for per-worker spend.

**Tests.** Hire → a `role_duty` row exists for the new worker. With the auto-gate +
generous policy, a fire produces a real `run_turn`; with a $0 budget policy, it's denied
and an approval item appears. Tenant isolation: worker A's duty never fires under tenant
B.

**Acceptance.** A hired teammate's scheduled duty fires unattended (when policy allows)
and the run is visible in the audit feed; risky/over-budget fires are held for approval,
not silently run.

**Effort.** ~4–5 briefs. **Risks.** This is the highest-stakes change — it makes the
product *act on its own*. Mitigations: default-off flag, policy-gated, approvals
fallback, 24h probation still forces approval, tenant pinning. **Invariants.** Tenant on
every duty/spend lookup; no `unwrap`; the gate is pure-decision (deterministic, testable).

---

### WS6 — Execution graduation: draft-only → real side-effects (Phase B, needs DG-2)

**Goal.** Let a *proven* teammate actually perform an action (send/post/file) via a real
executor, instead of only writing an instructions file for a human — while keeping
approval on sensitive verbs.

**Current (evidence).** Every generated action is `executor="human"`
(`manifest_actions_executor.rs:42` default; `materialize.rs:185` forces it); the real
executors exist (`McpExecutor`, `ComputerUseExecutorAdapter`, etc.) but `resolve_executor`
(`worker_actions_executor.rs:253`) only routes to them when the manifest declares that
kind, which Genesis never does. The MCP executor in the worker-actions path is currently
a not-implemented stub (`worker_actions_executor_stubs`).

**Target.** A **graduation workflow**: a teammate starts draft-only; after probation +
a track record of approved drafts, the operator promotes *specific* actions to a real
executor by rewriting that action's `ExecutorDeclaration` to the tagged form
(`executor = { kind = "mcp", server = "...", tool = "..." }`, parsed at
`manifest_actions_executor.rs:176`). Sensitive verbs keep `requires_approval=true`.

**Steps.**
1. **Finish `McpExecutor.dispatch`** in the worker-actions path so a tagged MCP action
   actually calls the MCP tool via the existing `McpClientPool` (the runtime dispatch
   path `crates/argo-agent-runner/src/dispatch.rs:228` already reaches real MCP — reuse
   that plumbing rather than a parallel one).
2. **Promotion API.** `POST /api/v1/workers/{id}/actions/{action}/executor` that
   rewrites the action's `ExecutorDeclaration` (human → mcp/computer_use), guarded by
   RBAC (role-based access control) + a "teammate is past probation + has N approved
   drafts" precondition.
3. **Graduation ledger.** Record promotions as audit rows; surface "X is now allowed to
   send email autonomously (with approval)" in the UI.
4. **Keep the fence.** The dangerous-autonomous-capability gate (`hire_gate.rs:317`) and
   sensitive-verb approval stay; promotion never removes an approval requirement, only
   changes *who executes after approval* (human vs tool).

**Data model.** Promotion audit rows; optional `worker_action_executor` override table
or in-place manifest edit (recommend manifest edit + audit, single source of truth).

**Tests.** A promoted MCP action actually performs the side-effect against a mock MCP
server in tests; a non-promoted action still produces a human-handoff bundle; promotion
without probation is rejected; tenant/RBAC enforced.

**Acceptance.** The operator can graduate "draft an email" → "send the email (after
approval)" for a specific teammate, and watch it execute for real.

**Effort.** ~4–6 briefs. **Highest risk** (real outbound side-effects). Mitigations:
per-action opt-in, approval retained, probation precondition, full audit, tenant + RBAC.
**Invariants.** Reuse `McpClientPool` (no parallel pool — the cross-tenant-leak P1 came
from exactly that); MCP-first; capability fence intact.

---

### WS7 — Goal-measurement dashboards (close G3; Phase A)

**Goal.** Show each teammate's work **scored against the success criteria the user set**
(target vs actual vs attainment %), not just generic activity counts.

**Current (evidence).** `kpi_table.rs:62` drops every domain metric as
`NotComputableV1`; `routes_worker_overview_kpis.rs:196` re-drops them; the overview JSON
emits `value`+`trend` only (`:204`); there is no target/goal field anywhere; bullets are
hardcoded per vertical (`kpi_table.rs:349`).

**Target.** Targets now exist on the manifest (`[[kpi]]` from WS2). Extend the compute
path to (a) read each KPI's `target`, (b) compute `actual` from the audit-derived count
*where a countable source exists*, (c) emit `attainment_pct` and a met/at-risk/missed
status, (d) add a dedicated **Scorecard** surface.

**Steps.**
1. **Targets into compute.** `routes_worker_overview_kpis.rs::compute_kpis` (`:186`):
   join the manifest `all_kpis()` (`manifest.rs:324`) with the audit-derived actuals;
   for KPIs whose source is countable, stop returning `None` at `:196` and instead emit
   `{key,label,value,target,attainment_pct,status,trend,sparkline_7d}`.
2. **Honest gaps.** For success criteria with no countable audit source yet (e.g.
   "revenue"), show "target set — measurement not wired" rather than a fake number
   (keeps NO-THEATER). Track those as follow-on instrumentation briefs.
3. **Scorecard endpoint.** New `routes_worker_scorecard.rs` →
   `GET /api/v1/workers/{id}/scorecard`; register a `scorecard ` arm in
   `routes_dispatch_get_api_groups.rs:225` (the per-worker subpath dispatcher), same
   `(status_line, json_body)` contract as `routes_worker_overview.rs:69`.
4. **SPA.** New `web/lib/api/scorecard.ts` (copy `usage.ts` template); new surface
   `web/app/(app)/workers/[id]` scorecard tab (or a `/scorecard` page) showing
   goal-vs-actual cards + a team-level "goals hit this week" roll-up on `/team`.
5. **Team headline.** Extend `BulletSource` (`kpi_table.rs:23`) with a `GoalAttainment`
   variant so the team page can say "Maya: 82% of reply-time goal" instead of dropping it.

**Data model.** None new (targets come from manifest); optionally cache attainment
snapshots for trend.

**Tests.** A teammate with a "respond within 60s" target and audit rows showing 45s
median renders attainment >100% / "met"; a missed target renders "at risk". Tenant
scoping on the scorecard route.

**Acceptance.** For a teammate you defined goals for, you see those exact goals with
real attainment — the "measurable usefulness" requirement.

**Effort.** ~4–5 briefs. **Risks.** Some success criteria aren't countable from audit
rows yet → be explicit, don't fake. **Invariants.** NO-THEATER on every number;
tenant on the route; reuse `KpiDefinition`.

---

### WS8 — Cost: per-teammate + visible in the app (close G4; Phase A, parallel)

**Goal.** Record cost per teammate and show real dollars in the product, including
cost-per-teammate and (later) cost-vs-value next to the scorecard.

**Current (evidence).** `spend_ledger` has `tenant_id, session_id` only — no `worker_id`
(`migrations/v2.rs:34`); `record_turn_spend` (`helpers.rs:150`) and `record_spend`
(`spend.rs:11`) never see a worker; the real `/costs/by-day|recent|by-session` routes
(`routes_costs_by_day.rs`, `routes_post_costs.rs`) have **zero callers in `web/`**; the
billing page shows action-count vs cap from `tenant_usage` (`usage_endpoint.rs`), a
different table.

**Target.** (a) Attribute spend to a worker; (b) surface real dollars in the SPA; (c) a
`/costs/by-worker` view powering cost-per-teammate.

**Steps.**
1. **Worker on the session (DG-3).** Add a nullable `worker_id` to the session record
   and set it at session creation where known (duty-fired sessions have it from
   `DutyRunContext.worker_id`; interactive teammate chats from the selected worker).
   *Implementer check:* confirm whether the `SessionStore` session row already has a
   worker/participant column before adding one.
2. **Worker on spend.** Additive migration (Rust `vNN.rs` system that owns
   `spend_ledger` — next is **v90**, `ALTER TABLE spend_ledger ADD COLUMN worker_id BLOB`).
   Thread `worker_id` into `SpendCtx` (`helpers.rs:127`, built at `turn_run_loop.rs:132`)
   → `record_turn_spend` → `record_spend` (`spend.rs`). Null for non-teammate sessions.
3. **Tenant-scope the cost routes.** The existing `/costs/*` read `spend_ledger`
   workspace-wide; add a tenant filter (multi-tenant safety) and a new
   `GET /api/v1/tenant/costs/by-worker` (GROUP BY worker_id).
4. **SPA cost surfaces.** New `web/lib/api/cost.ts` (copy `usage.ts`); add a real
   dollar panel to `/billing` (spend this month, by day, by teammate) and a cost chip on
   worker-detail + the WS7 scorecard ("this teammate cost $X, did Y" — the cost-vs-value
   pairing).
5. **Stamp `cost_usd` on the agent turn's audit row** (not just the ledger) so the
   existing per-action cost chip (`ActivityRow.tsx:109`) is populated for normal work,
   not only forge/eval.

**Data model.** `session.worker_id` (nullable), `spend_ledger.worker_id` (nullable,
additive). Both back-compatible.

**Tests.** A duty-fired teammate turn writes a `spend_ledger` row carrying its
`worker_id`; `/costs/by-worker` returns per-teammate totals; tenant A can't read tenant
B's costs; null worker for system sessions doesn't break rollups.

**Acceptance.** The operator sees, in dollars, total spend, spend per teammate, and
spend next to each teammate's goal attainment.

**Effort.** ~4–5 briefs. **Risks.** `spend_ledger` is hot/billing-critical — additive
column + backfill-null only; never rewrite history. **Invariants.** Additive migration;
tenant on every cost read; no `unwrap`.

---

## 6. Cross-cutting concerns

- **Testing.** Every WS ships unit + integration tests (CLAUDE.md: "tests for every
  feature"). Phase A ends with one **end-to-end test**: scripted interview →
  generated teammate with real KPIs → live competence test passes → drafts produced →
  scorecard shows goal-vs-actual → cost row attributed. This is also the demo spine.
- **Multi-tenant.** New tables (`genesis_intake`), new columns (`worker_id`), and every
  new route carry `tenant_id`; add tenant-isolation negative tests (the repo's standard,
  e.g. `gateway_uuid_routes_tenant_ownership.rs`).
- **CI invariants.** Watch the file-size budget (split intake/scorecard/cost modules);
  no `unwrap` outside tests; additive migrations only (`scripts/check-migration-slots.sh`);
  `cargo clippy -p argo-cli` has a known pre-existing `await_holding_lock` blocker
  (memory: `project_argo_cli_clippy_await_holding_lock_blocker`) — boundary-note it on
  argo-cli PRs.
- **Ledgers in-band.** Each shipped capability updates `docs/build-plan/FEATURE_MATRIX.md`
  (+ regen HTML) in the same commit; each DG decision gets a `DEC-NNN` ADR.
- **Observability first** (priority #2): new fires/promotions/spend all emit audit rows.

---

## 7. Brief decomposition for the swarm

| Brief id | WS | Depends on | One-liner |
|---|----|-----------|-----------|
| `useful-ws1-intakespec-type` | 1 | — | `IntakeSpec` + slot enums (reuse manifest KPI enums) |
| `useful-ws1-intake-machine` | 1 | intakespec | deterministic slot state machine |
| `useful-ws1-intake-llm` | 1 | machine | LLM extract+propose with wizard fallback |
| `useful-ws1-intake-route` | 1 | llm | `POST /genesis/intake` SSE + `genesis_intake` table (106__) |
| `useful-ws1-intake-spa` | 1 | route | chat UI + captured-slots panel + `genesis-intake.ts` |
| `useful-ws2-emit-real-kpis` | 2 | ws1 | success_criteria → `[[kpi]]` (drop the skeleton) |
| `useful-ws2-carry-cadence-systems` | 2 | ws1 | cadence/guardrails/systems → manifest/contract |
| `useful-ws4a-live-turn-subject` | 4a | ws2 | `LiveTurnSubject: EvalSubject` via `run_turn` |
| `useful-ws4a-judge-metric` | 4a | — | `EvalMetric::JudgeRubric` via `argo-judge` |
| `useful-ws4a-wire-gate-live-floor` | 4a | subject,judge | hire gate runs live behavioural floor |
| `useful-ws7-targets-into-compute` | 7 | ws2 | join manifest targets with audit actuals |
| `useful-ws7-scorecard-endpoint` | 7 | targets | `GET /workers/{id}/scorecard` |
| `useful-ws7-scorecard-spa` | 7 | endpoint | scorecard surface + team goals rollup |
| `useful-ws8-session-worker-id` | 8 | — | nullable `session.worker_id`, set on create |
| `useful-ws8-spend-worker-id` | 8 | session-worker | v90 `spend_ledger.worker_id` + thread `SpendCtx` |
| `useful-ws8-cost-routes-tenant+byworker` | 8 | spend-worker | tenant-scope `/costs/*` + `/costs/by-worker` |
| `useful-ws8-cost-spa` | 8 | cost-routes | `cost.ts` + dollar panels + audit `cost_usd` stamp |
| `useful-ws5-hire-roleduty-bridge` | 5 | ws2, DG-1 | hire writes `role_duty` (tenant TEXT→BLOB) |
| `useful-ws5-auto-approval-gate` | 5 | bridge, ws8 | `AutoApprovalDutyGate` adapts `auto_approval::evaluate` |
| `useful-ws5-flip-gate-flag` | 5 | gate | `ARGO_DUTY_AUTOFIRE` flag in boot_spawns |
| `useful-ws6-finish-mcp-executor` | 6 | DG-2 | real `McpExecutor.dispatch` via `McpClientPool` |
| `useful-ws6-promotion-api` | 6 | mcp-exec | per-action executor promotion + RBAC + probation |
| `useful-ws3-workflow-emit` | 3 | ws2 | emit `workflows/*.workflow.toml` |
| `useful-ws3-author-default-on` | 3 | — | default `ARGO_GENESIS_LLM_AUTHOR` on w/ fallback |
| `useful-e2e-phaseA-smoke` | A | ws1/2/4a/7/8 | end-to-end demo-spine test |

~25 core briefs (+ tests/splits likely pushes to ~30–40 work units).

---

## 8. Effort & rough timeline (estimates)

| Phase | Workstreams | Briefs | Rough swarm wall-clock* |
|---|---|---|---|
| **A — measurable & demonstrable** | WS1, WS2, WS4a, WS7, WS8 | ~17 | ~2–3 weeks |
| **B — autonomous & executing** (DG-1/DG-2) | WS5, WS6 | ~6 | ~1.5–2 weeks |
| **C — generation depth** | WS3 | ~4 | ~1 week (parallelizable) |

\* On the current 2–3 worker swarm with conductor review, mixing easy/hard briefs.
Estimates, not commitments; WS1 and WS6 carry the most uncertainty.

---

## 9. What "useful + measurable + demonstrable" looks like at each phase

**After Phase A (the headline):** You sit down, *have a conversation* with Skycot about
a role; it captures responsibilities, schedule, exceptions, the goals you care about,
your guardrails and your tools; it builds a teammate and **actually tests whether it can
do the job** before hiring; the teammate drafts work; and you watch a **scorecard of its
work against the goals you set**, with the **dollar cost per teammate** beside it. That
is a fully demonstrable, measurable usefulness story — with zero new autonomy risk
(still draft-only).

**After Phase B:** the teammate **does the work on your schedule** and, for actions
you've graduated, **executes for real** — every risky action still gated by approval,
every fire and spend audited.

**After Phase C:** generated teammates read like hand-authored hires and carry their own
step-by-step workflows.

---

*End of plan. Nothing herein is implemented; this is a design proposal pending operator
sign-off on the Phase B decision gates (§3).*
