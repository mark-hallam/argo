# `argo wizard onboard` — runbook

End-to-end first-operator setup. One command replaces the legacy
five-step bootstrap (`argo setup` → hand-edit `argo.toml` → `argo
new`). Telemetry-free: nothing is sent to any external service
except the explicit 1-token Anthropic key-validation call (~$0.001).

## Interactive

```
argo wizard onboard
```

Five prompts:

1. **detect** — prints platform + arch + CLI version (no input).
2. **provider key** — paste your Anthropic key. A 1-token call to
   `api.anthropic.com/v1/messages` confirms the key works.
3. **operator identity** — your name + email. Written to
   `argo.toml [operator]`.
4. **first vertical** — defaults to `marketplace-seller-browser`.
   Any id in `argo vertical list` is valid.
5. **first worker** — worker name (`/^[a-z][a-z0-9_-]{2,30}$/`) +
   a one-line persona summary.

On success the wizard prints next-step commands:

```
argo chat <worker>   — open a chat session
argo handoff list    — see active handoffs
argo doctor          — workspace health check
```

If `[operator]` already exists, the wizard asks "overwrite [y/N]?"
and defaults to **No** so it's safe to re-run.

## Non-interactive

```
argo wizard onboard --non-interactive --answers answers.toml
```

`answers.toml`:

```toml
provider_api_key = "sk-ant-..."
operator_name    = "Ada Lovelace"
operator_email   = "ada@example.com"
vertical         = "marketplace-seller-browser"
worker_name      = "ada"
worker_persona   = "first worker — answers customer inbound"

# Optional flags
skip_key_test      = false   # airgapped operators: set true
overwrite_existing = false   # idempotency: set true to replace an
                             # existing [operator] block without prompt
```

Same flow, no prompts. Every required answer must be present; bad
input (invalid email, malformed worker name, non-existent vertical)
errors out with a clear message before anything is written.

## Airgapped

Set `skip_key_test = true` in the answers file. The wizard accepts
the key verbatim and persists it to `.argo/credentials.toml`
without contacting Anthropic. The operator is responsible for
verifying the key out-of-band.

## What gets written

| Path | Contents |
| --- | --- |
| `<cwd>/argo.toml` | `[operator]` block (name, email, provider). Other sections preserved. |
| `<cwd>/.argo/credentials.toml` | `ANTHROPIC_API_KEY = "..."` (mode 0600). |
| `<cwd>/workers/<name>/worker.toml` | Worker manifest: name + vertical + persona. |

## Re-running

The wizard is idempotent. The first thing it checks is whether
`argo.toml` already has an `[operator]` block — if so it prompts
before overwriting. Aborting (default No) leaves every file
untouched, so a half-finished run is recoverable.
