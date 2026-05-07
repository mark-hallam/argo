# Set up approval channels

Approvals are how Argo asks a human to confirm an action before
execution. Channels are pluggable — install only the ones your
team actually uses.

## Channels Argo ships

| Channel | Use for |
|---|---|
| `push` | Mobile push via Expo (APNs / FCM) |
| `email` | Inbound webhook (Postmark) |
| `slack` | Block Kit buttons + HMAC-signed webhook |
| `sms` | Twilio SMS, HMAC token decision route |
| `voice` | Twilio voice + DTMF capture |
| `imessage` | Blue Bubbles bridge |
| `whatsapp` | Twilio WhatsApp |
| `webinbox` | Embedded HTML inbox + WS push |

## Configure a channel

Each channel has a small TOML file in
`<workspace>/.skycot/notify.toml`. The smallest setup:

```toml
[push]
expo_token = "ExponentPushToken[…]"

[email]
postmark_token = "…"
inbound_address = "approvals@yourco.example"
```

Set credentials with `argo cred set` (preferred — they end up at
mode `0600`):

```bash
argo cred set EXPO_PUSH_TOKEN "ExponentPushToken[…]"
argo cred set POSTMARK_TOKEN "…"
```

## Doctor your channels

```bash
argo doctor channels
```

Each channel runs a connectivity check (5 s reqwest timeout) and
reports `OK / WARN / ERROR / MISSING`. Fix anything not `OK`
before relying on the channel for production approvals.

## Test the full flow

The `gtm-engineer` smoke covers approval dispatch end-to-end:

```bash
./scripts/smoke-skycot.sh
```

For a single channel, force an action into escalation by lowering
the auto-approval threshold for the worker:

```bash
argo worker update <id> --auto-approval-json '{"max_spend_micros": 0}'
```

Any action that costs more than zero micros now escalates and
fires a notify event to every channel in `notify.toml`.

## What about budgets?

Channel notify is one half of the approval gate. The other half is
the spend budget — an action that exceeds the worker's daily or
monthly cap auto-escalates regardless of the auto-approval policy.
See [Reference → Configuration](../reference/configuration.md) for
the full policy schema.

## Next

- [How-to → Troubleshoot a stuck worker](./troubleshoot.md) when an
  approval doesn't arrive.
- [Operations → Approvals and audit](../operations/approvals-and-audit.md)
  for the steady-state operator playbook.
