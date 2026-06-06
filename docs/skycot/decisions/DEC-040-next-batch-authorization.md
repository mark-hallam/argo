---
id: DEC-040
date: 2026-06-06
status: active
amends: authorizes the BUILD of DEC-036 + DEC-037; relates DEC-025, DEC-038
---
# DEC-040 — Next-batch authorization: Phase B (build default-OFF), Phase C subset, demo-tenant fixes

## Decision
Operator (2026-06-06, in-session) greenlit the next batch across three tracks:

1. **Phase B — computer-use REAL execution: BUILD default-OFF now.** Un-gate the
   operator-HOLD on B1 (`uf-scheduler-hire-roleduty-bridge`), B2
   (`uf-scheduler-auto-approval-gate`, DEC-036), B3
   (`uf-execution-graduation-and-promotion`, DEC-037) and build them — **all behind
   default-OFF flags** (dormant, reversible). Authorizes the BUILD only; does NOT
   enable autonomous execution. **The live flip (`ARGO_DUTY_AUTOFIRE` / a teammate
   actually acting on the Mac) stays operator-only, after a conductor adversarial
   review of each PR.** DG-3 (spend attribution) already resolved via DEC-038.

2. **Phase C — deep Genesis onboarding: FOCUSED SUBSET first.** Build
   `mission-genesis-document-upload-and-memory-ingest` +
   `mission-genesis-comprehension-verification-quiz-mode` (the "it actually learned
   my SOP" moment). Defer voice-input, integration-import, access-controls/guardrails.
   (Comprehension-quiz dep re-scoped to doc-upload only; cascade-unblock when
   doc-upload merges.)

3. **Demo-tenant fixes: authorized to BRIEF.** Author + enqueue the
   tenancy/RBAC/pricing-adjacent fixes (demo tenant → enterprise tier; isolate
   demo-mint tenants; worker-detail permission-override gap); swarm builds;
   **operator reviews the PRs before merge.**

## Why
Phase A of the usefulness plan is complete + deployed (interview, rubric, KPIs,
scorecard, cost, A3 conversation). Phase B is the remaining mission gap ("real work
on the Mac"); its gates DG-1/DG-2 were greenlit in principle (DEC-036/037) — this
authorizes the BUILD (default-OFF, reversible) while reserving the irreversible
live-flip for the operator. Phase C subset deepens the Genesis differentiator
without blocking on execution. Demo-tenant fixes make the live showcase able to
demonstrate hiring/managing.

## How to apply
- Conductor un-gates B1 + doc-upload → TODO (re-stamped); B2/B3 + comprehension-quiz
  stay BLOCKED with CASCADE-UNBLOCK on their dep merging.
- Every Phase-B PR gets a conductor adversarial review before merge; the live flip
  is a separate operator go-live (NOT in this authorization).
- Demo-tenant fix PRs: operator reviews before merge.

## Relates
DEC-036 + DEC-037 stay the design ADRs (their live-enable is a future operator
decision). DEC-038 (DG-3 resolved). DEC-025 #6 (draft-only-at-birth preserved).
