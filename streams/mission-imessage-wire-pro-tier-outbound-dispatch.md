# mission-imessage-wire-pro-tier-outbound-dispatch

**Priority**: P1 (sales-page honesty — Pro $29/seat tier promises iMessage; production dispatcher refuses it)
**Owner**: claude
**Type**: Wire-up — flip an existing-but-rejected channel to live + add a regression pin
**Size**: ~120-220 LOC across 2 files (one new dispatcher module + one match-arm + 1 pin test)
**Authored against**: origin/main `cd338914f` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-imessage-wire-pro-tier-outbound-dispatch`

## Why

`web/app/pricing/page.tsx:64` advertises the Pro tier as
"Multi-channel (Slack / iMessage / Email)". The audit
`eval-channels-v1-config-vs-code-inventory` (PR #2145, MERGED) confirmed all
four iMessage substrate pieces ship on `main`:

- `crates/argo-channels/src/imessage.rs` — outbound `IMessageChannel`
  (BlueBubbles client wrapper, ~210 LOC).
- `crates/argo-bluebubbles-client/` — the HTTP client.
- `crates/argo-channels/tests/preferences_smoke.rs` — already routes "Bob"
  to `primary_channel = "imessage"`.
- `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs:27,44,63-79` —
  pin set already includes `imessage`.

**The single missing piece** is the dispatcher branch at
`crates/argo-cli/src/handlers/worker_actions_notify_env.rs:30`:

```rust
other => Err(format!(
    "channel kind '{other}' is not supported by dispatch_notify \
     (v1 channels: push, webinbox, slack, email, sms, discord, chat, sse, voice; \
     DEFERRED-v1.x: imessage, whatsapp, teams, matrix)"
)),
```

A Pro-tier tenant who configures iMessage today gets that error back through
the auto-approval pipeline — every iMessage approval falls back to the next
channel (or fails entirely if no chain is set), even though the substrate is
ready. This is a direct sales-promise breach (gap-manifest §"CRITICAL
sales-promise gap").

The other three DEFERRED-v1.x channels (whatsapp, teams, matrix) stay
deferred — they are NOT in any pricing tier copy on `web/app/pricing/page.tsx`
and per CLAUDE.md v1.0 strategic decisions are intentionally out of scope.
**This brief is iMessage-only.**

## What

Two surgical changes mirroring the existing `notify_discord` pattern:

1. **NEW** `crates/argo-cli/src/handlers/worker_actions_notify_imessage.rs`
   (~50-90 LOC). Mirrors `worker_actions_notify_discord.rs` but constructs
   an `IMessageChannel` via `IMessageChannelConfig::from_env()` (uses the
   existing `required_env("imessage", "...")` helper for `BLUEBUBBLES_HOST` +
   `BLUEBUBBLES_PASSWORD` — confirm exact env var names against
   `crates/argo-channels/src/imessage.rs` at claim time; if the config
   constructor needs additional fields, surface them via `required_env`).
   Build a `Notification` from the inbound `Notification` (already shaped
   for any channel) and call `IMessageChannel::send(...).await`, mapping
   `ChannelError` → `String` the same way `notify_email` does.

2. **EDIT** `crates/argo-cli/src/handlers/worker_actions_notify.rs` lines
   52-65 (the `#[path = ...] mod notify_*` block) — add a `notify_imessage`
   mount in alphabetical order.

3. **EDIT** `crates/argo-cli/src/handlers/worker_actions_notify_env.rs`:
   - line 1-4 `use` block: add `notify_imessage` to the import list.
   - line 25 dispatch table: insert
     `"imessage" => notify_imessage::send_imessage(ref_, n).await,`
     between the `discord` and `chat` arms.
   - line 28-30 error string: remove `imessage` from the DEFERRED-v1.x list
     and add it to the v1 channels list. Keep the new copy: `"v1 channels:
     ..., discord, imessage, chat, ...; DEFERRED-v1.x: whatsapp, teams,
     matrix"`.

4. **NEW** regression test in
   `crates/argo-cli/tests/notify_imessage_dispatcher_test.rs` (~50-80 LOC).
   The test must NOT hit a real BlueBubbles server. Pattern: build an
   `EnvChannelSender`, call `send_via` with `ref_.channel = "imessage"`,
   assert the returned error is the **missing-env** error from
   `required_env`, NOT the "channel kind 'imessage' is not supported"
   error. This pins that the dispatcher accepts the channel — the upstream
   transport's runtime config gap is a separate concern. Use the
   `notify_discord_dispatcher_test.rs` pattern as the template if it
   exists; otherwise mirror `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs`'s
   one-shot listener pattern for the success path (optional — the
   missing-env negative is sufficient acceptance).

## Acceptance

- [ ] `crates/argo-cli/src/handlers/worker_actions_notify_imessage.rs` exists
      and compiles.
- [ ] `EnvChannelSender::send_via` returns the missing-env error (not the
      "channel kind ... is not supported" error) when called with
      `channel = "imessage"`.
- [ ] `cargo clippy -p argo-cli --tests -- -D warnings` clean.
- [ ] `cargo test -p argo-cli --test notify_imessage_dispatcher_test` green.
- [ ] `cargo clippy -p argo-channels --all-targets -- -D warnings` clean
      (channel substrate is untouched, but the workspace should stay green).
- [ ] Workspace-clippy fast-fail clean.
- [ ] The pin set in `crates/argo-channels/tests/channels_v1_inventory_pin_test.rs`
      already includes `imessage`; this brief does NOT change that test.

## May-touch

- `crates/argo-cli/src/handlers/worker_actions_notify.rs` (mod mount only)
- `crates/argo-cli/src/handlers/worker_actions_notify_env.rs` (import + arm + error string)
- `crates/argo-cli/src/handlers/worker_actions_notify_imessage.rs` (NEW)
- `crates/argo-cli/tests/notify_imessage_dispatcher_test.rs` (NEW)

**Out of may-touch**: any `argo-channels/src/*.rs` (substrate untouched);
the pricing page (already advertises iMessage correctly); the other 3
DEFERRED-v1.x channels.

## Parallel surfaces

`rg 'fn send_imessage' crates/` returns zero hits — no parallel surface to
worry about. `rg 'IMessageChannel::' crates/` returns the existing test +
substrate; the new dispatcher module is the only new construction site.

## Sensible deviations

- If `IMessageChannelConfig::from_env()` doesn't exist, the dispatcher
  module can build the config inline from `required_env` calls — match
  whatever pattern `notify_email` / `notify_sms` use.
- If the BlueBubbles client needs an async constructor that returns
  `Result<_, ChannelError>`, surface that error through the `send_imessage`
  return type via `.map_err(|e| e.to_string())` — same as
  `notify_discord::send_discord`.
- If the pricing-page wording needs adjusting (it shouldn't — the page is
  the source of truth for the promise this brief honours), open a follow-up
  web-* brief and DO NOT edit it in this PR.
