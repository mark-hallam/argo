# DEC-051 — Computer-use operation risk-tiering (observe-autonomous, act-gated)

- **Status**: active
- **Date**: 2026-06-11
- **Operator-locked**: yes ("i would like the risk-tiered rule(s) for computer use")
- **Extends**: [DEC-041](DEC-041-execution-posture-risk-tiered-act-by-default.md) (risk-tiered act-by-default) to computer-use OPERATIONS.
- **Closes**: the cu3/#2816 hole (a manifest could self-declare a clicking/typing
  computer-use action as `risk=low, requires_approval=false` → autonomous host
  control with no approval). Recorded as a flip prerequisite in [DEC-037](DEC-037-graduated-real-execution.md).

## Context

DEC-041 tiers execution: SAFE (low-risk + reversible → auto-act) vs SENSITIVE
(mutating / irreversible / public-facing → approval-gated), failing closed on
unknown. The cu3 review (#2816) found the tier for a computer-use action was
stamped from a SELF-ASSERTED manifest field — so an author could mark a
host-controlling action "low-risk, no-approval" and, once `ARGO_EXECUTION_GRADUATION`
flips on, drive the Mac autonomously with no human in the loop.

The operator was offered two floors: (a) all computer-use ALWAYS requires
approval, or (b) risk-tiered. **The operator chose risk-tiered (b).** Rationale:
an unconditional wall blocks benign observation (a teammate that can't even take
a screenshot autonomously is dull and unhelpful); the right line is reversibility,
exactly DEC-041's.

## Decision

A computer-use action's autonomy is decided **per-operation by whether the
operation COMMITS a consequential action, NOT by a self-asserted manifest
field**:

- **AUTONOMOUS tier — eligible to run without approval (safe, per DEC-041).**
  Operations that observe, navigate, or STAGE input but cannot, on their own,
  commit a consequential or irreversible effect: `screenshot`,
  `cursor_position`, screen read / OCR, window/app enumeration, `wait`,
  **cursor/mouse move**, **scroll**, and **`type` (entering text into a
  field)**. Run within DEC-041's envelope (spend cap, rate limit, scope,
  hash-chained audit). **Typing is autonomous ONLY because the operation that
  SUBMITS typed text — a key press (Enter/Return) or a click — stays gated
  (below).** Typed text is staged and reviewable before that gated commit; on
  its own, filling a field changes nothing irreversibly.
- **GATED tier — approval-required (sensitive, per DEC-041), REGARDLESS of the
  manifest's claimed `risk`/`requires_approval`.** Operations that COMMIT or
  trigger an action: `left_click`/`right_click`/`middle_click`/`double_click`,
  `left_click_drag`, `key`/`hold_key` (Enter/Return/Tab/shortcuts =
  submit/save/close/destructive), app launch, and any `osascript` that performs
  (not just reads). The platform FLOORS these to approval-required — a manifest
  cannot downgrade them. These are the operations that turn staged work (typed
  text, a filled form) into a real, often irreversible effect.

**Fails closed:** an unrecognized / unclassifiable operation is treated as GATED
(sensitive), per DEC-041 boundary 4.

**Residual edge (accepted):** in a command-interpreting app (a modal editor, a
single-keystroke-shortcut UI), individual typed characters can themselves BE
commands rather than text entry, so autonomous `type` could trigger an action
there. `type` is autonomous on the assumption of text-entry contexts (forms,
search boxes, message bodies — the "real work" case). A teammate meant to drive
a command-mode app should be hired with `type` denied/gated in its manifest;
this floor governs the platform default.

## Enforcement (where the floor lives)

In the **computer-use executor run loop** (`argo-executor-computer-use-macos`),
because that is where the agent decides each operation at runtime (one manifest
cu action runs a LOOP of operations):

1. OBSERVE operations run freely.
2. At the FIRST ACT operation, if the action is not operator-approved-for-acting,
   the executor returns `HumanHandoff` — the action is HELD in `/approvals`, and
   the operator's "Approve & run" ([DEC-050 cu5/#2818](DEC-050-computer-use-buildout-morning-real-work-demos.md))
   grants + resumes it. After approval the act-ops execute.
3. Belt-and-suspenders: the manifest keeps its declared `risk`/`requires_approval`
   for the gate's coarse tiering, but the executor enforces the act-floor even
   when the manifest claimed `requires_approval=false`.

This makes "read-only computer-use autonomous, clicking/typing gated" a PLATFORM
guarantee bound to the operation's reversibility, not the manifest author's word.

## Consequences

- Unblocks cu3 (#2816) once the floor lands (the executor registration is safe
  with the floor in place). Removes the cu-floor item from DEC-037's flip-gate.
- Still default-OFF: the autonomous OBSERVE path only fires when
  `ARGO_EXECUTION_GRADUATION` is on (operator-only flip, DEC-041 boundary 1-2).
  ACT always gates regardless of the flag.
- The DEC-037 tenant-blind cu-macos audit item (#2816 secondary) remains a flip
  prerequisite.

## Verifier

- An observe-only cu action (screenshots a dashboard) runs autonomously (flag on);
  no approval row.
- A cu action that reaches a click/type op HOLDS at the first act-op
  (`HumanHandoff` → `/approvals`), even when its manifest declared
  `requires_approval=false`; the operator's Approve & run resumes + acts.
- An unknown op classifies as ACT → gated.
- Brief: `streams/cu-risk-tiered-computer-use-floor.md`.
