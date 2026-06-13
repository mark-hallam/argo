# win-computer-use-executor-smoke

**Owner**: windows-fleet — **P2 — first real Windows run.**
**Depends on**: win-computer-use-executor-first-build MERGED.
**Authored against**: origin/main `f142baad5` on `2026-06-13`.
**Fleet**: WINDOWS — claimed from docs/build-plan/BUILD_STATUS_WINDOWS.md, NOT the main ledger. Build + verify ON REAL WINDOWS, push a branch, flip the row to IN_REVIEW (do NOT self-merge — the macOS conductor gates + merges).

## Do (on real Windows, after the executor compiles)
Run the executor preflight + a minimal real smoke: screenshot the desktop,
move+click at a safe coordinate (e.g. an open Notepad), type a string, read it
back. Capture evidence (the screenshot, the typed-text confirmation) into
docs/skycot/windows/CU-EXECUTOR-SMOKE.md. If a primitive doesn't work, record
the real failure (NO THEATER — a partial smoke with honest gaps beats a fake
pass).

## Verify
The smoke report shows real Windows screenshot+click+type evidence or honest
per-primitive pass/fail. Branch + IN_REVIEW.
