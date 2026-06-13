# win-cu-executor-end-to-end-run

**Owner**: windows-fleet — **P2 — first real model-in-loop CU run on Windows (DEC-054).**
**Depends on**: 
## Goal
With the executor wired (sibling brief), run the FULL computer-use loop on a
real Windows host: a real `run()` driving screenshot→model→action→screenshot.
This is the first end-to-end proof Windows is a live CU target.

## Do (real Windows + an ANTHROPIC_API_KEY)
Drive one small real task (e.g. "open Notepad and type a sentence") through the
wired dispatch path. Capture evidence into docs/skycot/windows/CU-E2E-RUN.md:
the real screenshot bytes (now decodable PNG, #2919), the model's chosen
actions, the input effect (clipboard read-back like #2910), and the
truth-bounded/approval behavior. NO THEATER — real run or honest gap, never a
mocked loop.

## Verify
The doc shows a genuine end-to-end Windows CU run with real screenshot + real
input effect. Branch + IN_REVIEW.
.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Goal
With the executor wired (sibling brief), run the FULL computer-use loop on a
real Windows host: a real `run()` driving screenshot→model→action→screenshot.
This is the first end-to-end proof Windows is a live CU target.

## Do (real Windows + an ANTHROPIC_API_KEY)
Drive one small real task (e.g. "open Notepad and type a sentence") through the
wired dispatch path. Capture evidence into docs/skycot/windows/CU-E2E-RUN.md:
the real screenshot bytes (now decodable PNG, #2919), the model's chosen
actions, the input effect (clipboard read-back like #2910), and the
truth-bounded/approval behavior. NO THEATER — real run or honest gap, never a
mocked loop.

## Verify
The doc shows a genuine end-to-end Windows CU run with real screenshot + real
input effect. Branch + IN_REVIEW.
