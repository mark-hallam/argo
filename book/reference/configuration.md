# Configuration

Argo's configuration surface is small by design. Most behavior is
derived from the workspace state (the SQLite store), not from
config files.

## Files

| File | Purpose |
|---|---|
| `<workspace>/argo.toml` | Workspace config (gateway, providers) |
| `<workspace>/.skycot/notify.toml` | Channel manifests |
| `<workspace>/.skycot/credentials.toml` | Provider keys (mode 0600) |
| `<workspace>/.skycot/auto_approval.toml` | Workspace default policy |
| `<workspace>/.skycot/onboarding.toml` | `argo onboard --non-interactive` inputs |
| `<workspace>/.skycot/verticals/<id>/vertical.toml` | Vertical manifest |
| `~/Library/LaunchAgents/com.skycot.argo.plist` | macOS daemon (optional) |

## `argo.toml`

Workspace config. Created by `argo workspace init`.

```toml
[workspace]
name = "my-seller"
created_at = "2026-01-15T10:00:00Z"

[gateway]
enabled = true
adapters = ["slack", "imessage"]
```

## `.skycot/credentials.toml`

Provider keys. Always mode `0600`. Set via `argo cred`:

```bash
argo cred set ANTHROPIC_API_KEY sk-ant-…
argo cred set OPENAI_API_KEY sk-…
argo cred set EXPO_PUSH_TOKEN ExponentPushToken[…]
argo cred list
argo cred clear ANTHROPIC_API_KEY
```

`argo_config::cred::get(key)` resolves env-first, then falls back
to this file.

## `.skycot/notify.toml`

Channel manifests. One section per channel:

```toml
[push]
expo_token = "ExponentPushToken[…]"
device_id = "device-uuid"

[email]
postmark_token_env = "POSTMARK_TOKEN"
inbound_address = "approvals@yourco.example"

[slack]
bot_token_env = "SLACK_BOT_TOKEN"
channel = "#argo-approvals"

[sms]
twilio_account_sid_env = "TWILIO_ACCOUNT_SID"
twilio_auth_token_env = "TWILIO_AUTH_TOKEN"
from = "+15551234567"
to = "+15557654321"
```

`*_env` keys reference an entry in `credentials.toml` or env. The
trait `NotifyChannel` resolves them at dispatch time.

## `.skycot/auto_approval.toml`

Workspace default policy. Worker-instance and roster policies can
tighten further.

```toml
[auto_approval]
max_spend_usd = 5.0
max_spend_micros = 5_000_000
max_actions_per_day = 50
require_sigs = 1
wake_action_types = ["mcp", "document_creation"]

[auto_approval.quiet_hours]
start = "22:00"
end = "08:00"
timezone = "America/Los_Angeles"
```

Caps tighten only along the lineage `worker ← roster ← workspace`.
Signatures escalate to the maximum. Quiet hours always apply at
the lowest level.

## Vertical manifest

`vertical.toml` declares the role / persona / memory schema /
allowed executors.

```toml
[vertical]
id = "gtm-engineer"
name = "GTM Engineer"
archetype = "marketplace_seller"

[persona]
prompt_pack = "system_prompt.md"

[memory]
schema = "memory_schema.toml"

[executors]
allowed = ["mcp", "computer_use", "document_creation"]
```

## Environment variables

A small set of env vars override file-based config:

| Env | Effect |
|---|---|
| `ARGO_HOME` | Override `~/.argo/` |
| `ARGO_VERTICALS_DIR` | Add a directory to the vertical search path |
| `ARGO_CIRCUIT_THRESHOLD` | Tweak the consecutive-fail breaker |
| `ARGO_DOC_CHROME` | Override the headless Chrome path for PDF |
| `ARGO_E2B_TOKEN` | Backend token for code-execution sandbox |
| `ARGO_CODE_EXEC_INTEGRATION` | Gate code-exec integration tests |
| `ARGO_TEST_*` | Various test-only fixture controls |

## Single-binary

The platform ships as a single binary (`argo`). All configuration
is read from the cwd-relative workspace; nothing is required at
install time.

## Hard rules CI enforces

These shape what config can be added:

- Every new schema needs an additive migration. Drops are forbidden.
- Every new env var needs a doc entry and a default that doesn't
  break the demo path.
- Tests for every feature.

The full list is in [Reference → Invariants](./invariants.md).
