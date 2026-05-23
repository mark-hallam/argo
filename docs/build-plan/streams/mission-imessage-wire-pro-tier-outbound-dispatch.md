# mission-imessage-wire-pro-tier-outbound-dispatch

**Owner**: claude or codex.
**Depends on**: nothing.
**Authored against**: origin/main `56015c0b9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-imessage-wire-pro-tier-outbound-dispatch`.

🟡 **P1 — Pro-tier sales promise refuses outbound iMessage at dispatch time.** Source: `docs/skycot/evals/scheduled/2026-05-23/eval-channels-v1-config-vs-code-inventory.md` (Finding 1).

`/pricing` Pro card (`web/app/pricing/page.tsx:64`) advertises `"Multi-channel (Slack / iMessage / Email)"`. The production outbound dispatcher (`crates/argo-cli/src/handlers/worker_actions_notify_env.rs:17-31`) refuses iMessage with:

```rust
other => Err(format!(
    "channel kind '{other}' is not supported by dispatch_notify \
     (v1 channels: push, webinbox, slack, email, sms, discord, chat, sse, voice; \
     DEFERRED-v1.x: imessage, whatsapp, teams, matrix)"
)),
```

The `IMessageChannel` impl is real (`crates/argo-channels/src/imessage.rs:56`, 342 LOC, backed by `argo-bluebubbles-client`), the env-var loader works (`from_env_default()` reads `BLUE_BUBBLES_URL` + `BLUE_BUBBLES_PASSWORD` + `ARGO_CHANNEL_HMAC_SECRET`), and the inbound webhook route is wired (`routes_channels_imessage.rs`, 200 LOC). Only the dispatcher branch is missing.

## Scope

1. Add `crates/argo-cli/src/handlers/worker_actions_notify_imessage.rs` (~50-80 LOC) — same shape as `worker_actions_notify_voice.rs` (which already passes through `IMessageChannel::send` via the rate-limit retry helper).
2. Add the `"imessage" => notify_imessage::send_imessage(ref_, n).await,` arm to `worker_actions_notify_env.rs:17` match.
3. Remove `imessage` from the `DEFERRED-v1.x:` substring in the no-match error message (leaves `whatsapp, teams, matrix` deferred per operator decision — see CLAUDE.md "v1.0 strategic decisions").
4. Update the `mod` line in `worker_actions/mod.rs` (or wherever sibling `notify_*` handlers are declared) to register the new module.
5. Add an env-var-driven integration test under `crates/argo-cli/tests/` modelled on `slack_approval_smoke.rs` that asserts `EnvChannelSender::send_via(&NotifyChannelRef::new("imessage", "<route>"), &n)` returns `Ok(())` when `BLUE_BUBBLES_URL` points at a local mock server (use `tokio::net::TcpListener` for the mock per the existing pattern).
6. Update the v1 channels list in the no-match error message string to include `imessage` between `slack` and `email` (preserves alphabetic-by-tier-promise ordering used in the Pro card).

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_notify_env.rs` — add the match arm + amend the error string.
- `crates/argo-cli/src/handlers/worker_actions_notify_imessage.rs` — NEW handler (~50-80 LOC).
- `crates/argo-cli/src/handlers/mod.rs` (or `worker_actions/mod.rs`) — register the module.
- `crates/argo-cli/tests/imessage_approval_smoke.rs` — NEW (~80 LOC) integration test.
- `docs/build-plan/streams/mission-imessage-wire-pro-tier-outbound-dispatch.md` — this file (status note on completion).

## Files you must NOT touch

- `crates/argo-channels/src/imessage.rs` — production code is fine as-is; do not refactor.
- `crates/argo-channels/src/imessage.rs::from_env_default()` — env var names are part of the Blue Bubbles operator contract.
- `web/app/pricing/page.tsx` — the sales promise stays; this brief makes the code honor it.

## Acceptance criteria

- [ ] `EnvChannelSender::send_via(&NotifyChannelRef::new("imessage", "<route>"), &n)` returns `Ok(())` with a live Blue Bubbles mock.
- [ ] No-match error message no longer lists `imessage` under DEFERRED-v1.x.
- [ ] New smoke test passes.
- [ ] `cargo clippy -p argo-cli --tests -- -D warnings` clean.
- [ ] Eval report `eval-channels-v1-config-vs-code-inventory.md` updated: iMessage row verdict flips 🔴 → 🟢, with a back-cite to the PR.
- [ ] BUILD_STATUS.md updated with PR number.

## Test plan

- Crate-scoped clippy + tests for `argo-cli`.
- The integration smoke test exercises the full send path: dispatcher → `IMessageChannel::send` → Blue Bubbles HTTP POST → mock 200 → `Ok(())`.
- Re-run `argo doctor channels` manually to confirm the iMessage row stays `OK` after the wire-up.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## Out of scope

- Re-classifying whatsapp/teams/matrix from DEFERRED-v1.x to v1 — operator decision per CLAUDE.md "v1.0 strategic decisions".
- Per-tenant credential scoping for Blue Bubbles — separate sweep (`mission-cred-get-tenant-parameter`).
- Building a `/settings/connectors` UI for iMessage — owned by sibling brief `web-settings-add-approval-channels-config-page`.

~150-200 LOC across handler + smoke test + env match arm. Single-crate touch.
