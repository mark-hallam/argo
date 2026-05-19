# Stream Audit-P2-6 — browser MCP cloud backends marked DEFERRED + wait-for-network-idle wired

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-browser-mcp-cloud-docs`.
**Source**: 2026-05-19 codex audit P2-7, validated by claude `VALIDATION.md` (CONFIRMED — `mcp-servers/browser/src/service.rs:5-8` says 14 local tools wired against running Chrome via raw CDP; cloud backends opt-in but not wired; `schemas.rs:179` says network-idle waiting not yet wired).

## Scope

Browser MCP (`mcp-servers/browser/`) ships 14 working tools against a local Chrome via CDP (Chrome DevTools Protocol — the wire protocol Chrome exposes for programmatic control). Two gaps:

1. **Browserbase / BrowserUse cloud backends are opt-in via `server.toml` but not actually wired**, so an operator who selects them gets undefined behavior. Customer docs and marketing should not imply these backends work in v1.
2. **`wait_for` with `network_idle` is documented but not implemented** at `schemas.rs:179`. Customers who pass `network_idle` as a wait condition will get inconsistent behavior.

This brief does TWO things, mostly docs:

(a) Mark cloud backends as `DEFERRED-v1.x` in the MCP server's README + the operator-facing docs. When an operator's `server.toml` selects a cloud backend at boot, the MCP server logs a clear `WARN` + falls back to local CDP (rather than failing silently or behaving unpredictably).

(b) Wire the `network_idle` wait semantics. CDP exposes `Network.requestWillBeSent` + `Network.responseReceived` + `Network.loadingFailed` events; track in-flight requests and resolve when count returns to zero for `quiet_period_ms` (default 500ms). Mirror Playwright's semantics; cap total wait at the brief's existing `timeout_ms`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `mcp-servers/browser/src/service.rs` — at boot/config load, check `server.toml` backend field; if it selects `browserbase` or `browseruse`, log `WARN` ("Cloud backend X selected but not wired in v1 — falling back to local CDP. Track v1.x for cloud support.") and override to local. Update the doc comment at lines 5-8 to reflect the fallback.
- `mcp-servers/browser/src/schemas.rs` — at the `wait_for` schema (around line 179), document that `network_idle` IS supported as of this brief. Update the `WaitFor::NetworkIdle { quiet_period_ms }` enum variant if it exists, or add it if not.
- `mcp-servers/browser/src/tools/wait_for.rs` (or wherever the wait logic lives) — implement network-idle tracking: maintain a per-page in-flight-request counter that the CDP event handlers increment/decrement; when count is zero for `quiet_period_ms`, resolve. Cap at `timeout_ms`.
- `mcp-servers/browser/src/wait_network_idle_tests.rs` (NEW) — three test cases against a fixture HTTP server: (1) page with zero fetches resolves immediately; (2) page with one fetch resolves after fetch completes + quiet period; (3) page with continuous fetches times out at `timeout_ms`.
- `mcp-servers/browser/README.md` — under "Backends," list local CDP as the only supported v1 backend; mark Browserbase + BrowserUse as `DEFERRED-v1.x`. Update tool docs to confirm `network_idle` is now supported.
- `docs/skycot/operator/mcp-browser.md` (or wherever browser-MCP docs live in operator-docs) — same treatment: cloud backends explicitly deferred.

## Files you must NOT touch

- The 14 working local-CDP tools — they work; don't touch.
- The Browserbase / BrowserUse SDK code (if any was stubbed-in) — leave as deferred-stub; don't delete (operator may have started a v1.x effort).
- The MCP protocol shape — tools and signatures stay identical.
- Any other MCP server (`mcp-servers/slack`, etc.) — out of scope.

## Acceptance criteria

- [ ] Selecting `browserbase` or `browseruse` in `server.toml` logs a clear WARN and falls back to local CDP without crashing.
- [ ] `wait_for: { mode: "network_idle", quiet_period_ms: <N>, timeout_ms: <M> }` works: resolves after N ms of quiet, times out at M.
- [ ] Three new tests in `wait_network_idle_tests.rs` pass.
- [ ] README + operator docs mark cloud backends as `DEFERRED-v1.x`.
- [ ] No regression in existing browser MCP tests (`cargo test -p argo-mcp-browser` or whatever the crate name is).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim in PR body.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-audit-browser-mcp-cloud-docs`
- PR title: `mcp/browser: mark cloud backends deferred + wire wait_for network_idle (mission-audit-browser-mcp-cloud-docs)`

## Notes for the implementer

- Use the existing CDP client (already wired for the 14 working tools) to subscribe to `Network.requestWillBeSent` etc. — don't open a second CDP channel.
- The quiet-period default 500ms matches Playwright's `networkidle0`. Document this in the schema rustdoc.
- Tests need a real (or fixture) HTTP server that the fixture page hits — a `httpmock` or `wiremock` instance works. Document the fixture setup.
- The fallback message must be loud (WARN, not INFO) so an operator who configured a cloud backend hears about it without grepping. Once cloud backends are wired in v1.x, remove the fallback.

## Out of scope (follow-up briefs)

- Actually wiring Browserbase + BrowserUse — v1.x work, separate briefs.
- A local CDP smoke E2E that covers navigate + fill + click + read + screenshot + wait — separate brief once CI runners are back.
- Chrome version pinning / version-skew handling — not yet a real problem.
- Multi-tab support inside wait_for (currently the wait is page-scoped) — separate brief.
