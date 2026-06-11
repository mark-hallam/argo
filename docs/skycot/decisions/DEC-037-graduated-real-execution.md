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

## ⛔ Before `ARGO_EXECUTION_GRADUATION` is flipped ON in production (added 2026-06-11)

The gateway Run-now executor wiring (DEC-050 G2, #2814) is merged but ships
**default-OFF** behind this flag. Adversarial review confirmed the execution path
preserves the capability fence, tenant scoping, audit chain, and approvals. These
are BINDING prerequisites on the operator flag-flip (NOT on the merged wiring):

1. **Executor dispatch timeout.** `executor.dispatch` is a synchronous trait call
   on the async gateway task with NO timeout/cancellation. Today's executors are
   quick, but when the real computer-use / E2B run-loop lands behind this trait a
   hung dispatch would block the gateway worker. Wrap in `tokio::time::timeout` +
   `spawn_blocking` before the flip.
2. **Inline-path tenant + fence tests.** Today the inline executor's tenant
   scoping and worker-fence binding are proven by code-trace (MockExecutor), not
   asserted. Add a test that a tenant-X Run-now cannot resolve/mutate a tenant-Y
   action, and that the executor runs under the worker's least-privilege profile.
3. **Close the `resolve_workspace` cwd fallback** (`routes_worker_run_exec.rs`):
   when a tenant has zero workspaces it falls back to a cwd-derived workspace,
   discarding `worker.tenant_id` — a tenant-binding smell to close when arming.

4. **Computer-use approval floor — ✅ RESOLVED via [[DEC-051]] (cu-risk-tiered-computer-use-floor).**
   The risk was: the inline auto-execution gate (`!requires_approval && risk==low`)
   stamps those fields verbatim from the manifest, so a manifest could self-declare
   an autonomous, no-approval `computer_use_native_macos` action and drive the host
   Mac with NO approval the moment this flag flips. The operator chose risk-tiering
   (DEC-051) over manifest-validation: the floor is now bound to the OPERATION's
   reversibility and enforced **in the macOS CU run loop** (`cu_risk::classify` +
   the `act_approved` gate in `executor_loop.rs`; shipped #2822). AUTONOMOUS ops —
   observe (screenshot/cursor-read/OCR/enumerate/wait) PLUS navigate/STAGE input
   (scroll, cursor-move, `type`) — run on their own; the loop returns `HumanHandoff`
   at the FIRST committing ACT op (click/double_click/`key`/hold_key incl Enter-as-
   submit/drag/navigate/app-launch/mutating-osascript) unless the request is
   explicitly approved — regardless of the manifest's `requires_approval`/`risk`.
   Typing is autonomous ONLY because the SUBMIT (`key`/click) stays ACT, so staged
   text is reviewable before any irreversible commit (operator tiers 2026-06-11).
   Both the CLI auto-dispatch path and the gateway inline cu-macos path construct
   the request with `act_approved=false`, so neither can self-approve host control.
   Unblocked cu3/#2816 (register the macOS executor — now MERGED). **Residual (smaller follow-up,
   NOT a flip-blocker):** non-macOS computer-use executors (E2B) are not on the v1
   path and are not yet covered by an `is_auto_eligible_with_flag` belt-and-suspenders;
   add that floor if/when an E2B cu executor is wired to the inline run path.
5. **Tenant-blind cu-macos audit (pre-existing).** The inline path runs cu-macos
   via `MacosCuExecutor::new_unguarded` with `TenantId(nil)`, so its audit rows
   land under the nil tenant rather than the real one. Single-host, limited blast
   radius, but cu-macos audit attribution should be tenant-bound before the flip.

Same posture as DEC-049's `ARGO_WORKER_SWARM` flip-gate: default-off, operator-only
flip after these land + a fresh adversarial re-verify.

Relates: [[DEC-025]] (#6 deliberate post-hire grant — operationalized here; #2
probation), [[DEC-028]] (marketing draft-only stays), [[DEC-007]]/[[DEC-029]]
(no theater / truth-bound). Phase B of the usefulness plan.
