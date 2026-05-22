# V1.4 — Approval rate limit per channel

**Owner**: claude. **Depends on**: D-slack/D-sms/D-push/D-email
channels merged; worker_actions_notify.rs (#931) wires
dispatch.

## Why

A misbehaving (or simply busy) worker can fire dozens of
approval requests in seconds. The Slack channel today will
happily flood a #ops room with 100 prompts in a tight loop —
operator brain trauma, real risk that critical prompts get
ignored. Push/SMS providers will rate-limit US back at the
provider level (worse: the user just stops trusting the app).
Add a per-channel token-bucket so the system self-throttles.

## Scope

Add a small reusable `RateLimit` type to argo-channels that
each `NotifyChannel` impl wraps its `send_notification` call
in. Default budgets are conservative (slack: 5/sec, 60/min;
sms: 1/sec, 10/min; push: 5/sec, 100/min; email: 5/sec, 30/min;
voice: 1/min, 5/hour; webinbox: unlimited). Operators override
via channel config (`[notify_channels.slack.rate_limit]`).

When a request would exceed the budget, the channel returns
`Err(NotifyError::RateLimited { retry_after })`. The
notification dispatcher (worker_actions_notify.rs) already has
retry logic — extend it to honour `retry_after` as backoff
(small change, narrowly scoped).

NOT in scope: cross-process rate limiting (single-process
in-memory bucket is enough for v1). NOT a global rate limiter
across all channels.

## Approach

1. NEW `crates/argo-channels/src/rate_limit.rs` (~220 LOC):
   - `pub struct RateLimit { per_sec: TokenBucket, per_min: TokenBucket, per_hour: TokenBucket }`
   - `pub fn try_acquire(&self) -> Result<(), Duration>` returns
     `Err(retry_after)` when any window is exhausted
   - `TokenBucket` is a small struct holding `last_refill: Instant`,
     `tokens: f64`, `capacity: f64`, `refill_per_sec: f64` —
     classic leaky bucket
   - All Send + Sync via `Arc<Mutex<...>>`; no async fns inside
2. Wire into each existing `NotifyChannel` impl narrowly:
   - `slack.rs`, `sms.rs`, `email.rs`, `push.rs`, `voice.rs`,
     `imessage.rs`, `whatsapp.rs` — each gains a private
     `rate_limit: RateLimit` field + a `try_acquire` call at
     the top of `send_notification`
   - `webinbox.rs` opts out (unlimited)
3. Channel config schema gains an optional
   `[rate_limit] per_sec, per_min, per_hour` block per channel.
   Manifest parsing in `manifest.rs` extends to read it (narrow).
4. `worker_actions_notify.rs` retry path: when error matches
   `RateLimited { retry_after }`, sleep for that duration
   (capped at 60s) before next attempt.

## Files you may touch

- `crates/argo-channels/src/rate_limit.rs` (NEW, ≤ 320 LOC)
- `crates/argo-channels/src/rate_limit_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-channels/src/lib.rs` — narrowly: 1 pub mod line +
  re-export of `RateLimit`, `NotifyError::RateLimited`
- `crates/argo-channels/src/{slack,sms,email,push,voice,imessage,whatsapp}.rs` —
  narrowly: add field + try_acquire call. NO refactor.
- `crates/argo-channels/src/manifest.rs` — narrowly: parse the
  optional `rate_limit` config block
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` —
  narrowly: handle `RateLimited` retry-after backoff

## Files you must NOT touch

- Channel send-payload formats (no message change)
- The dispatcher's overall retry policy beyond honouring
  retry_after
- Storage schema

## Acceptance criteria

- [ ] Sending 10 slack notifications in 100ms results in the
      first 5 going through, the rest deferred with
      `RateLimited { retry_after }`
- [ ] Default budgets are documented in `lib.rs` doc comment
- [ ] Operator override via TOML works: `per_sec = 20` raises
      the budget
- [ ] webinbox channel unaffected (unlimited)
- [ ] Retry path in worker_actions_notify.rs honours
      retry_after (verified via mocked clock)
- [ ] Token buckets refill correctly over time (unit test
      with `tokio::time::pause` + `advance(1s)`)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-approval-rate-limit`
- PR title: `channels: per-channel approval rate limit (claude-approval-rate-limit)`

## Notes for the implementer

- Use deterministic `tokio::time::Instant` via `tokio::time::pause`
  for tests. Never wall-clock sleep in tests.
- The bucket MUST be `Arc<Mutex<...>>`-shareable so cloning a
  channel handle still hits the same bucket.
- If the channel is reconstructed per-request (unlikely but
  possible), put the `RateLimit` in a `static OnceLock<HashMap<ChannelId, Arc<RateLimit>>>` keyed by channel id.
- Document the default budgets in the channel manifest schema
  doc — operators need to know what they're inheriting.
