# eval-argo-agent-runner-turn-lifecycle

**Owner**: claude (evaluator-shape brief — investigation + findings + tests, NOT a runtime fix).
**Depends on**: none.
**Authored against**: origin/main `5a8726465` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-agent-runner-turn-lifecycle`.

🔴 **P0 — LARGEST UNAUDITED MISSION-CRITICAL SURFACE.** Source: evaluator round-8 `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`:

> "`argo-agent-runner` — THE runtime; largest unaudited mission-critical surface, every mission claim routes through it."

`argo-agent-runner` is the load-bearing crate that turns a hire (claim 1) into a turn-by-turn agent loop. Every mission claim flows through it:
- Claim 1 (teammate-not-tool): worker identity + memory loaded per turn.
- Claim 2 (delegation): duties + KPIs become turn objectives.
- Claim 3 (platform builds workers): vertical pack drives system prompt.
- Claim 4 (skills): skills resolved + injected at turn boot.
- Claim 5 (any-PC execution): executor (MCP / computer-use) called from turn.
- Claim 6 (multi-tenant): tenant_id plumbed through every primitive.

Despite being the runtime spine, the crate has never been end-to-end audited. The auditor T21-39 and evaluator round-8 both name this as the single highest-leverage unaudited surface.

## Scope

This is an **investigation brief**, not a runtime fix. Outputs are findings + a brief-shape recommendation per gap. The deliverable is a comprehensive report in `docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md` covering:

### 1. Surface inventory

Walk every `pub` symbol in `crates/argo-agent-runner/src/`. List by category:
- Entry points (e.g. `AgentRunner::new`, `run_turn`)
- Per-turn primitives (memory load, prompt assembly, skill resolution, executor dispatch, audit emission)
- Helper modules (turn_helpers, turn_skills, etc.)
- Test fixtures + harness

For each entry point, trace the call chain end-to-end. Document which mission claims it serves.

### 2. Tenancy audit

For every SQL query in the crate (search via `rg 'sqlx::query|SELECT|UPDATE|INSERT|DELETE' crates/argo-agent-runner/src/`):
- Does it bind `tenant_id`?
- If not, is it scoped via another primitive (e.g. `worker_id` which is itself tenant-scoped)?
- Cross-tenant negative tests for any bare query.

Per memory `feedback_enterprise_grade_defaults.md`: strict defaults are the target architecture. Any unbound query is a finding.

### 3. Audit-chain integrity

For every action the runner takes (executor call, memory write, skill load, decision):
- Is an `audit_record` row written?
- Is it inside the same transaction as the action (per `mission-workflow-audit-write-transactional` shape from #2078)?
- Cross-tenant negative tests on audit emission.

### 4. Skill resolution path

Trace: hire → worker.skills (set) → SkillStore::available_for_worker → turn boot → system prompt injection.
- Does the skill propagation loop (#1384) fire at every turn?
- Are skill min_trust_grade gates honored at runtime (not just declared)?
- Cross-tenant skill leak: does worker_a (tenant_a) ever see skills from tenant_b's worker_b's namespace?

### 5. Memory namespace honoring

Verify per-worker `MemoryNs::WorkerSelf` scope is enforced at SQL bind layer (round-8 cited this as separate P1 gap).

### 6. Executor backend coverage

Per mission claim 5, the runner must dispatch to BOTH executor backends:
- MCP executor (well-known APIs)
- Computer-use (Anthropic-style desktop control, currently macOS-only per v1 strategy)

For each: does the runner call the right entry point? Does the executor's pre-flight enforce tenancy (per #2076)?

### 7. Failure modes

For each "what could go wrong" in §1-6:
- Is the failure recoverable / observable / atomically rolled back?
- Document each as a typed error variant or a `tracing::warn!` (acceptable for advisory failures only).

## Deliverables

ONE comprehensive report at `docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md` — 1500-3000 words, sections per §1-7 above. Each finding includes:
- Severity (P0 / P1 / P2 / P3)
- File + line citation
- Brief-shape recommendation (name + one-paragraph scope)

Optionally: a small set of `#[ignore]`'d cargo tests pinning load-bearing invariants the eval discovered (cross-tenant negatives, audit-chain atomicity, skill resolution per turn). These ship as FINDINGS-ONLY artifacts the next fix-brief un-ignores.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-argo-agent-runner-turn-lifecycle.md` — the report.
- `OPTIONAL NEW: crates/argo-agent-runner/tests/eval_findings_*.rs` — `#[ignore]`'d FINDINGS-ONLY tests (one file per finding tier).
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — append-only updates marking audited rows 🟢.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-agent-runner/src/**` — strict read-only. This is an audit, NOT a refactor. Any production fix lands in follow-up briefs.
- Other crates outside the call chain you're tracing.
- Briefs other than this one.

## Acceptance criteria

- [ ] Report covers all 7 sections.
- [ ] Every finding has severity + file:line + brief-shape recommendation.
- [ ] At least 1 cross-tenant negative test added as `#[ignore]`'d FINDINGS-ONLY (or note explicitly if surface is already proven tenant-scoped).
- [ ] `mission-vs-code-gap-manifest.md` updated: argo-agent-runner row 🔴 → 🟢 (with audit citation).
- [ ] Cost target: 60-120K tokens (this is a deep audit).
- [ ] Each finding's recommended brief is actionable + scoped <500 LOC.
- [ ] BUILD_STATUS.md updated with the report path (no PR — this is doc-only).

## Branch + PR

- Branch: `eval-argo-agent-runner-turn-lifecycle`
- PR title: `eval(agent-runner): turn-lifecycle audit — 7-section findings report (eval-argo-agent-runner-turn-lifecycle)`

## Notes for the implementer

- ~200-500 LOC of report content + 100-300 LOC of optional FINDINGS-ONLY tests.
- **Anti-theater**: a "we read it, looks fine" report with no actionable findings is a failure. Each section MUST surface at least one concrete gap or one concrete proof-of-correctness (with file:line).
- **This is the round-8 highest-leverage unaudited surface.** Operator + auditor + evaluator all flagged it. Take the time to get it right.
- After this brief lands, the conductor will queue 3-7 follow-up fix-briefs from the recommendations — that's the multiplier.
- Cite auditor T21-39 + evaluator round-8 manifest verbatim in the PR body.
