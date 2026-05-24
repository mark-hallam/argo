# mission-sms-inbound-approval-reply-route

**Owner**: claude
**Priority**: P1 — claim 6 approval settlement gap
**Authored against**: origin/main `50d329dc2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-sms-inbound-approval-reply-route`.

## Source

PR #2174 (`eval-channels-inbound-coverage-v2`) pins the channel matrix and names SMS as the only outbound-routable channel without an inbound approval-settlement path. Existing siblings already show the patterns:

- `routes_channels_whatsapp.rs`: Twilio form webhook, `Body` reply parsing, signed approval-token settlement.
- `routes_channels_voice.rs`: Twilio signature verification and route registration.
- `routes_channels.rs`: central channel webhook dispatcher and shared `settle(...)`.

## Goal

Add a real Twilio SMS inbound approval route so an operator can reply to an SMS prompt with `yes <token>` / `no <token>` and settle the pending action through the same approval-token + audit path as Slack, WhatsApp, iMessage, push, and web inbox.

## May Touch

- `crates/argo-cli/src/handlers/gateway/mod.rs`
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
- `crates/argo-cli/src/handlers/gateway/routes_channels_sms.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs` only if a small Twilio form/signature helper is extracted or shared.
- `crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs` only if route-test helpers need to stay consistent.
- `crates/argo-cli/src/handlers/gateway/routes_voice_auth.rs` only if Twilio signature helpers are generalized without changing voice behavior.
- `crates/argo-channels/src/sms.rs` only for doc comments or unit tests proving the outbound SMS body carries tokens in the reply-friendly shape.
- `crates/argo-channels/tests/channel_inbound_coverage_pin_test.rs` if present on the worker's base, to flip the SMS finding from ignored/failing to live passing.

## Must Not Touch

- Do not change channel policy, quiet-hours, rate-limit, or SMS high-risk gating semantics.
- Do not change Twilio outbound credentials or message endpoint config.
- Do not route Teams/Matrix/WhatsApp deferrals as part of this brief.
- Do not edit web UI.

## Acceptance

- [ ] New SMS route predicate accepts `POST /twilio/sms/decision` or `POST /channels/sms/webhook` consistently; pick one route shape and document it in tests.
- [ ] Request handling verifies `X-Twilio-Signature` before parsing or settling.
- [ ] Body parsing accepts explicit approval tokens from `Body` (`approval_token=...`, `token=...`, or second whitespace token) and rejects missing/mismatched tokens.
- [ ] `yes` / `y` / `approve` resolves approve; `no` / `n` / `reject` / `deny` resolves reject. Unsupported replies return a 400 with no state change.
- [ ] Calls shared `settle(...)` with source `"sms"` so `approval.replied` audit rows remain tenant-stamped via the existing gateway path.
- [ ] Late replies are idempotent through existing `settle(...)` behavior.
- [ ] Add unit tests for predicate, valid approve, valid reject, bad signature, missing token, mismatched reply-vs-token decision, unsupported text, and late reply behavior if cheap.
- [ ] If PR #2174's pin test is present, update it so SMS inbound is no longer a FINDINGS-only gap.
- [ ] `cargo clippy -p argo-cli -p argo-channels --all-targets -- -D warnings` is clean, or the PR body records the current workspace-clippy blocker with a clean-main repro if unrelated.

## Notes

Keep this narrowly about SMS inbound settlement. If helper extraction would push any file above 500 LOC, split the helper into a sibling gateway module instead of growing an already-large route file.
