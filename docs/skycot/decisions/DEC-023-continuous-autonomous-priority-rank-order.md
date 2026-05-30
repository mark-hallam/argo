---
id: DEC-023
date: 2026-05-30
status: active
supersedes: none
---

# DEC-023 — Continuous autonomous execution through a fixed P1–P5 priority order; operating model = conductor + on-demand agents (swarm-daemon retired for now)

## Decision

The operator locked **two** things on 2026-05-30:

### 1. The priority rank order (work top-down, continuously, to completion)

This is the operative execution order for the [[DEC-019]] build-out program. The
conductor + auto-resume machine works it **top-down, continuously, until done** —
finish the highest band before dropping to the next; a blocker that needs operator
input is marked PARTIAL/BLOCKED and the next item is picked up (never idle).

- **P1 — Kill the theatre on camera-visible screens.** Per-screen audit of the
  ~24 SPA data modules that still fall back to mock data: wire each to its real
  backend endpoint where one exists; where none exists, render an honest empty
  state, never faked data. Folds in the live queue bugs that are themselves
  theatre — notably the **P0** onboarding name-parser (`name_from_description`
  names every demo hire "A") and the workflow-detail Run-now disabled bug. The
  biggest "is it real?" lever; resume point from the Portia session.
- **P2 — The two "no competitor does this" wow moments, end-to-end.**
  (a) **Voice loop**: TTS is real (OpenAI, [[DEC-021]]); finish STT + a talk
  button on Portia's page so you speak and she answers in voice, truth-bounded.
  (b) **Genesis end-to-end** in the live app — describe a role → a hired teammate
  appears; backend + page exist, the gap is full live verification + polish.
- **P3 — Real model-backed delegation in orchestration.** The Supervisor engine
  ([[DEC-022]]) makes the topology real; each delegated step still returns a
  placeholder acknowledgement (an honest seam). Swap in a real model-backed
  executor so "watch the team do the work" is genuine output.
- **P4 — Close the self-improvement loop.** Pieces shipped (generate cases,
  export training data, retrain history); close it by A/B-testing a skill and
  showing it measurably improve.
- **P5 / v1.x — Enterprise governance tail.** SSO (SAML/OIDC), SCIM, BYOK,
  data-residency. Table-stakes to *close enterprise sales*, not to *win the
  funding round* — deferred to v1.x. Informed by the competitive gap research.

### 2. The operating model (how we run it)

The **default operating model is locked to the "new way"**: an interactive
conductor (human-in-the-loop, operator-aligned priority) + **on-demand,
worktree-isolated, self-committing build-agents** + the launchd auto-resume tick
([[DEC-020]]). The **old always-on swarm-daemon** (worker-supervisor LaunchAgent +
worker bash loops + auditor/evaluator/genesis/watchdog crons) stays **retired for
now**; its halt sentinels (`~/.argo/stop`, `/tmp/argo-stop`) remain in place.

- **Auditor + evaluator are kept as on-demand adversarial passes**, not 24/7
  daemons — spun up for a high-stakes claim or merge, not every PR.
- **Genesis is a product feature**, not a build daemon — keep building it (P2).
- **Bring a bounded swarm back ONLY** for a large, mechanical, low-ambiguity
  backlog (e.g. "wire these 24 screens to live endpoints, one brief each") where
  throughput matters and drift risk is low. For exploratory/decision-heavy/
  demo-critical work, conductor + on-demand agents wins.

## Why

- **The stage suits it.** A pre-seed funding / waitlist demo rewards correctness,
  honesty, and operator-aligned priority over raw PR throughput. The swarm's
  strengths (24/7 volume, self-replenishing queue) aren't what this stage needs.
- **The swarm's weaknesses are exactly the recurring pain** — documented in this
  repo: ~24% brief intent-drift (parallel/duplicate surfaces), theatre
  (scaffolds with no callers), source-of-truth-ledger lag (caught 3×), RAM-
  pressure build crashes, and heavy coordination tax (merge-trains, rebase
  waves, stale-claim sweeps). The new model structurally reduces all of these.
- **Proven harness limits** (this session): ~3 concurrent heavy Rust builds is
  the safe ceiling on this machine; file-partition by independent crate gives
  clean parallel rebased pushes; gateway-dispatcher builds serialize.

## How to apply

- The master tracker [`docs/skycot/overnight-build-out.md`](../overnight-build-out.md)
  holds the **PRIORITY RANK ORDER (DEC-023)** section as the live continuous-work
  queue the auto-resume tick reads each tick. Work it top-down.
- Every item stays under the build-out discipline ([[DEC-019]], [[DEC-007]]):
  real + tested + committed, NO THEATER, and — per the `CLAUDE.md` "## Maintain"
  rule — the FEATURE_MATRIX row updated in the same commit + any new operator
  lock written as a DEC in the same turn.
- Do **not** relight the worker-supervisor swarm without a fresh operator call;
  if a future request implies "turn the swarm back on," confirm against the
  bounded-mechanical-backlog trigger above.

Related: [[DEC-019]] (build-out program), [[DEC-020]] (autonomous machine +
auto-resume), [[DEC-021]] (voice = OpenAI), [[DEC-022]] (Sprint-2 full-program
scope — this DEC sequences it), [[DEC-007]] (no theater).
