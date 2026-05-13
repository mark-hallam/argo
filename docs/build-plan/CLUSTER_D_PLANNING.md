# Cluster D — Architectural redesign planning (6 cases)

**Authored 2026-05-13 by conductor** against `origin/main 033b046c`. Prep doc for the planning session the SKYCOT CEO authorized to walk through the 6 DEEP-DRIFT briefs that require real architecture decisions (not mechanical rewrites or precursor briefs).

**Meeting shape**: 30 min. Per-case ~5 min. CEO picks 1 of 2-3 options per case. Conductor then authors the resulting v2 brief in a follow-up tick.

---

## Case 1 — `mission-judge-dreams-output`

**Original intent**: ship a "judge" that scores Dreams (offline-curated skill candidates) and decides which to promote vs reject vs revise.

**Current reality**: there is no `crates/argo-dreams/` crate. Dreams logic is split across 3 existing crates:
- `argo-skill::dreams_distillation` — generates skill proposals from session trajectories
- `argo-runtime::dreams_plan_templates` — learns plan templates from successful sessions
- `argo-memory::dreams_backend` — caches Dreams artifacts

There is no central judge / approval-queue plumbing.

**Options**:
- **A. New `argo-dreams` crate** that aggregates the 3 existing surfaces + ships the judge. Big refactor; ~1500 LOC across 3 crate moves + the judge.
- **B. Add judge as a new module in `argo-skill`** alongside `dreams_distillation`. Smallest churn; ~400 LOC. Couples judge to skill-specific Dreams; doesn't address plan-template + memory Dreams.
- **C. Drop the brief** — the existing 3 surfaces already use scoring via `argo-skill::trust::TrustGrade`. No judge needed; the existing trust-grade ladder IS the judge.

**My recommendation**: **C (drop)**. The TrustGrade ladder (Pending → Promoted → Rejected, see `argo-store::learned_plan_template`) is already the promote-vs-reject decision. The brief was authored before that lifecycle existed. If you need a unified judge across skill + plan-template + memory Dreams, that's a v2 architecture conversation, not a single brief.

---

## Case 2 — `mission-roster-resolve-policy`

**Original intent**: per-worker policy override resolution — when a worker hits an approval gate, the roster store should return a possibly-overridden policy.

**Current reality**: `RosterStore::lookup_for_worker(worker_id)` does not exist. Closest API is `RosterStore::rosters_for_worker(worker_id) -> Vec<Roster>`. No `policy_override` schema column. The policy-override concept is implemented elsewhere as **operator-risk-override** rows (migration slot 62) — a different (broader) shape.

**Options**:
- **A. Add `lookup_for_worker` + `policy_override` schema** to RosterStore as the brief originally intended. New migration + new index. Doesn't align with the operator-risk-override pattern that already exists.
- **B. Rewrite brief to use `operator_risk_overrides`** (the existing surface). Re-target the brief at extending operator-risk-overrides with policy-shape semantics rather than introducing a 2nd override mechanism.
- **C. Drop the brief** — operator-risk-overrides already covers the same operator need. Per-worker policy is a UX nice-to-have, not a load-bearing feature for v1.

**My recommendation**: **B (rewrite onto operator_risk_overrides)**. Avoids a second override mechanism. The brief becomes "extend operator_risk_overrides with per-worker policy resolution". ~300 LOC. Confirms the operator-risk-overrides table is the canonical override mechanism.

---

## Case 3 — `mission-delegate-task-fallback-chain`

**Original intent**: when `delegate_task` tool dispatch fails, try a fallback chain (different worker → different vertical → operator escalation).

**Current reality**: 4 cited surfaces don't match:
1. `ToolContext` doesn't carry `WorkerId`/`session_id`/`handoff_id` fields (the actual struct at `argo-tool/src/tool.rs:51-87` is much smaller).
2. `Handoff` type referenced doesn't exist (the closest thing is `OrchestratorStore::update_handoff_status`, a string-state API).
3. `EffectivePolicy` doesn't exist as a typed concept.
4. `delegate_task` is still a **stub tool** in `argo-tool::tools::delegate_task` — never wired into production.

**Options**:
- **A. Implement the missing 4 surfaces** (ToolContext extension, Handoff type, EffectivePolicy, real delegate_task). Massive — easily 2000+ LOC. Foundational architecture pull.
- **B. Implement `delegate_task` first** (make the stub real, with the existing primitives) as a precursor brief; revisit the fallback chain after.
- **C. Drop the brief** — delegate_task being a stub suggests the operator never had user-demand pressure for it. v1.x scope or later.

**My recommendation**: **C (drop)**. delegate_task is a stub because nothing's calling it. Building a fallback chain on top of a stub is over-engineering. Revisit once a real customer needs delegate_task and a `mission-delegate-task-make-real` brief lands first.

---

## Case 4 — `mission-llm-call-dedup-cache`

**Original intent**: dedup LLM provider calls within a request lifecycle (same prompt repeated within a turn → cache).

**Current reality**: brief targets `argo-cli/src/handlers/gateway/boot.rs` for the dedup wire-up, but **gateway boot does not construct provider transports**. The real construction path is `crates/argo-agent-runner/src/transport.rs::build_transport`.

**Options**:
- **A. Rewrite brief to target `argo-agent-runner::transport::build_transport`** — the actual construction site. The dedup wrapper becomes a transport-layer decorator. ~300-400 LOC, clean scope. **This is essentially a mechanical fix to wrong-may-touch — the brief's design is fine, just the location is wrong.**
- **B. Drop the brief** — Anthropic's prompt-cache (already wired via #1539 + #1543) gives most of the dedup benefit. Marginal value in a separate dedup cache.

