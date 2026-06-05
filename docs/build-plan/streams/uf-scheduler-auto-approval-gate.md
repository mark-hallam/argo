# uf-scheduler-auto-approval-gate (UF Phase-B · B2) — OPERATOR-GATED

**Owner**: claude.
**Depends on**: B1; A6 (per-worker spend snapshot); [[DEC-036]].
**Authored against**: origin/main `58a35a9f9` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-scheduler-auto-approval-gate`.

## 🔒 HOLD — operator-gated (do NOT flip to TODO until released)
Do not build until ALL of: (1) every Phase A brief MERGED, (2) the live signup→hire→work→approve→pay gate passes, (3) Mark explicitly says "go-live Phase B". Per DEC-036/037, default-off + reversible. Flips AFTER B1. The **auto-approval budget/risk decisions get an adversarial 'try-to-break-it' review** before merge; the `ARGO_DUTY_AUTOFIRE` flag stays OFF until the gate is verified firing (current-turn audit-row evidence).

## Why
The scheduler's gate is a `ClosedGate` (deny-all; re-exported `crates/argo-orchestrator/src/lib.rs:92`). Replace it with a real gate that **reuses the MERGED `auto_approval::evaluate`** (`crates/argo-identity/src/auto_approval.rs:140`) — do NOT author a new policy engine.

## Grep-before-write (verified 2026-06-05)
- `rg 'AutoApprovalPolicy|auto_approval::evaluate' crates` — wire the EXISTING evaluator (auto_approval.rs:140).
- `ClosedGate` — the flag select point in the duty-scheduler boot path.

## Files you may touch
- new `crates/argo-orchestrator/src/duty_scheduler_auto_gate.rs` (`AutoApprovalDutyGate impl DutyGate`: resolve the worker's `AutoApprovalPolicy` + a fresh `SpendSnapshot` by `worker_id` (A6's column), build a `PendingAction` from the duty's `entry_action`, map `AutoApprovalDecision`→`FireDecision`; `Ask`→approvals inbox).
- the duty-scheduler boot flag select: `ARGO_DUTY_AUTOFIRE`, **default OFF**.

## Acceptance
Flag ON + a generous policy → a fire produces a real `run_turn`; a $0 budget → denied + an approval item appears; tenant-isolated; flag OFF = today's deny-all preserved. Adversarial budget/risk review before merge.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (scheduler auto-gate row) + regen HTML.

## Effort
~2 briefs (M–L).
