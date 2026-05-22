# Mission load-bearing wire #1 — outbound notify on Decision::Ask

**Owner**: claude. **Depends on**: H-auto-approval (#796/#799/#800/#801/#803/#805) merged, all D-* channels (#780/#782/#783/#798) merged.

> SOURCE: deep-research findings tick 198, Section 1, item #1. The
> single largest mission-claim-6 ("multi-channel approvals")
> credibility win. Roughly 50–100 LOC.

## Scope

Today the auto-approval gate at
[worker_actions_auto_approval.rs:251](crates/argo-cli/src/handlers/worker_actions_auto_approval.rs)
returns `Ok(decision)` on `Decision::Ask` and the channel layer is
NEVER called for outbound notification. `argo-channels` ships eight
working `NotifyChannel` implementations (email/slack/sms/push/voice/
whatsapp/imessage/webinbox), all with real backends, but they only
fire today via `budget_watch::*` for budget-anomaly alerts.

Result: when a worker action lands `Pending` and the gate returns
`Ask`, the operator receives nothing. The action sits in the queue
until a human polls `argo worker actions list`. The mission promise
"asks via the appropriate channel" is undelivered.

This brief wires the missing connection: read the action's manifest
`[action.<name>.approval]` table, construct a `Notification`, and
fan out `channel.send(n).await` for each declared channel. Failure
to dispatch a notification is logged but does not change the
`Decision` returned to the caller.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` —
  add a small `dispatch_notify` helper called on the `Decision::Ask`
  arm; reuses existing manifest reader for the `[action]` block
- `crates/argo-cli/src/handlers/worker_actions_notify.rs` (NEW,
  sibling) — extract the dispatch_notify helper + per-channel
  fan-out + manifest-reader glue
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: ONE
  `pub(crate) mod worker_actions_notify;` line
- `crates/argo-cli/src/handlers/worker_actions_notify_tests.rs`
  (NEW) — 6 unit tests (no manifest channels → no-op; one channel →
  one send; multi-channel fan-out; one channel fails → others still
  fire; risk-class respect; quiet-hours respect)
- `crates/argo-cli/tests/notify_on_ask_smoke.rs` (NEW) — integration
  smoke: Decision::Ask round-trip with a `StubNotifier` registered

## Files you must NOT touch

- `crates/argo-channels/**` (NotifyChannel trait + per-channel impls
  are stable from D-stack; reuse, don't modify)
- `crates/argo-skill/**` (judge wiring is a separate brief)
- Any other handler beyond the two listed

## Acceptance criteria

- [ ] `Decision::Ask` arm calls `dispatch_notify(action, decision)`
      before returning Ok
- [ ] dispatch_notify reads manifest `[action.<name>.approval]`
      and resolves the channel list
- [ ] Each declared channel's `send(notification)` is awaited;
      errors are logged via `tracing::warn!` but do not propagate
- [ ] If no channels are declared, dispatch_notify is a no-op (does
      not block the Decision)
- [ ] Risk-class deferral honored (high-risk actions still fire on
      SMS/Voice/WhatsApp; low-risk skipped per existing channel logic)
- [ ] Quiet-hours filter honored where the channel implements it
- [ ] 6 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-wire-outbound-notify-on-ask`
- PR title: `actions: dispatch outbound notify on Decision::Ask (mission-wire-outbound-notify-on-ask)`

## Notes for the implementer

- The `Notification` shape lives in `argo-channels` — reuse it, do
  not invent a new envelope.
- The `[action.<name>.approval]` manifest section already exists;
  H-auto-approval reads it for gate evaluation. Locate that reader
  and reuse — do not parse the TOML twice.
- The existing budget-anomaly path in the watcher is the model for
  fan-out — read it for the dispatch-and-log idiom.
- Do NOT block on dispatch failure. The action's gate Decision is
  the source of truth; notify is best-effort. If Postmark is down,
  the action stays Pending and a human can still query it via CLI.
- After this lands, `mission-wire-mcp-action-executor` becomes the
  next demo unblocker (the marketplace-seller send_buyer_followup
  action depends on both).
