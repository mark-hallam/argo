# V1.4 — Connector authorize CLI

**Owner**: codex. **Depends on**: codex-credentials-management-cli
(in flight) merged.

## Scope

Most MCP connectors need OAuth or token credentials. Today the
operator manually finds a token from each provider's UI, copies
it, sets an env var, hopes the connector picks it up. Replace
with `argo connector authorize <kind>`:

- For OAuth connectors (gmail, gcal, hubspot, notion, salesforce):
  open browser to the provider's auth URL, capture the redirect,
  exchange for tokens, store in `<workspace>/.argo/connectors/<kind>/`.
- For token connectors (slack, jira, asana, linear, monday,
  pagerduty, fireflies, airtable, stripe, zendesk, clickup,
  discord): prompt for token (hidden), validate via a no-op API
  call, store on success.
- `argo connector list-status` — show which connectors are
  authorized vs need auth.
- `argo connector revoke <kind>` — delete stored credentials.
- `argo connector test <kind>` — exercise a no-op call to verify
  credentials still work.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/connector_authorize.rs` (NEW)
- `crates/argo-cli/src/handlers/connector_list_status.rs` (NEW)
- `crates/argo-cli/src/handlers/connector_revoke.rs` (NEW)
- `crates/argo-cli/src/handlers/connector_test.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 4 pub mods
- `crates/argo-cli/src/cli_connector_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly: register
  `Connector` subcommand
- `crates/argo-cli/tests/connector_authorize_smoke.rs` (NEW)

## Acceptance criteria
- [ ] OAuth flow opens browser, captures redirect, stores tokens
- [ ] Token-auth connectors prompt + validate before storing
- [ ] `connector list-status` reports per-connector state
- [ ] `connector test <kind>` runs a real no-op call
- [ ] `connector revoke <kind>` deletes the stored creds
- [ ] `--json` envelope on all 4 subcommands
- [ ] 8 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-connector-authorize-cli`
- PR title: `cli: argo connector authorize/list-status/revoke/test (codex-connector-authorize-cli)`
