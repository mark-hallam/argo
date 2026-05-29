---
id: DEC-017
date: 2026-05-29
status: active
supersedes: none
---

# DEC-017 — Teammate-performance eval harness: synthetic golden-companies first; historical replay is a later seam

## Decision

Build the AI-teammate performance eval harness (item-6 of the 6-item program)
with **two scenario-source modes**, but ship them in this order:

1. **Synthetic "golden companies" FIRST** — committed, reproducible fixtures
   (seeded company context + scenario banks with known-correct expected
   behaviours). Built now.
2. **Historical replay LATER (a seam, not built yet)** — replay a real
   customer's past data (tickets/CRM/emails) and score the teammate against
   the human's actual outcomes. The strongest signal, but needs real customer
   data we don't have yet (no design-partners live; privacy-gated). Build the
   pluggable seam now, switch on once we have a real customer.

**Reference teammate = Portia** (the Pitch Agent, [[DEC-016]]) — she already
has the mature 141-case investor-Q&A suite, so she proves the scorecard
pattern. **Mariah** (marketplace) is the first *synthetic-company* teammate.

## Why

- The operator framed the harness as "backbone tech that ensures the AI
  teammates we build can actually do real-world work" and intuited both data
  sources (historical + simulated). Confirmed 2026-05-29.
- Synthetic-first is reproducible → safe for regression gating; historical is
  higher-fidelity but unavailable + privacy-sensitive today.
- Synthetic-data generation pattern: **LLM (Opus) drafts the company +
  scenarios → ratify the "correct answers" once → freeze as committed
  fixtures.** Realism without sacrificing reproducibility.

## How to apply

- Layer the harness ON TOP of the existing `argo-eval` runner + `argo-demo-seed`
  — do not build a new engine.
- Score *behaviours*, not just strings: correctness, escalation (both
  directions), boundary adherence (reuse pitch `evaluate_claim`), skill usage,
  honesty (reuse `argo-eval::honesty`), latency/cost budget (reuse
  `argo-forge::cost`).
- Treat full rollout as **post-MVP** (build after the demo MP4 records); the
  phase-1 scaffold + Portia reference can land now.
- Design doc: `docs/skycot/evals/TEAMMATE_PERFORMANCE_EVAL_HARNESS_DESIGN.md`.

Related: [[DEC-016]], [[project_pitch_agent_v1_demo]].
