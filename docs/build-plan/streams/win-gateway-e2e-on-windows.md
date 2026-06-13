# win-gateway-e2e-on-windows

**Owner**: windows-fleet — **P2 — does the full stack SERVE on Windows.**
**Depends on**: none.
**Authored against**: origin/main `033c9589c` on `2026-06-14`.
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build+verify ON REAL WINDOWS; cross-platform changes → conductor re-verifies unix (RUN tests, not just compile) before merge. Push branch, IN_REVIEW; conductor reviews+merges.

## Do (real Windows)
Boot `argo gateway start` on Windows, then exercise the API surface: /health +
/readyz return ok; a magic-link signup or the demo-workspace path; GET an audit
feed; confirm the SPA (if run) or curl reaches the gateway. Capture into
docs/skycot/windows/GATEWAY-E2E.md per-endpoint PASS/GAP. This proves the
SERVER half runs on Windows (the binary-runs smoke covers CLI; this covers the
served API). NO THEATER — real curl output or honest gap.

## Verify
Gateway serves /health + at least one real API round-trip on Windows; honest
record of any endpoint that fails. Branch + IN_REVIEW.
