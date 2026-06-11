# dec042-p1b-eval-floor-gates-distilled-proposal

**Owner**: claude or codex — conductor review before merge (self-improvement loop semantics).
**Depends on**: dec042-p1a-distilled-skill-tenant-binding MERGED (shared files: self_improve_cron.rs / dreams_distillation.rs / store_vertical.rs).
**Authored against**: origin/main `f9eb6bec3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh dec042-p1b-eval-floor-gates-distilled-proposal

## Problem (DEC-042 Stage-2 P-1: the eval floor is advisory, not a gate)

DEC-042 Stage-2 acceptance: "a distilled skill **beats the eval floor** and
gets installed, audited, draft-only." Today the order is inverted
(`crates/argo-skill/src/self_improve_cron.rs:280-308`, conductor-verified by
direct read): `distill_and_propose_judged()` puts the candidate into
`pending_review` FIRST (:281), `run_ab_eval()` runs AFTER (:289), and
`decide_promotion()` (:303) only controls promotion + the decision-audit row.
A candidate the eval would **Reject** still sits in `pending_review`, where an
operator can approve it never having beaten the floor — the eval result is
advisory. That violates the DEC-042 verifier-spine rule (the loop must end in
a checkable gate, not a self-report).

## Fix (gate before the queue; reuse, don't fork)

Restructure the tick so the candidate is A/B-evaled and judged by
`decide_promotion()` (`self_improve_loop.rs:220` — the single source of truth,
REUSE it, no parallel policy) **before/atomically-with** entering the review
queue:

- `Reject` → candidate is installed as `LifecycleState::Rejected` (or not
  proposed at all — pick whichever keeps the decision-audit row coherent and
  say so in the PR body), never `PendingReview`.
- `Hold` / `Promote` → existing behavior (PendingReview; promote path
  unchanged via `apply_promotion`).
- The decision audit row (`append_decision_audit`) keeps firing for ALL
  outcomes — do not lose audit coverage while reordering.

## May touch

- `crates/argo-skill/src/self_improve_cron.rs` (+ tests)
- `crates/argo-skill/src/dreams_distillation.rs` (+ tests)
- `crates/argo-skill/src/store_vertical.rs` (+ tests; a propose variant that
  takes the decision/eval outcome is acceptable if additive)

## Acceptance / verifier

1. Test: a candidate whose A/B report triggers `Reject` (regression ≥ 0.05 or
   below floor) is NOT in `pending_review` after the tick (negative pin).
2. Test: a candidate that beats the floor lands in `pending_review` and
   promotes exactly as today (no regression in the promote path).
3. Decision-audit rows still emitted for promote/hold/reject (existing
   helpers test extended, not forked).
4. `cargo test -p argo-skill` green; clippy clean.

~40–90 LOC. **🟠 P1 — completes the DEC-042 Stage-2 acceptance bar.** Claim
only after p1a merges.
