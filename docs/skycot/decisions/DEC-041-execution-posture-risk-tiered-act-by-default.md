# DEC-041 — Execution posture: risk-tiered act-by-default

**Date:** 2026-06-06
**Status:** active
**Operator call:** "b" (chose option B from the conductor's execution-posture evaluation, 2026-06-06)

## Context

The Phase-B adversarial blast-radius review (`docs/skycot/PHASE-B-BLAST-RADIUS-REVIEW-2026-06-06.md`) confirmed the merged execution machinery (B1/B2/B3) is safe but **inert** — defaulted to "deny-all / draft-only," so a hired teammate never actually *does* anything autonomously. The conductor's usefulness evaluation surfaced that this is a **posture** problem, not an architecture problem, and that the goal-measurement layer (already wired: hire `success_criteria` → scored KPI targets) is **starved** — it has no real *actuals* to score because the teammate only drafts.

The operator was presented two options:
- **A** — keep draft-only-until-manually-graduated (safest, enterprise-credible, but the "teammate that works" promise stays perpetually gated).
- **B** — risk-tiered act-by-default (the teammate *does* the safe, reversible work autonomously; risky verbs stay gated).

**The operator chose B.**

## Decision

Evolve the uniform "every generated action is `executor="human"` / draft-only at birth" model ([DEC-025](DEC-025-genesis-done-right-full-synthesis.md) #6) into a **risk-tiered** posture:

- **Safe tier — auto-act.** Actions classified **low-risk AND not `requires_approval`** (the conservative proxy for "low-risk + reversible") auto-execute within a tight, auto-enforced envelope: per-worker spend cap, rate limit, scoped permissions. Every step writes a hash-chained audit row and is reversible.
- **Sensitive tier — stays gated.** Anything **high/medium-risk, `requires_approval`, irreversible, or public-facing** (send / post / pay / delete) stays approval-gated exactly as today.

The differentiator (tamper-evident audit + truth-boundary + approval-on-sensitive + instant-undo) is what *makes* act-by-default safe — act-by-default is only trustworthy because every action is visible and reversible.

## Boundaries (load-bearing — these do NOT change)

1. **Built default-OFF + demoable first.** The risk-tiered behaviour ships behind the existing flags (`ARGO_DUTY_AUTOFIRE` + `ARGO_EXECUTION_GRADUATION`), both unset by default. It is verifiable in a sandbox **before** any live enablement.
2. **The live flip stays operator-only.** Enabling autonomous execution on the operator's real Mac / systems is the operator's explicit call, **after** the blast-radius review is re-run against the now-*wired* path (the review found the read-seam currently unwired). The conductor never flips it.
3. **DEC-028's red line holds.** No autonomous **public posting** — public/irreversible actions are the sensitive tier and stay approval-gated. B does not loosen this.
4. **Reversibility is a precondition.** An action auto-executes only if it is classified safe-and-reversible; the classification fails closed (unknown → treated as sensitive → gated).

## Consequences / what this operationalizes

- **[DEC-036](DEC-036-autonomous-scheduled-execution.md)** — the `AutoApprovalDutyGate`'s default workspace policy becomes the risk-tiered one (auto-approve safe tier, hold sensitive), instead of the disabled "ask everything" default.
- **[DEC-037](DEC-037-graduated-real-execution.md)** — graduation auto-applies for the safe tier at hire (no manual per-action grant needed for low-risk-reversible actions); the manual deliberate grant stays for the sensitive tier. This is the wire of `resolve_executor_with_promotion` into live dispatch, flag-gated.
- **[DEC-025](DEC-025-genesis-done-right-full-synthesis.md) #6** — evolved from "all actions draft-only at birth" to "safe-reversible actions act, sensitive actions draft+ask." Not reversed wholesale; tiered.
- Feeds the measurement layer real actuals so the goal-vs-actual scorecard lights up.

## Build sequence (all default-OFF; conductor-built; operator does the live flip)

1. Risk-tiered default auto-approval policy (gate side) — auto-approve `low && !requires_approval`, hold else. **+ B2 risk fidelity** (carry the action's real `risk_level` onto `DutyRecord` so the gate tiers on truth, not the flattened `"medium"`).
2. Wire `resolve_executor_with_promotion` into live dispatch, flag-gated (dormancy preserved when off) — so a safe-tier action executes via a real executor instead of drafting.
3. Safe-tier auto-graduation — **direction locked "A then B" (operator, 2026-06-06).** Finding during piece-2 build: the conservative `Builtin` intent ("side-effect-free read/draft/clarify") has **no executor** — it materializes to `human` (draft), so it cannot execute. The only Genesis-proposed intent with a real executor is external `Mcp`. So the enterprise trust-ladder is built bottom-up:
   - **(A) FIRST — internal-safe executor:** give the safe tier real reach with **zero external blast radius** — the teammate produces an internal document/summary (`DocumentCreation` executor exists) + writes its own memory, and marks the action complete (the artifact IS the deliverable; self-completing). Auto-graduated via the flag-gated grant path. This is the enterprise-adoptable foundation + generates the audit history that justifies (B).
   - **(B) LATER — external `Mcp`, graduated:** per-action, post-probation, approval retained on anything mutating/sensitive — never an auto-default. Arrives after (A) has proven the teammate via audit history.
4. The tight envelope (spend cap + rate limit + scope) enforced on the auto-act path.
5. Re-run the blast-radius review against the wired path → operator live-flip → verify with a real executed-action audit row (NO THEATER).

WS1 (principal-vs-cwd tenant resolution) remains a pre-live-flip dependency.

## ⛔ Blast-radius re-review against the wired path — 2026-06-06 — FLIP BLOCKED

Operator authorized the flip ("you can flip the switch" / "run the finish"). The
conductor-driven re-review (operator watching) ran FIRST and **blocked it** — the
review worked. Findings:

1. **No-grant fallback returns the manifest executor (the dangerous one).** Piece-2's
   `resolve_executor_with_promotion`, on the no-grant path, returns the *manifest*
   executor. 3b only graduates NEW hires to the safe `document` executor — existing
   seeded workers have no grant, so a safe-tier duty fire would resolve their actions
   to whatever the manifest declares, **including external `mcp`**.
2. **3 live manifests mis-tag external actions as safe-tier** (low-risk + no-approval +
   external executor): `healthcare-intake/schedule_appointment` (gcal), `real-estate-agent/schedule_viewing`
   (gcal), `real-estate-agent/follow_up_lead` (gmail.send). Under the flags these would
   auto-create calendar events / send email — NOT internal documents.
3. **Unconfirmed auto-execution reachability:** the duty runner's `run_turn` invokes
   `argo_agent_runner::run_turn` with `MAX_TOOL_ROUNDS`, so a fired duty's agent *can*
   call tools within the turn. Whether those tool calls auto-execute via the executor
   path (reaching the manifest-external fallback) or only record proposals is NOT yet
   confirmed — and that uncertainty alone blocks a safe flip.

**Required before any flip (the fix):**
- **(A) Architectural defense:** the AUTONOMOUS path (duty-fired) must require an explicit
  grant to execute — no grant → DRAFT, never the manifest's external executor — regardless
  of the graduation flag. (Distinguish autonomous vs manual dispatch, or force the duty
  path through grant-required resolution.) This is the robust fix; it protects against any
  present/future mis-tagged manifest.
- **(B) Data:** correct the 3 mis-tagged manifests to `requires_approval = true`.
- **(C) Confirm** whether `run_turn`'s tool calls auto-execute or only propose — trace before flip.

Then re-run THIS review → clean → operator flips → verify a real internal-document
executed-action audit row. The flip remains operator-only; nothing was flipped.

### Deeper finding (same re-review): the autonomous surface is the agent TOOL engine, not (only) the action path

Tracing the duty path: a fired duty → `GatewayDutyAgentRunner::run_turn` →
`argo_agent_runner::run_turn(..., MAX_TOOL_ROUNDS)` → the agent runner's OWN tool
dispatch (`argo-agent-runner/src/dispatch.rs`): a `CapabilityEngine`-gated tool
registry of builtin tools + MCP tools via `McpClientPool`. This is a **separate
execution path** from the worker-action executor lifecycle
(`worker_actions_dispatch` → `resolve_executor[_with_promotion]`) that pieces 1–3
gate. So flipping `ARGO_DUTY_AUTOFIRE` lets a fired duty's agent call its
**capability-gated tools directly** — the blast radius is whatever the
`CapabilityEngine` defaults + the registered MCP tools allow for an autonomous
duty session, which is NOT yet analyzed.

**Implication:** the risk-tiered gate + the document-graduation (pieces 1–3) gate
the *action* lifecycle, but the duty-fire→agent-tool path needs its OWN blast-radius
analysis + gating before any flip. Required additional work (pre-flip):
- **(D) Analyze + constrain the autonomous agent-tool surface:** verify what the
  `CapabilityEngine` permits for an autonomously-fired duty session (can it reach
  external MCP tools like gmail.send?), and gate it so an autonomous turn cannot call
  external/mutating tools without an explicit grant — the same "no grant → no external"
  principle as fix (A), applied to the agent-tool path.

The flip stays blocked until (A)+(B)+(C)+(D) land and this review re-runs clean.
