# dec042-p1c-distill-and-proposal-audit-rows

**Owner**: claude or codex — conductor review before merge (audit/compliance surface).
**Depends on**: dec042-p1b-eval-floor-gates-distilled-proposal MERGED (shared files; p1b may move the proposal point this brief audits).
**Authored against**: origin/main `f9eb6bec3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh dec042-p1c-distill-and-proposal-audit-rows

## Problem (DEC-042 Stage-2 "audited" leg — distill + proposal are silent)

The only audit row in the self-improve loop today is the per-cluster
**decision** row (`self_improve_cron_helpers.rs:18-86`,
kind `skill.self_improve.decision`). The two upstream events are silent
(verified 2026-06-12):

- **Distill call**: `distill_cohort` / `distill_and_propose_judged`
  (dreams_distillation.rs:253-265) and the CLI path
  (`skill_distill_now.rs:200-223`, a REAL provider hit with real cost) emit no
  audit row — provider spend + model + outcome are invisible to the audit
  trail.
- **Proposal entry**: `propose_for_review_judged_with_evals`
  (store_vertical.rs:296-358) inserts the pending-review row silently — the
  operator sees a skill in the queue with no record of HOW it arrived
  (distilled vs manual).

## Fix (two new audit kinds; reuse the hash-chain helpers, don't fork)

1. `skill.distilled` — emitted when a distillation call returns: payload =
   cohort (vertical, action, size), model, est. cost, candidate skill
   name/version, tenant_id. Emit from the cron path AND the
   `skill distill-now` CLI path.
2. `skill.proposed_for_review` — emitted when a candidate enters the review
   queue: payload = skill name/version, vertical, source
   (`distilled | judged_memory | manual`), learned_in session, tenant_id.

Reuse the BEGIN-IMMEDIATE + `chain_hash`/`last_audit_hash` pattern from
`self_improve_cron_helpers.rs` — do NOT hand-roll chaining. Check the
`AuditKind` extension point for LOC headroom first (the event-kind file has a
history of ceiling pressure); if the enum file cannot take 2 variants under
the 500-LOC gate, STOP and queue the split brief instead of force-fitting.

## May touch

- `crates/argo-skill/src/dreams_distillation.rs`, `self_improve_cron.rs`,
  `self_improve_cron_helpers.rs`, `store_vertical.rs` (+ tests)
- `crates/argo-cli/src/handlers/skill_distill_now.rs` (+ tests)
- the audit-kind registry: BOTH `crates/argo-audit/src/event_kind.rs`
  (the enum, ~321 LOC — additive variants only) AND
  `crates/argo-audit/src/event.rs` (the as_str/parse/all mapping at
  ~:45/:143/:241, ~326 LOC) — a new kind needs edits in both
  (discovery-verified 2026-06-12)

## Acceptance / verifier

1. Test: one self-improve tick over a qualifying cohort yields ≥3 chained
   audit rows — distilled → proposed_for_review → decision — hash-chain
   verifies end-to-end.
2. Test: `argo skill distill-now` (mock caller) emits `skill.distilled` with
   tenant + model + cost fields populated.
3. `cargo test -p argo-skill -p argo-cli` targeted suites green; clippy clean.

~90–140 LOC. **🟡 P2 — closes the "audited" leg of Stage-2.** Claim only
after p1b merges.
