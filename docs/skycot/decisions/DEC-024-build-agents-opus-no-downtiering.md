---
id: DEC-024
date: 2026-05-30
status: active
supersedes: none
---

# DEC-024 — Conductor build-agents run on Opus 4.8; no task-based down-tiering

## Decision

Every Claude Code build-agent the conductor spawns runs on **Opus 4.8** (the
session / frontier tier). The conductor proposed routing *trivial mechanical*
tasks (e.g. a one-line `#[allow(...)]` fix) to Sonnet for speed/cost while
keeping Opus for judgment-heavy work; the operator declined — **"opus for all is
the call"** (2026-05-30). Quality + drift-avoidance over speed/cost.

## Why

- Judgment quality has been load-bearing this session: the P1 finisher agent
  *refused to fake a change* when its brief's premise was wrong (verified +
  corrected the ledger instead); the kpi-rollup agent looked up the exact
  `audit_kind` string byte-for-byte rather than guessing. A cheaper tier is more
  likely to take the lazy/literal path.
- At this stage (pre-seed demo, NO-THEATER discipline) correctness matters more
  than token cost. Revisit only if cost becomes a real constraint — via a new DEC.

## How to apply

- When spawning a build-agent, do **not** pass a `model` override to Sonnet/Haiku.
  Omit `model` (the agent inherits the Opus session) or pin Opus explicitly.
- This is distinct from **[[DEC-013]]** (Sonnet floor) and **[[DEC-014]]** (model
  catalog), which govern the **SKYCOT product's** worker model-routing — *not* the
  conductor's own coding sub-agents. DEC-024 governs the latter.

Related: [[DEC-023]] (operating model — conductor + on-demand agents),
[[DEC-013]] / [[DEC-014]] (product-side model policy, separate concern).
