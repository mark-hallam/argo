# eval-channels-v1-config-vs-code-inventory

**Authored**: 2026-05-23 (evaluator, claude worker).
**Audited surface**: `crates/argo-channels/src/`, `crates/argo-cli/src/handlers/worker_actions_notify_*.rs`, `crates/argo-cli/src/handlers/gateway/routes_channels_*.rs`, `web/app/pricing/page.tsx`, `docs/skycot/marketing/PRICING.md`, `web/app/(app)/settings/`.
**Audited against**: origin/main `56015c0b9` on 2026-05-23.
**Brief**: `docs/build-plan/streams/eval-channels-v1-config-vs-code-inventory.md`.
**Gap-manifest row touched**: `argo-channels` (line 90 of `mission-vs-code-gap-manifest.md`).

## TL;DR

- **Channel count is 11, not 13.** The gap-manifest's "13 channel impls" tally was wrong; `crates/argo-channels/src/` contains exactly **11** distinct channel-id implementations (discord, email, imessage, matrix, push, slack, sms, teams, voice, webinbox, whatsapp). The other `.rs` files in the directory are infrastructure (dispatch, manifest, message_dispatcher, fallback, operator_*, quiet_hours, rate_limit, token, *_format, *_payload, *_inbound).
- **🟢 iMessage sales-promise gap is closed.** The Pro-tier card still promises `"Multi-channel (Slack / iMessage / Email)"`, and the production outbound dispatcher now routes all three. The pricing claim register tracks this as a live Pro promise at `docs/skycot/marketing/PRICING.md`.
- **No OAuth-style approval-channel Connect button exists yet.** The earlier audit pre-dated `/settings/notify-channels`, which now provides a read-mostly operator catalog and env-var copy surface. Approval channels are still configured through env vars / workspace credentials; OAuth Connect buttons wait on per-tenant secret storage.

## Method

For each of the 11 channel ids, I checked:

1. **Impl present** — `crates/argo-channels/src/<channel>.rs` exists, `id() -> ChannelId::new("<channel>")` declared, `send()` performs a real provider call (HTTP POST or platform-specific client) and not `todo!()`/`unimplemented!()`.
2. **Outbound wired** — branch exists in `worker_actions_notify_env.rs::EnvChannelSender::send_via` match.
3. **Inbound wired** — a `*_inbound.rs` / `parse_webhook()` substrate exists in `argo-channels` AND a `routes_channels_<channel>.rs` route in `argo-cli/src/handlers/gateway/`.
4. **Operator-configurable** — env var or `argo doctor channels` probe surface exists (all rows alike); `/settings/notify-channels` is now a read-mostly catalog, not an OAuth Connect surface.
5. **Tier** — appears in the `TIERS` constant on `/pricing` (`web/app/pricing/page.tsx:37-93`).

Verdict legend:
- 🟢 **LIVE-in-v1.0** — promised in a v1.0 tier card AND impl-wired-routed end-to-end.
- 🟡 **SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0** — code-shipped + wired but never promised to a paying tier; no operator-facing config surface beyond env vars.
- 🔴 **STUB-OR-NOT-WIRED** — either the outbound dispatcher refuses the channel, or critical send-path links are missing, or the channel is in a v1.0 tier card without a working dispatch path.

## Matrix

