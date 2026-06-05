# uf-genesis-capture-cadence-exceptions-success (UF Phase-A · A1)

**Owner**: claude.
**Depends on**: tenant keystone `sprint-ws1-tenant-resolve-from-session-jwt-not-cwd` (#2676, MERGED); genesis Phase-2 SPA panels (capture-panel pattern).
**Authored against**: origin/main `2264560a2` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-capture-cadence-exceptions-success`.

## Why
The genesis Phase-2 intake captures guardrails / systems / docs / voice / comprehension, but **nothing captures cadence/schedule, exceptions, or success-criteria**. Add those three as typed slots so a teammate's intake records WHEN it runs, WHAT exceptions apply, and HOW success is measured.

## Grep-before-write (extend, do NOT fork — verified 2026-06-05)
- `rg 'pub struct CapabilitySpec' crates/argo-genesis/src/spec.rs` → EXISTS at `spec.rs:144`. Extend it.
- `rg 'RoleContract' crates/argo-genesis/src/role_contract.rs` → EXISTS.
- Reuse the manifest `KpiKind` / `KpiComparator` enums for success-criteria (do NOT define a new comparator enum).

## Files you may touch
- `crates/argo-genesis/src/spec.rs`: add to `CapabilitySpec` — `cadence`, `exceptions`, `success_criteria: Vec<SuccessCriterion{name,kind,target,comparator}>` reusing manifest `KpiKind`/`KpiComparator`; ALL additive `#[serde(default)]` for back-compat.
- `crates/argo-genesis/src/role_contract.rs`: carry the new slots into the contract.
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_payload.rs` (~:45) + TS mirror `web/lib/api/genesis.ts` (~:249): surface in the `vertical_ready` review step.
- a capture panel in `web/components/skycot/genesis/` consistent with the existing Phase-2 panels (follow the pattern; do not fork the panel tree).

## Acceptance
An intake stating a schedule + an exception + a measurable goal persists all three as typed fields visible in the `vertical_ready` review; OLD specs (no new fields) still deserialize (back-compat). Tests pin the round-trip.

## On ship (in-band ledger rule — same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (genesis-intake capture row) + regen its HTML via `python3 scripts/render-feature-matrix.py`.

## Effort
~1 brief (M).
