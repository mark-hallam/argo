---
id: DEC-039
date: 2026-06-05
status: active
supersedes: none
amends: none
---

# DEC-039 — Genesis discovery: the conversational interview is an additive "Talk it through" mode, not a replacement

## Decision

Operator authorized (2026-06-05) resolving the open **A3** design question. The
multi-turn conversational interview (brief `uf-genesis-conversation-layer`) ships
as a **new, additive "Talk it through" mode** alongside the existing one-shot
"describe and watch" flow — **NOT a replacement** of it. Both entry points feed
the **same** `IntakeSpec → CapabilitySpec` materialization. Reversible — revisit
if usage data favours consolidating to one.

## Why

- The one-shot describe-and-watch flow + the genesis Phase-2 panel tree (guardrails,
  systems, docs, voice, comprehension) are merged or about-to-unblock; **replacing**
  that flow would waste/fork that work and risk the demo path.
- **Additive is lower-risk**: users who want speed keep the textarea; users who want
  to be guided get the interview. Two front doors, one backend.
- Avoids forking `web/components/skycot/genesis/`.

## How to apply

- `/genesis` offers two affordances: **"Describe it"** (today's textarea + Phase-2
  panels) and **"Talk it through"** (the conversation). Both land on the same
  review → hire.
- The conversation captures responsibilities + the A1 slots (cadence, exceptions,
  success-criteria) by dialogue, with a deterministic wizard fallback on provider failure.
- Reuse the materializer and the Phase-2 panels — do not fork.

Relates: [[DEC-025]] (Genesis core build), briefs A1/A3 in
`docs/skycot/plans/USEFULNESS-FLOW-RECONCILED-BRIEFS-2026-06-05.md`.
