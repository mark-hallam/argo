# mission-slack-approval-smoke-scenario — slack approval round-trip in smoke (P1)

**Owner**: codex. **Depends on**: `mission-wire-slack-approval-roundtrip` (per the existing brief in `streams/`; check ledger for state).
**Authored against**: origin/main `c7275d4c` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-slack-approval-smoke-scenario`.
**Source**: v1.0 ship gate per ledger header: "extended `./scripts/smoke-skycot.sh` passes including BrowserExecutor + **Slack-approval scenarios**". Today's smoke covers neither end-to-end.

## Mission alignment

- **🎯 v1.0 SHIP GATE prerequisite** — explicitly named in the ledger as part of the gate.
- **Reliability** (CLAUDE.md priority #1) — Slack-approval is the load-bearing escalation primitive; smoke catches regression.

## Scope

P1. Add one smoke scenario to `scripts/smoke-skycot.sh` that exercises the Slack approval round-trip end-to-end against a local install with a stub Slack webhook receiver.

Flow (single scenario, ~30 seconds):

1. Spin up a tiny HTTP stub server on `localhost:9876` that captures POST requests and exposes `GET /captured` returning the received Slack-bound payload as JSON.
2. Configure argo with `ARGO_CHANNEL_SLACK_WEBHOOK_URL=http://localhost:9876/slack/captured`.
3. Issue a worker action via `argo work serve --once` with a vertical configured for `approval_mode = "ask"`.
4. Assert: stub server captured a POST with Slack-shaped payload (`blocks` array, `Approve` + `Deny` buttons).
5. Simulate operator approval: POST to `localhost:8090/api/v1/channels/slack/interactive` with the action's `action_id` + `payload.actions[0].value = "approve"`.
6. Wait up to 10 seconds for action resume.
7. Assert: worker action completed with `Outcome::Success`; audit chain shows `ApprovalAsked` + `ApprovalReplied` rows with `approved=true`.
8. Cleanup: stub server kill, gateway cleanup.

LOC: ~200.

## Files you may touch

- `scripts/smoke-skycot.sh` — add scenario as a new numbered step (next free; ~scenario 38 after `codex-deploy-smoke-fly-app-target` lands its 8 deploy scenarios). ≤80 modified LOC.
- `scripts/smoke-skycot-helpers.sh` (existing or NEW) — Slack stub server launcher + interactive-callback helper. ≤120 LOC.
- `crates/argo-cli/tests/slack_approval_smoke_test.rs` (NEW) — Rust-native equivalent (cargo-test friendly). ≤180 LOC.

## Files you must NOT touch

- The Slack dispatcher (`crates/argo-tool/src/tools/send_message.rs` + dispatcher modules) — read-only consumer.
- The interactive callback handler (`crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs`) — read-only consumer.
- Existing smoke scenarios — pure additive.

## Acceptance criteria

- [ ] `bash scripts/smoke-skycot.sh` includes a "slack-approval-roundtrip" scenario.
- [ ] Scenario runs against a local install in <60s.
- [ ] Scenario PASSES on current main (no behavior change required — this brief is test-only).
- [ ] Scenario emits SKIP (not FAIL) if `ARGO_SKIP_SLACK_SMOKE=1` env set (for environments without curl + python3 + bash compositions).
- [ ] Stub HTTP server is reaped at scenario end (no leaked process).
- [ ] Equivalent `cargo test -p argo-cli slack_approval_smoke` Rust-native test exists for CI gating without smoke.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-slack-approval-smoke-scenario`
- Title: `test(smoke): slack approval round-trip scenario (mission-slack-approval-smoke-scenario)`
- Worker verification: bash + cargo test + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Stub server in bash** — `python3 -m http.server` is too dumb; use a 15-line `nc` + `awk` OR a small inline `python3 -c "import http.server; ...` with a captured-payload write to `/tmp/`.
- **Interactive-callback signing** — real Slack signs requests with HMAC. The stub callback can either (a) sign with a known test key + configure argo with that key, OR (b) skip-signing in `ARGO_SKIP_SLACK_SIG_VERIFY=1` test mode. Pick (a) for fidelity, (b) for simplicity. Worker chooses, documents in PR body.
- **No real Slack workspace.** Smoke runs offline; if a future scenario wants a real Slack hit, that's a separate "real-slack-smoke.yml.disabled" workflow.
- **Audit assertion is the load-bearing check.** Without it, "approval round-trip works" is paper. Use `argo audit verify-chain --session <id>` for the assertion.
- **`approval_mode = "ask"` is set per-vertical or per-worker.** Test fixture creates an ephemeral worker with this mode; doesn't mutate a production vertical pack.
