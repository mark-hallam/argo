# demo-dry-run fixtures

Inputs consumed by `scripts/demo-dry-run.sh`.

| File                       | Purpose                                                |
| -------------------------- | ------------------------------------------------------ |
| `dry-run.mjs`              | Playwright scene walker (copied into `web/` at run)    |
| `slack_stub.py`            | Local Slack API stub on `127.0.0.1:4455`               |
| `first-hire-payload.json`  | Throwaway payload for `POST /api/v1/workers`           |

## Sanitization notes

- No real API keys, real email addresses, or real Slack tokens in this
  directory. The payload uses placeholder values only.
- The Slack stub responds with the minimum `{"ok": true, "ts": ...}`
  shape required by `crates/argo-mcp-connector-slack` and does not
  forward any data.

## Updating the wizard step list

If the SPA's `/onboarding/*` routes change, edit the `wizard` array in
`dry-run.mjs`. Each entry is `[step-slug, /forward-button-regex/i]`.
