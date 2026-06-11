# cu-risk-tiered-computer-use-floor

**Authored against**: origin/main `729166cc6` on `2026-06-11`
**Validate before claim**: bash scripts/check-brief-freshness.sh cu-risk-tiered-computer-use-floor

## Goal (DEC-051 — operator-locked)

Enforce the risk-tiered computer-use floor by whether an op COMMITS an action.
AUTONOMOUS ops (screenshot, cursor_position, cursor/mouse move, scroll, **type**,
screen read, wait — observe / navigate / STAGE input) run without approval;
GATED ops (any click, `key`/`hold_key` — incl Enter/Return-as-submit, drag, app
launch, mutating osascript — the ops that COMMIT) are approval-gated **regardless
of the manifest's self-asserted `requires_approval`/`risk`**. Typing is
autonomous ONLY because the submit (a key press or click) stays gated, so typed
text is staged + reviewable before the gated commit. Unknown → Gated (fail
closed). Closes the cu3/#2816 self-assertion hole; unblocks #2816.

## Why (DEC-051 + #2816 review)

Today a cu action's autonomy is stamped from the manifest's `requires_approval`/
`risk` (`worker_actions_emit.rs:130-196` → `is_auto_eligible_with_flag`). A
manifest can declare a clicking/typing cu action `risk=low, requires_approval=false`
→ autonomous host control with no approval once `ARGO_EXECUTION_GRADUATION` flips.
Operator chose risk-tiered (DEC-051): bind the floor to the OPERATION's
reversibility, enforced in the executor.

## Fix

1. **Classify cu operations** (a small `enum`/fn in
   `crates/argo-executor-computer-use-macos`, or a shared spot the run loop
   uses): `Autonomous` { screenshot, cursor_position, cursor/mouse move, scroll,
   `type`, wait, read/OCR, enumerate } vs `Gated` { left/right/middle/double
   click, click_drag, `key`/`hold_key` (incl Enter/Return/Tab — these SUBMIT
   typed text), app launch, mutating osascript }. **Unknown → Gated** (fail
   closed, DEC-041 boundary 4). Note: `type` is Autonomous BUT `key` is Gated —
   that pairing is load-bearing (typed text only commits via a gated key/click).
   Map the Anthropic CU action vocabulary the run loop already parses (grep
   `action_kind` in `run_loop*.rs`).
2. **Gate in the run loop**: Autonomous ops execute (incl typing into fields). Before the FIRST Gated op, if the
   action is not approved-for-acting, return `ActionOutcome::HumanHandoff` (HELD)
   — the operator's cu5 "Approve & run" (#2818) grants + resumes. Observe ops
   before that point are NOT wasted (their screenshots inform the held review).
3. **Belt-and-suspenders**: even when the manifest stamped `requires_approval=false`,
   the executor floor holds at the first Act op. (The gate's coarse manifest
   tiering stays; the executor is the act-floor.)
4. **Unblock cu3**: after this lands, flip `cu3-register-cu-macos-executor-in-run-path`
   / #2816 BLOCKED→TODO (or merge #2816 with the floor in place) and remove the
   cu-floor item from DEC-037's flip-gate.

## May touch
- `crates/argo-executor-computer-use-macos/src/{run_loop.rs,executor.rs,lib.rs}` (+ tests)
- possibly `crates/argo-cli/src/handlers/work_dispatch_executor_inline.rs` (belt-and-suspenders)
- docs/skycot/decisions/DEC-037 (remove the cu-floor flip-gate item once enforced)

## Acceptance / verifier
1. An observe-only cu action (only screenshots) runs to completion autonomously
   (flag on) — no approval row.
2. A cu action that reaches a click or key-press (Enter/submit) op returns `HumanHandoff`
   (held) at the first GATED op, EVEN with manifest `requires_approval=false`; operator Approve &
   run resumes + the act executes.
3. An unknown/unclassified op → treated as Act → held.
4. `cargo test -p argo-executor-computer-use-macos` green; gate green.

## Size / owner
~150-250 LOC (classifier + run-loop gate + tests). Owner: claude. 🔴 P1 — the
operator-locked computer-use safety floor; gates the `ARGO_EXECUTION_GRADUATION`
flip + unblocks cu3/#2816.
