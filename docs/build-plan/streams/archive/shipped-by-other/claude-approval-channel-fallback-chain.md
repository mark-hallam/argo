# V1.4 — Approval channel fallback chain

**Owner**: claude. **Depends on**: D-slack/D-email/D-webinbox
channels merged; worker_actions_notify.rs (#931) wires
dispatch.

## Why

Slack workspaces go down. Email goes to spam. SMS providers
have outages. A v0 single-channel notification model means the
operator misses the prompt and the worker stalls. Mission
claim 6 says "v1 multi-channel" — fallback is the simplest
durable expression of that.

## Scope

Add a `[notify_channels.fallback_chain]` config block:

```toml
[notify_channels.fallback_chain]
order = ["slack", "email", "webinbox"]
per_step_timeout_secs = 30
```

When an approval notification is dispatched, try `slack`
first. On any of: transport error, rate-limit
(`claude-approval-rate-limit`), or no acknowledgement within
`per_step_timeout_secs`, fall through to `email`. On further
failure, fall through to `webinbox` (which is the always-on
local store and effectively the floor).

The notification record gains a `delivery_attempts: Vec<DeliveryAttempt>`
field where each attempt logs `channel, started_at, outcome`.
This is what `argo doctor channels` will read.

NOT in scope: parallel fan-out (send to all channels at once)
— that's a separate feature. NOT changing the approval
decision flow — only the notification side.

## Approach

1. NEW `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs`
   (~280 LOC):
   - `pub async fn dispatch_with_fallback(req: &NotificationRequest, chain: &FallbackChain, channels: &ChannelRegistry) -> ArgoResult<DeliveryReport>`
   - Walks `chain.order`, `tokio::time::timeout`-wraps each
     attempt, collects `DeliveryAttempt` rows
   - Returns the first success, or `Err` only if all attempts
     fail (webinbox should never fail in practice — it's a
     local sqlite write — so this is degenerate)
2. Schema for `FallbackChain` in `crates/argo-types/src/channels.rs`
   (narrow add). `DeliveryAttempt` and `DeliveryReport` go
   alongside.
3. Wire into existing `worker_actions_notify.rs` narrowly:
   when config has a fallback_chain, route through the new
   helper instead of the single-channel dispatch.
4. `argo doctor channels` (existing) extended to print the last
   N delivery reports (1 line each) — narrow change.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_notify_fallback.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/worker_actions_notify_fallback_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod line
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` —
  narrowly: route through fallback when config present
- `crates/argo-cli/src/handlers/doctor_channels.rs` —
  narrowly: print delivery_attempts summary
- `crates/argo-types/src/channels.rs` (or wherever channel
  types live) — narrowly: add `FallbackChain`,
  `DeliveryAttempt`, `DeliveryReport`
- `crates/argo-channels/src/manifest.rs` — narrowly: parse the
  `fallback_chain` config block

## Files you must NOT touch

- Individual channel impls (slack.rs, email.rs, etc.) — they
  stay single-shot; fallback orchestration lives above them
- Storage schema for approvals — delivery_attempts can live
  in a sibling table, not the approval row
- `session_approval.rs` decision logic

## Acceptance criteria

- [ ] With chain `[slack, email, webinbox]` and slack returning
      a transport error, email is tried; if email fails too,
      webinbox is tried
- [ ] Per-step timeout is honoured (verified with mocked
      clock; channel that hangs > timeout counts as failure)
- [ ] First successful channel returns; subsequent channels
      not tried
- [ ] DeliveryReport contains one DeliveryAttempt per channel
      tried, with outcome
- [ ] `argo doctor channels` prints last 10 delivery reports
- [ ] No fallback_chain config → existing single-channel
      behaviour preserved
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `claude-approval-channel-fallback-chain`
- PR title: `channels: approval fallback chain (claude-approval-channel-fallback-chain)`

## Notes for the implementer

- webinbox should be the LAST entry in any chain — it's a
  local sqlite write, effectively never fails, and gives the
  "floor" guarantee that no notification is ever fully lost.
- Treat rate-limit error (from `claude-approval-rate-limit`)
  as a fallback trigger, NOT a fatal error. Log the
  retry_after but move on.
- Per-attempt logs at INFO level so audit-export
  (`codex-approval-audit-export`) can pick them up.
- For the test fixtures, define a `MockChannel` that returns
  scripted outcomes (Ok / TransportErr / Hang). Keep it inside
  the test file — do NOT add a public mock helper.
