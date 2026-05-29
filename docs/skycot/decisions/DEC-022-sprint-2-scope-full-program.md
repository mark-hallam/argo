---
id: DEC-022
date: 2026-05-30
status: active
supersedes: none
---

# DEC-022 — Sprint-2 scope is the full program (all four tracks + real voice)

## Decision

For Sprint 2 (2026-05-30), the operator (awake, choosing from a menu) selected
the **full program** — all four tracks **plus** real voice:

1. **Go live + verify** — rebuild the gateway + reseed so the features committed
   overnight (voice routes, `/settings`, evals tab, governance readiness, retrain
   recent) become real in the running binary; verify in-browser.
2. **Kill ALL theatre** — finish the remaining mock-in-demo-live surfaces (the
   T-series theatre-removal queue + any remaining fixtures the user can reach).
3. **Multi-worker orchestration — the Supervisor model** — parallel fan-out +
   dependency layers over the existing handoff chain in `argo-orchestrator` (the
   Supervisor engine, *not* a visual DAG; resolves the prior parked
   "Supervisor vs DAG" blocker in favour of Supervisor).
4. **Broad backlog** — work down FEATURE_MATRIX 🟡 Partial / 🟧 Scaffold rows,
   highest value first.

Plus **real voice** ([[DEC-021]] — OpenAI for launch).

## Why

- Operator chose all four tracks + real voice when offered the Sprint-2 menu
  (2026-05-30). This is the concrete scope of the [[DEC-019]] build-out program
  for this sprint.
- It also **resolves two previously-parked blockers**: the orchestration model is
  now decided (Supervisor), and the voice vendor is now decided ([[DEC-021]]).

## How to apply

- Sequencing (per the build-out tracker): the conductor drives tracks 1-2; the
  auto-resume tick + build-agents ([[DEC-020]]) carry 3-5.
- The master tracker [`docs/skycot/overnight-build-out.md`](../overnight-build-out.md)
  holds the live Sprint-2 queue; treat it as the operative work list for this
  sprint.
- All work stays under the build-out discipline ([[DEC-019]]): real + tested +
  committed, NO THEATER, FEATURE_MATRIX + register updated in the same cycle.

Related: [[DEC-019]] (build-out program), [[DEC-020]] (autonomous machine),
[[DEC-021]] (voice = OpenAI), [[DEC-007]] (no theater).