**My recommendation**: **A (rewrite, retarget to transport.rs)**. The dedup is genuinely useful (saves money on retry storms) and the brief's design is sound. Just point it at the right surface. ~30 min conductor work.

---

## Case 5 — `mission-anomaly-alerting-notifychannel`

**Original intent**: anomaly detection in audit chain / metrics → fire alerts via the NotifyChannel infrastructure.

**Current reality**:
- No `NotifyChannelDispatch` type exists.
- Live `argo-channels::NotifyChannel` sends approval-shaped `Notification`s (not anomaly-shaped).
- The only multi-channel dispatcher is `worker_actions_notify::EnvChannelSender`/`NotifySender`, which is CLI-private (outside may-touch).
- Tracing subscriber doesn't have the alert-emission hook the brief assumes.

**Options**:
- **A. Add `NotifyChannelDispatch` type + extend `NotifyChannel` to carry anomaly shapes** — significant API change to the channels crate.
- **B. Rewrite brief to use `EnvChannelSender`** — expand may-touch to argo-cli's notify infrastructure. ~600 LOC.
- **C. Drop the brief** — `mission-audit-chain-watchdog` (shipped via #1270 / `argo-audit::watchdog`) already fires alerts on chain breaks via the existing tamper-notifier callback. Anomaly alerting beyond that is undefined v1 scope.

**My recommendation**: **C (drop)**. The watchdog already alerts on the most critical anomaly (audit-chain break). Broader "anomaly detection" without a clear definition is over-engineering — operators haven't asked for it.

---

## Case 6 — `mission-prometheus-exporter-default-registry`

**Original intent**: wire Prometheus exporter to `argo-telemetry::default_registry()` so operators can scrape SKYCOT metrics with standard tooling.

**Current reality**:
- `/metrics` is **already wired** in `boot.rs:221` via early-dispatch — serves custom Prometheus-text exposition with `argo_sessions_total`, `argo_audit_chain_ok`, `argo_spend_usd_micros_total`, etc. (all derived from store SQL, not the `prometheus` crate).
- `default_registry()` does NOT exist on `argo-telemetry` — telemetry uses a `TelemetrySink` trait + Noop/InMemory/Tracing sinks, not a Prometheus `Registry`.

The "wire to default_registry()" goal is unimplementable as stated.

**Options**:
- **A. Convert existing `/metrics` to use `prometheus` crate** — replace the custom-text encoder with `prometheus::TextEncoder` over a real `Registry`. Preserves the existing metrics; standardizes the implementation. ~250 LOC + workspace dep on `prometheus`.
- **B. Add `/openmetrics` parallel route** — keep custom `/metrics` for backward compat, add a second route using the `prometheus` crate. ~300 LOC.
- **C. Drop the brief** — the existing custom-text `/metrics` already Prometheus-compatible (Prometheus scrapers don't care about implementation detail; they parse the text format). No operator-visible improvement from rewriting.

**My recommendation**: **C (drop)**. The brief's goal (operators can scrape with Prometheus) is already met. Rewriting to use the `prometheus` crate is internal cleanup with no external value. If we ever need `prometheus`-specific features (histogram buckets, label cardinality protection), a different brief targets that explicitly.

---

## Summary table — my recommendations

| Case | Brief | My pick | Resulting action |
|---|---|---|---|
| 1 | judge-dreams-output | **Drop** | Mark MERGED-DUP (TrustGrade ladder already covers) |
| 2 | roster-resolve-policy | **Rewrite onto operator_risk_overrides** | Author v2 brief, ~300 LOC |
| 3 | delegate-task-fallback-chain | **Drop** | Mark BLOCKED-DEFER (stub-on-stub) |
| 4 | llm-call-dedup-cache | **Rewrite to transport.rs target** | Author v2 brief, ~30 min |
| 5 | anomaly-alerting-notifychannel | **Drop** | Mark MERGED-DUP (watchdog covers chain-break alerts) |
| 6 | prometheus-exporter-default-registry | **Drop** | Mark MERGED-DUP (existing /metrics is Prometheus-compatible) |

**Net effect if you accept all 6 recommendations:**
- 4 dropped (mark as MERGED-DUP or BLOCKED-DEFER) → ledger cleanup, no engineering work
- 2 rewritten → ~1-2 hours of conductor work to author v2 briefs

**If you disagree on any case, pick another option above and I'll author the resulting brief.** No rush — these can wait for a convenient 30-min slot with you.

---

## Briefs that were originally Cluster D but ended up in other clusters

For completeness — the agent flagged 16 cases initially; 10 were re-bucketed:

- **Already shipped (3, ledger-fixed in `8ae79fee`):** mission-identity-delete-worker, mission-auto-continue-max-iterations-policy, mission-audit-chain-watchdog
- **Cluster B precursors authored (3 → unblocks 5 DEEP-DRIFTs in `adeb3faf`):** compliance-crate-skeleton, gateway-request-log-schema, plugin-registry-skeleton
- **Cluster C sub-briefs authored (6, replacing 2 oversized parents in `033b046c`):** error-types-rewrite-{io,schema,invariant,flip-ci}, config-{audit-refresh,migrate-v2}

This Cluster D doc covers the genuinely-architectural residue.
