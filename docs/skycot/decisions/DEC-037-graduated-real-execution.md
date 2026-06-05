---
id: DEC-037
date: 2026-06-05
status: active
supersedes: none
amends: DEC-025 (operationalizes point #6 — does NOT reverse draft-only-at-birth)
---

# DEC-037 — Graduated real execution: a proven teammate acts for real via the deliberate post-hire grant

## Decision

Operator greenlit (2026-06-05) the **"deliberate post-hire grant"** path that
[[DEC-025]] point #6 already named — letting a **proven** teammate graduate from
draft-only to **actually executing** an action (send / post / file) via a real
executor (MCP / computer-use), **per-action, after probation, with approval
retained on sensitive verbs**.

This **extends, does not reverse, [[DEC-025]]**: teammates are still
**draft-only at birth** ([[DEC-025]] #6 first clause — the `executor="human"`
default stays) and integrations are still **never auto-bound at birth**. The
new commitment operationalizes the second clause ("deliberate post-hire grant")
as an explicit, audited graduation step.

## Why

Verified 2026-06-05: the real executors are merged but never reachable from a
hired teammate (every generated action materializes as `executor="human"`).
[[DEC-025]] #6 always intended a deliberate post-hire grant; without it, a
teammate can only ever draft and a human does all execution forever — which is
not "the platform builds workers that work." Graduation closes that gap while
keeping the safety that makes Genesis trustworthy.

## How to apply

- **Birth unchanged**: draft-only, no auto-bound integrations ([[DEC-025]] #6
  first clause; [[DEC-028]] marketing "no autonomous posting" still holds).
- **Graduation precondition**: teammate past the 24h probation ([[DEC-025]] #2)
  **and** a track record of approved drafts.
- **Promotion is per-action, opt-in, operator/RBAC-gated**: it rewrites that one
  action's `ExecutorDeclaration` from `human` → `{mcp,computer_use}`
  (`manifest_actions_executor.rs` tagged form); every promotion is an audit row.
- **Approval STAYS** on sensitive verbs (send / pay / post / delete) — promotion
  changes **who executes after approval** (tool vs human), it never removes the
  approval requirement or the dangerous-autonomous-capability fence.
- Brief: `uf-execution-graduation-and-promotion`.

Relates: [[DEC-025]] (#6 deliberate post-hire grant — operationalized here; #2
probation), [[DEC-028]] (marketing draft-only stays), [[DEC-007]]/[[DEC-029]]
(no theater / truth-bound). Phase B of the usefulness plan.
