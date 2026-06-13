# DEC-054 — Windows becomes a real computer-use executor target

**Status**: active
**Date**: 2026-06-14
**Operator-locked**: yes (this session)

## Decision
Wire the native Windows computer-use executor
(`argo-executor-computer-use-windows`) into `argo-cli`'s `resolve_executor`
dispatch so a Windows host can actually DISPATCH computer-use actions to it —
making Windows a **real executor target**, not just a build/portability target.

## Context
DEC-002 deferred all non-macOS computer-use executors for v1 ("macOS is the
v1 demo path; E2B/Linux/Windows executors stay shipped, not v1-polished").
This session's Windows worker fleet took the Windows executor from
never-compiled to: compiles on Windows (#2906), input primitives proven on a
real host via clipboard read-back (#2910), real screenshot PNG encode (#2919).
The #2910 smoke noted the one remaining gap: the executor's `run()` is NOT
wired into `resolve_executor`, so it's unreachable from the dispatch path.

The operator (2026-06-14) elected to close that gap and make Windows a live
executor target, given the primitives are already proven and a spare Windows
machine is available to do the work.

## Scope expansion vs DEC-002
This is a deliberate, operator-locked expansion of DEC-002's deferral, limited
to the **Windows** executor (E2B/Linux remain deferred). The macOS demo path
is unchanged and remains the v1 reference.

## Guardrails
- Wiring is behind the same default-OFF / opt-in posture as other non-default
  executors — a macOS/Linux deployment is byte-unchanged.
- NO THEATER: the end-to-end Windows CU run must show real evidence (real
  screenshot bytes + real input effect), per the #2910 standard.
- Conductor re-verifies the unix dispatch path is unchanged before merging the
  resolve_executor wiring.

## Consequences
Windows joins macOS as a runnable computer-use target. Tracked via the
`win-cu-executor-wire-into-resolve-executor` + `win-cu-executor-end-to-end-run`
briefs in `docs/build-plan/BUILD_STATUS_WINDOWS.md`.
