# Phase B (autonomous execution) — adversarial blast-radius review

**Conductor-driven, operator-observed. 2026-06-06. Code-grounded against merged `main` (B1 #2730, B2 #2733 `8a919003f`, B3 #2735 `a0c02c6f1` + schema fix `c84ec73`).**

Per DEC-037/DEC-040: this is the blast-radius review that must pass **before** the operator-only live flip of autonomous execution. It asks one question: **can a teammate's action execute autonomously (drive the Mac / call a real MCP tool) when it shouldn't?**

## Verdict: ✅ CONTAINED — no current blast-radius hole

With the shipped defaults (both flags unset — verified absent from `~/.argo/env.local` and the live gateway process env this session), **zero autonomous execution is possible.** Every path that could execute is gated, and the most dangerous path (B3 graduation) is additionally *unwired*. The merged capability is dormant by construction, not just by configuration.

## Attack vectors probed (8 + B1) — all closed

| # | Vector | Result |
|---|--------|--------|
| 1 | **Auto-fire flag bypass (B2)** | `build_gateway_duty_scheduler` constructs `AutoApprovalDutyGate` **only** when `duty_autofire_enabled()`; else `ClosedGate`. No construction path skips the flag. ✅ |
| 2 | **Default gate** | `ClosedGate::evaluate` returns `FireDecision::DeniedGate` unconditionally — true deny-all. ✅ |
| 3 | **Probation bypass (B3 route)** | `post_executor_promote` calls `is_in_probation(worker.persona.provenance_chain, now)` → 409 + `RbacDeny` audit, before any write. Worker loaded server-side from the store, so provenance can't be forged via the request. ✅ |
| 4 | **RBAC bypass (B3 route)** | `enforce_role(WorkerExecutorPromote)` (owner-only) → 403 + `RbacDeny` audit, before the write. ✅ |
| 5 | **Approval bypass** | Promotion changes *who* executes **after** approval; the grant carries no approval-clearing field and the honor-seam never touches `requires_approval`. Approval stays on sensitive verbs. ✅ |
| 6 | **Tenant isolation (B3)** | Grant is tenant-stamped; the read-seam `executor_promotion::get` is tenant-scoped (a grant for tenant A is never returned for tenant B). *Which* tenant a request resolves to uses the cwd model — see pre-flip dependency #2. ✅ (for B3 itself) |
| 7 | **Draft-only fence + B3 wiring** | **Double-locked.** (a) `ARGO_EXECUTION_GRADUATION` default-OFF → honor-seam is byte-for-byte `resolve_executor`; (b) `resolve_executor_with_promotion` has **zero production callers** (verified by grep) — the seam is `#![allow(dead_code)]`, not wired into live dispatch. Production dispatch (`worker_actions_dispatch.rs:204`) calls the safe `resolve_executor` → manifest executor → `human` → draft. **Even with both flags flipped, no execution occurs** because the read-side wire does not exist. ✅ (strongest containment) |
| 8 | **Autonomous trigger surface** | The duty scheduler is the only background loop that dispatches worker actions, and it spawns `run_turn` **only** on `FireDecision::AutoApprove`. Other loops (alerts, cu-screenshot-sweep) don't dispatch. So `ARGO_DUTY_AUTOFIRE` is the master gate for *all* autonomous execution — including manifest-declared `cu_macos`/`mcp` actions, not just B3-graduated ones. ✅ |
| B1 | **Hire→role_duty bridge** | Runs on every hire but mirrors **only `default_enabled=true`** duties, tenant-scoped. Harmless because B2's `ClosedGate` denies every fire by default. ✅ |

## Pre-live-flip gating checklist (what must be true BEFORE flipping)

These are **not** current-state holes (the dormant capability is safe). They are the preconditions the operator's eventual go-live depends on. The live flip is **two** flags AND a wire-brief — not just env vars.

1. **B3 wire-brief must land first.** `resolve_executor_with_promotion` must be wired into the live dispatch callsite. Until then, flipping `ARGO_EXECUTION_GRADUATION` does nothing — graduation is inert. (This is the deliberate next brief; it should ship with its own focused review.)
2. **WS1 principal-tenant resolution.** The promote route (like the canonical hire route) resolves tenant from cwd, not the request principal's JWT `tnt`. This is the platform-wide cwd-vs-JWT keystone (WS1), not a B3 regression. Land principal-aware resolution before the flip so promotion is correctly per-principal-tenant-isolated.
3. **B2 risk fidelity.** The duty gate flattens every fire to `"medium"` risk (`DutyRecord` carries no risk metadata). When `ARGO_DUTY_AUTOFIRE` goes live, a genuinely high-risk scheduled action could auto-fire its turn under a generous policy. Carry the action's real risk level onto `DutyRecord` before flipping.
4. **Flag-parser consistency (minor).** `graduation_flag` accepts only `1/true/TRUE/yes` (case-sensitive, no trim, no `on`), unlike `parse_autofire_flag` (case-insensitive, trimmed, `+on`). Errs OFF (safe) but is a footgun: an operator setting `ON` or ` true ` would silently leave graduation dormant. Align before the flip.

## Sequence for go-live (operator-gated, in order)
land wire-brief (#1) → land WS1 (#2) → fix B2 risk fidelity (#3) + parser (#4) → re-run this blast-radius review against the wired path → operator flips `ARGO_DUTY_AUTOFIRE` + `ARGO_EXECUTION_GRADUATION` → verify with a current-turn fired-duty audit row AND executed-action audit row (NO THEATER).

**No flag was flipped by this review. The live flip remains operator-only.**
