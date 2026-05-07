# mission-notify-channels-rest-of-pack — wire the 6 stub NotifyChannel kinds (Phase A follow-up to #931)

**Owner**: codex. **Depends on**: nothing — all infrastructure already merged via #931 (mission-wire-outbound-notify-on-ask).

## Why

#931 wired notification dispatch on `Decision::Ask` but only **2 of 8** NotifyChannel kinds actually deliver:
- ✅ **push** (env-driven; works)
- ✅ **webinbox** (in-process; works)
- ⚠️ **slack / sms / email / discord / chat / sse** — return a clear `"not yet wired"` error so the gap is visible but the v1 ship was bounded

This brief wires the remaining 6, completing **mission claim 6** ("multi-channel approvals work end-to-end").

## Scope

For each of the 6 currently-stubbed channel kinds in `EnvChannelSender::send_notification` (in `crates/argo-cli/src/handlers/worker_actions_notify.rs` per the #931 merge), replace the `Err(_)` stub with a real send path:

1. **slack** — POST to incoming webhook URL via `SLACK_WEBHOOK_URL` env (or workspace-level `[notify.slack]` from `.skycot/notify.toml`). Body: small JSON payload with action_id + summary + approve/deny links. Reuses workspace `reqwest` dep — no new deps.

2. **email** — SMTP send via `lettre` crate (already a workspace dep — verify in Cargo.lock). Env: `SMTP_HOST` / `SMTP_PORT` / `SMTP_USER` / `SMTP_PASS` / `SMTP_FROM`. Subject + body templates.

3. **sms** — Twilio REST API via `TWILIO_ACCOUNT_SID` / `TWILIO_AUTH_TOKEN` / `TWILIO_FROM` env. POST to `https://api.twilio.com/2010-04-01/Accounts/{sid}/Messages.json`. Reuses `reqwest`.

4. **discord** — POST to incoming webhook URL via `DISCORD_WEBHOOK_URL` env. Discord webhook payload shape (NOT the full bot API — that's `codex-channel-discord-approvals` separately). Reuses `reqwest`.

5. **chat** — generic webhook fallback: POST to URL given by `CHAT_WEBHOOK_URL`, JSON body with action_id + summary. Useful for Mattermost/Rocket.Chat/MS Teams generic incoming webhooks.

6. **sse** — push to a server-sent-events broadcaster (already exists in argo-channels — verify path). Send a `notify_ask` event keyed by action_id. If the SSE broadcaster has no subscribers, log + continue (don't fail the action).

For each channel: missing required env → return clear `"<channel> not configured: set <env-var>"` error so operators can self-diagnose.

## Approach (concrete)

1. Each channel gets its own small `send_<kind>` private function (~30-40 LOC each) inside `worker_actions_notify.rs` — same module as #931, single source of truth.
2. **If `worker_actions_notify.rs` would exceed 500 LOC** after adding all 6 — split into per-channel siblings under a new `worker_actions_notify_*.rs` filename family. Mirror the proven split pattern.
3. The dispatcher `EnvChannelSender::send_notification` becomes a `match channel.kind { ... }` over all 8 variants, each calling its specialized helper.
4. **Tests**: per-channel unit tests with mock HTTP (where reqwest is involved) — assert payload shape, env-var failure paths, and success-on-2xx behaviour. Reuse the test pattern from `mcp_action_executor_smoke.rs` / `api_action_executor_smoke.rs` if any channel needs an integration smoke.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_actions_notify.rs` — narrowly: replace 6 stub arms; if total exceeds 500 LOC, split (see below).
- `crates/argo-cli/src/handlers/worker_actions_notify_<channel>.rs` (NEW, one per channel that doesn't fit) — only if needed for LOC budget.
- `crates/argo-cli/src/handlers/worker_actions_notify_tests.rs` (NEW or extend existing #931 sibling) — 12+ unit tests (2 per channel: env-missing + happy path).
- `crates/argo-cli/Cargo.toml` — narrowly: add `lettre` to deps if not already there (likely already a workspace dep)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 `pub mod` line per new sibling
- `docs/skycot/RUNBOOK.md` — narrowly: §approvals, list each channel's required env vars

## Files you must NOT touch

- `argo-channels` crate internals — this brief consumes the existing channel kinds; no schema change to `NotifyChannel`/`NotifyChannelRef`.
- `argo-identity` — Worker.channels stays as-is; this brief reads workspace-level `[notify.<kind>]` config (matches #931's deviation note).
- `routes_orchestrator.rs` or webinbox routes — webinbox already works; this brief touches the OUTBOUND side only.
- Anything in `argo-mcp-connector-discord` (#953) — that's the bot-API connector for the inbound side; this brief uses Discord *webhooks* for outbound notify, which is simpler and doesn't need a bot.

## Acceptance criteria

- [ ] All 8 NotifyChannel kinds dispatch successfully when their env is configured (push, webinbox unchanged from #931 + slack/sms/email/discord/chat/sse newly wired)
- [ ] Missing required env per channel → clear error message naming the missing var
- [ ] 12+ unit tests covering happy path + env-missing per channel
- [ ] If file exceeds 500 LOC → split by channel into sibling modules (mirror proven pattern)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] RUNBOOK §approvals lists every channel's required env variables

## Branch + PR

- Branch: `codex-mission-notify-channels-rest-of-pack`
- PR title: `notify: wire remaining 6 NotifyChannel kinds (mission-notify-channels-rest-of-pack)`

## Notes for the implementer

- This is the **completion-pass** for mission claim 6 — every channel kind a workspace can declare must actually deliver.
- For SMS/email/discord-webhook/slack-webhook: these are well-documented HTTP-only protocols. The implementations should be ~30 LOC each. Don't overthink.
- If `lettre` isn't in workspace deps, add it. If it is, no Cargo.toml changes needed.
- For SSE: confirm the existing broadcaster API in `argo-channels` (likely `Broadcaster::publish(action_id, event)` or similar) — DO NOT extend it. Just call the existing API.
- If during implementation you hit a "the integration here would need a new dep / new schema" — STOP, mark the channel as still-stubbed, and call it out in the PR body. Better to land 4-5 of 6 cleanly than block on the trickiest one. The boundary-deviation pattern from #931 (HTTP caller in argo-cli not argo-skill / stdin not nightly tick) is fine here.
- Mission claim 6 ("multi-channel approvals work") is currently ~30% delivered (push + webinbox only). This brief takes it to ~95%.
