# win-binary-runs-smoke-doctor-and-cli

**Owner**: windows-fleet — **P2 — does the binary actually RUN on Windows (not just compile/test).**
**Depends on**: none.
**Authored against**: origin/main `c24ae0d84` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform fixes → conductor re-verifies unix. Push branch, IN_REVIEW; conductor reviews+merges.

## Why
Everything COMPILES + tests pass on Windows, but we haven't confirmed the
`argo` binary actually RUNS on a real Windows host. Compiling != running.

## Do (on real Windows, the built release argo.exe)
Run + record output for: `argo --version`, `argo doctor` (the install/health
check — note any unix-assumption failures), `argo session list --json` (JSON
output contract holds on Windows?), `argo gateway start --port <free>` then
`curl http://127.0.0.1:<port>/health` then stop it (does the gateway boot +
serve on Windows?). Capture each command's real output into
docs/skycot/windows/BINARY-RUN-SMOKE.md with PASS/GAP per command. NO THEATER:
record real failures (e.g. a unix path in doctor) honestly + spawn briefs.

## Verify
The doc shows real Windows command output; the gateway /health returns ok on
Windows OR the failure is honestly recorded. Branch + IN_REVIEW.
