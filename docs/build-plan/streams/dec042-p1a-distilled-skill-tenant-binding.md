# dec042-p1a-distilled-skill-tenant-binding

**Owner**: claude or codex — **WS1 TENANT-ISOLATION → conductor adversarial review before merge, do NOT self-merge.**
**Depends on**: none (extends MERGED #2739 self-improvement-cron-driver + #2770 real provider caller).
**Authored against**: origin/main `f9eb6bec3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh dec042-p1a-distilled-skill-tenant-binding

## Problem (DEC-042 Stage-2 P-1 gap, verified 2026-06-12)

The self-improve tick reads traces **tenant-scoped** (`self_improve_cron.rs:250`
filters `tenant_id = ?` at :369) but the distilled skill is installed
**without the tenant**: `distill_and_propose_judged()` (dreams_distillation.rs:253)
carries no tenant param, and the propose chain
(`propose_for_review` :227 → `propose_for_review_judged` :264 →
`propose_for_review_judged_with_evals` :296 in
`crates/argo-skill/src/store_vertical.rs`) ends in `insert_row_with_state(...,
crate::store::PLACEHOLDER_TENANT_ID)` (~:356, conductor-verified by direct
read). Tenant X's self-improve output lands under the placeholder tenant —
wrong tenant binding on a Stage-2 acceptance surface ("installed, audited,
draft-only" must mean *within the originating tenant*).

## Fix (additive param threading — no parallel surfaces)

Thread `tenant_id` from the cron's trace scope through
`distill_and_propose_judged` → the `propose_for_review_judged*` chain → the
`insert_row_with_state` call, replacing `PLACEHOLDER_TENANT_ID` **at this
callsite only**. Use the additive-constructor pattern (new `*_for_tenant`
variant or additive `Option<TenantId>` param with the existing fns delegating)
so other callers compile unchanged — grep all callers first per
DRIFT_PREVENTION. Do NOT touch the active-skill lookup filters in this brief.

## May touch

- `crates/argo-skill/src/store_vertical.rs` (+ tests)
- `crates/argo-skill/src/dreams_distillation.rs` (+ tests)
- `crates/argo-skill/src/self_improve_cron.rs` (+ tests)

## Acceptance / verifier

1. Unit: a self-improve tick run with `tenant_id = T1` installs the distilled
   skill row with `tenant_id = T1` (not placeholder).
2. **Cross-tenant negative pin**: tenant T2's skill listing/lookup never sees
   T1's distilled pending-review skill; T1's does once promoted.
3. `cargo test -p argo-skill` green; clippy clean; no new pub types
   (`rg 'pub (struct|enum|type)'` delta = 0).

~30–60 LOC. **🔴 P1 — DEC-042 Stage-2 keystone + WS1 class.** Sequenced FIRST
of the p1a→p1b→p1c trio (they share files; do not claim p1b/p1c until this
merges).
