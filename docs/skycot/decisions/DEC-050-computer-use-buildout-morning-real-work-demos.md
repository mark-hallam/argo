# DEC-050 — Computer-use buildout is the active focus; morning demos of teammates doing REAL Mac work

- **Date:** 2026-06-11
- **Status:** **active** — operator-locked 2026-06-11 (morning): *"the next
  stage would be to ensure we build the computer use out on mac, so audit
  what is built, work out what we need so our ai teammates can get all the
  real work get done, thats what we must concentraite on. i want to see in
  the morning demonstrations of our ai teammates doing real work… running
  live in the chrome browser as real work no theatre."*
- **Owner:** conductor + swarm; demo execution on the operator's Mac.
- **Builds on:** [DEC-002](DEC-002-macos-first-computer-use.md) (macOS-first),
  [DEC-009] (computer-use in MVP), [DEC-040](DEC-040-next-batch-authorization.md)
  (Phase B built default-OFF), [DEC-041](DEC-041-execution-posture-risk-tiered-act-by-default.md)
  (risk-tiered act-by-default, live flip operator-only),
  [DEC-048](DEC-048-genesis-interview-depth-over-speed-fidelity-contract.md).

## 1. The directive

1. **Audit** the computer-use surface as built (executor crate ↔ runtime
   dispatch ↔ worker actions ↔ approval gates): Live / Partial / Scaffold,
   evidence-based.
2. **Close the gap** between "teammate minted" and "teammate does real Mac
   work": whatever wiring, binding, or permissions stand between a hired
   teammate and real, observable work on this Mac.
3. **Morning deliverable:** live demonstrations, driven from the Chrome SPA,
   of AI teammates doing REAL work (real files moved / real app actions /
   real artifacts on disk) — DEC-007/DEC-029 NO THEATER: every demo step
   verifiable (artifact, audit row, screenshot), nothing mocked.

## 2. Guardrail interpretation (DEC-040/041 compatibility)

Code defaults stay OFF. Enabling real execution FOR THE DEMOS on the
operator's own machine — via env flags / per-worker grants — is exactly the
"operator-only live flip" those DECs reserve, exercised here by this
directive. Any flip is machine-local (env/config), never a repo default
change; approval-gated verbs stay approval-gated IN the demos (the approval
moment is part of the demo, not an obstacle to it).

## 3. Demo bar (what counts as "real work")

- A real artifact a skeptic can inspect after (file moved/created, app state
  changed, audit row hash-chained).
- Driven through the product (Chrome SPA: hire → run/schedule → approvals →
  audit), not via CLI seams the camera can't see.
- The teammate's claims truth-bounded: anything it says it did must be
  checkable in the same demo.