| channel | impl | outbound | inbound | config | tier | verdict |
|---|---|---|---|---|---|---|
| **email** | ✅ Postmark HTTP (`email.rs:46`, 214 LOC) | ✅ `notify_email::send_email` | ✅ `EmailInbound` (stub, Postmark-shape) + `routes_channels_email::handle_email_webhook` | env vars (`POSTMARK_*`, `ARGO_CHANNEL_HMAC_SECRET`); no SPA Connect | **Pro** (Multi-channel) | 🟢 LIVE-in-v1.0 |
| **slack** | ✅ chat.postMessage (`slack.rs:66`, 321 LOC) | ✅ `notify_slack::send_slack` | ✅ `SlackInbound` (HMAC-verified, real) + `routes_channels_slack::slack_webhook_response` | env vars (`SLACK_APPROVALS_BOT_TOKEN`, `SLACK_APPROVALS_SIGNING_SECRET`); no SPA Connect | **Pro** (Multi-channel) | 🟢 LIVE-in-v1.0 |
| **imessage** | ✅ Blue Bubbles client (`imessage.rs:56`, 342 LOC; uses `argo-bluebubbles-client`) | ✅ `notify_imessage::send_imessage` (wired via `mission-imessage-wire-pro-tier-outbound-dispatch`, claude PR) | ✅ `IMessageChannel::parse_webhook` + `routes_channels_imessage::imessage_webhook_response` (200 LOC) | env vars (`BLUE_BUBBLES_URL`, `BLUE_BUBBLES_PASSWORD`, `BLUE_BUBBLES_WEBHOOK_SECRET`); no SPA Connect | **Pro** (Multi-channel) | 🟢 LIVE-in-v1.0 — sales-promise gap closed by `mission-imessage-wire-pro-tier-outbound-dispatch` |
| **webinbox** | ✅ JSONL local file (`webinbox.rs:43`, 359 LOC) | ✅ `notify_env::send_webinbox` (inline) | ✅ `routes_channels_webinbox::webinbox_response` (378 LOC; SSE-eventable approval inbox) | env var `ARGO_WEBINBOX_PATH`; web inbox UI surfaces the events | **Free** (`"Single channel (web inbox)"`); implicit in Pro+Ent | 🟢 LIVE-in-v1.0 |
| **discord** | ✅ webhook (`discord.rs:30`, 259 LOC) | ✅ `notify_discord::send_discord` | ✅ Button-decision route `routes_channels_discord` (127 LOC; `is_discord_decision` / `discord_decision_response`) | env vars (`DISCORD_*`); no SPA Connect | **(none — not in any tier card)** | 🟡 SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0 |
| **push** | ✅ Expo Push HTTP (`push.rs:50`, 296 LOC) | ✅ `notify_env::send_push` (inline) | ✅ button decision route `routes_channels_push` (108 LOC) | env vars (`EXPO_PUSH_ENDPOINT`, `EXPO_ACCESS_TOKEN`); no SPA Connect | **(none — not in any tier card)** | 🟡 SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0 |
| **sms** | ✅ Twilio Messages API (`sms.rs:49`, 290 LOC) | ✅ `notify_sms::send_sms` | 🔴 no inbound — no `routes_channels_sms.rs`, no Twilio-SMS webhook | env vars (`TWILIO_*`); no SPA Connect | **(none — not in any tier card)** | 🟡 SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0 (outbound-only; mission-claim-6 inbound gap is owned by sibling brief `eval-channels-inbound-coverage`) |
| **voice** | ✅ Twilio Voice (`voice.rs:61`, 336 LOC) | ✅ `notify_voice::send_voice` | ✅ DTMF webhook `routes_channels_voice::voice_webhook_response` (229 LOC; verifies Twilio HMAC) | env vars (`TWILIO_*`); no SPA Connect | **(none — not in any tier card)** | 🟡 SHIPPED-BUT-NOT-CONFIGURED-IN-V1.0 |
| **whatsapp** | ✅ Twilio WhatsApp (`whatsapp.rs:50`, 325 LOC) | 🔴 REFUSED — `worker_actions_notify_env.rs:30` lists `whatsapp` in `DEFERRED-v1.x` | ✅ `routes_channels_whatsapp::whatsapp_webhook_response` (295 LOC; Twilio HMAC + keyword parser) | env vars (`TWILIO_*`); no SPA Connect | **(none — not in any tier card)** | 🔴 STUB-OR-NOT-WIRED — outbound asymmetry (inbound webhook exists but dispatcher won't route outbound) |
| **teams** | ✅ MS Graph (`teams.rs:78`, 427 LOC; OAuth client-creds + Bot Framework activity) | 🔴 REFUSED — `worker_actions_notify_env.rs:30` lists `teams` in `DEFERRED-v1.x` | 🔴 no inbound — no `routes_channels_teams.rs` and no parse path | env vars (`TEAMS_*`, `<client_secret_env>`); no SPA Connect | **(none — not in any tier card)** | 🔴 STUB-OR-NOT-WIRED |
| **matrix** | ✅ Matrix CS API (`matrix.rs:72`, 302 LOC) | 🔴 REFUSED — `worker_actions_notify_env.rs:30` lists `matrix` in `DEFERRED-v1.x` | 🔴 no inbound — no `routes_channels_matrix.rs` | env vars (`MATRIX_HOMESERVER`, `<access_token_env>`); no SPA Connect | **(none — not in any tier card)** | 🔴 STUB-OR-NOT-WIRED |

### Pseudo-channels also routed by the dispatcher (out of scope for this matrix)

`worker_actions_notify_env.rs:24-25` also routes `"chat"` (generic HTTP webhook → `CHAT_WEBHOOK_URL`) and `"sse"` (server-sent-event push → `SSE_NOTIFY_URL`). Neither is a `NotifyChannel` impl in `argo-channels` — they live entirely in `argo-cli/src/handlers/worker_actions_notify_chat.rs` / `worker_actions_notify_sse.rs` and don't carry a channel id. They are not included in the 11-row matrix above; they are wire-level conveniences for operators who want to fan approval payloads to a Zapier-style endpoint or their gateway's SSE bus without standing up a real channel provider.

## Three most-surprising findings

### Finding 1 — iMessage is now in the Pro-tier sales promise and the outbound dispatcher

`/pricing` Pro card (`web/app/pricing/page.tsx:65`):

```
"Multi-channel (Slack / iMessage / Email)"
```

Production outbound path (`crates/argo-cli/src/handlers/worker_actions_notify_env.rs:17-31`):

```rust
match ref_.channel.as_str() {
    "push" => send_push(n).await,
    "webinbox" => send_webinbox(n).await,
    "slack" => notify_slack::send_slack(n).await,
    "imessage" => notify_imessage::send_imessage(ref_, n).await,
    "email" => notify_email::send_email(ref_, n).await,
    "sms" => notify_sms::send_sms(ref_, n).await,
    "discord" => notify_discord::send_discord(n).await,
    "chat" => notify_chat::send_chat(n).await,
    "sse" => notify_sse::send_sse(n).await,
    "voice" => notify_voice::send_voice(ref_, n).await,
    other => Err(format!(
        "channel kind '{other}' is not supported by dispatch_notify \
         (v1 channels: push, webinbox, slack, imessage, email, sms, discord, chat, sse, voice; \
         DEFERRED-v1.x: whatsapp, teams, matrix)"
    )),
}
```

The historical finding is therefore closed by `mission-imessage-wire-pro-tier-outbound-dispatch` / PR #2161. The canonical pricing register records `"Multi-channel (Slack / iMessage / Email)"` as a green Pro claim and cites this report as the channel inventory back-stop.

### Finding 2 — Inbound webhook routes exist beyond the original two-path claim

The gateway has fully-routed inbound handlers for `whatsapp` (Twilio HMAC + keyword parser, 295 LOC) and `imessage` (Blue Bubbles webhook, 200 LOC). WhatsApp remains asymmetric because outbound dispatch is still deferred; iMessage is now symmetric after #2161. The two routes are exercised by integration tests (`whatsapp_smoke.rs`, `claim_6_outbound_signed_token_e2e.rs`).

This is mostly harmless (no outbound = no inbound traffic to attack), but it inflates the apparent "is wired" surface in the gap manifest's "Channel inbound paths" row, which currently claims `email_inbound.rs` + `slack_inbound.rs` are the only inbound paths. In reality the gateway has 8 inbound webhook routes (discord-decision, email-Postmark, imessage-BlueBubbles, push-button, slack-callback, voice-DTMF, webinbox-internal, whatsapp-Twilio). The sibling brief `eval-channels-inbound-coverage` should own correcting that row; this report just flags the count mismatch.

### Finding 3 — Approval-channel config UI is read-mostly, not a Connect flow

The brief asked whether each channel exposes a "Connect button via `/settings/connectors/page.tsx`". That exact page is for work-tool MCP connectors, not approval channels. Approval channels now have `/settings/notify-channels`, which lists the 11 channel ids, env vars, and audit fallback status, but it does not yet store per-tenant secrets or run OAuth-style Connect flows. The `slack` overlap remains easy to misread: the work-tool Slack connector is distinct from the SLACK_APPROVALS_* approval channel.

Concretely: a Pro customer who needs to set up Slack-for-approvals today still must:
1. Create a separate Slack app for approvals (different from the Slack MCP work-tool app).
2. Set `SLACK_APPROVALS_BOT_TOKEN`, `SLACK_APPROVALS_SIGNING_SECRET`, `SLACK_APPROVALS_TEST_CHANNEL` env vars.
3. Run `argo doctor channels` to verify.
4. Restart the gateway so `routes_channels_slack` picks up the secrets.

The notify-channels page makes the env-var names discoverable, but the operator still owns the credential provisioning. This was implicit in the v1.0 scope (operator-locked: enterprise focus, single-binary, env-driven), and it remains worth tracking because the brief's premise of "matrix per /settings/connectors Connect button" still does not hold for approval channels.

**Follow-up fix-brief status**: `web-settings-notify-channels-config-page` has delivered the read-mostly catalog; per-tenant secret storage / Connect buttons remain later work.

## Cross-cutting observations

- **Tier promise vs verdict count**: 4 channel mentions are promised in v1.0 tier cards (Free: webinbox; Pro: slack + imessage + email). All 4 tier-card-mentioned promises are now 🟢 LIVE-in-v1.0. The remaining 7 channels are either 🟡 shipped/configurable but not promised, or 🔴 deferred by outbound dispatch. The pricing register at `docs/skycot/marketing/PRICING.md` is now the canonical sales-promise tracker.
- **Free tier is unambiguously honest** — `"Single channel (web inbox)"` matches the implementation: webinbox is the only channel that requires no external provider keys (it writes to local JSONL) and is the default fallback target when no `[notify_channels]` is declared.
- **Enterprise tier is silent on channels.** The Enterprise tier card lists `"SSO / SAML"`, `"Audit export"`, `"Custom verticals"`, `"On-prem"` — but no channel promises. By implication everything Pro has is in Enterprise; the matrix above and the pricing register assume this.

## Gap-manifest row update

The `argo-channels` row in `mission-vs-code-gap-manifest.md` (line ~90) has been updated:
- "13 channel impls present" → "11 channel impls present" (counted directly from `crates/argo-channels/src/` rather than from the original brief author's tally).
- Verdict cell now records the post-#2161 split: 4 🟢 / 4 🟡 / 3 🔴 with cite to this report.
- "Pro tier promise" line: explicit call-out that iMessage moved from red to green after the dispatcher branch landed.

## Follow-up briefs authored this tick

- `streams/mission-imessage-wire-pro-tier-outbound-dispatch.md` — shipped via #2161; iMessage is no longer a Pro-tier sales-promise gap.
- `streams/web-settings-notify-channels-config-page.md` — shipped the `/settings/notify-channels` read-mostly catalog; per-tenant secret storage / OAuth Connect buttons remain later work.

## Pin test

`crates/argo-channels/tests/channels_v1_inventory_pin_test.rs` asserts that the 11 channel ids found by this audit remain present in `argo-channels`. The test imports each channel impl module's `id()` (or constructs the impl with `disabled_for_tests`/`local`/`from_env_default`) and pins the resulting `ChannelId` string. Any future delete or rename will fail to compile or fail the assertion, forcing the audit and the gap-manifest row to be re-validated rather than silently drifting.

## Out of scope (called out explicitly)

- **Fixing remaining 🔴 channels.** The 3 🔴 rows (WhatsApp, Teams, Matrix) are recorded for follow-up briefs; this report doesn't touch their source.
- **Per-channel cross-tenant isolation.** Owned by sibling brief `eval-channels-cross-tenant-scope-sweep`.
- **Per-channel inbound coverage as its own matrix.** Owned by sibling brief `eval-channels-inbound-coverage`. This report's inbound column is informational only.
- **Pricing copy edits.** Pricing copy is tracked in `docs/skycot/marketing/PRICING.md`; this report only updates the channel-inventory evidence that backs it.
