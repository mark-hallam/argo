# V1.4 — Credentials fallback wiring

**Owner**: codex. **Depends on**: codex-credentials-management-cli
merged.

## Scope

Companion brief to `codex-credentials-management-cli`. That brief
ships the storage surface (`argo cred set/list/clear/export/import`)
and a helper `argo_config::cred::get(key)`; this brief threads
that helper through every existing `std::env::var(...)` call site
in the codebase that today reads operator-supplied tokens.

For each occurrence: replace
```rust
std::env::var("FOO")
```
with
```rust
argo_config::cred::get("FOO")
```

Wiring is mechanical, one-line-per-call, no behaviour change other
than the new fallback path (env unset → read credentials.toml).

## Surfaces to wire

NotifyChannel implementations:
- `crates/argo-channels/src/email.rs` — POSTMARK_API_KEY
- `crates/argo-channels/src/sms.rs` — TWILIO_AUTH_TOKEN /
  TWILIO_ACCOUNT_SID / TWILIO_FROM
- `crates/argo-channels/src/push.rs` — EXPO_ACCESS_TOKEN
- `crates/argo-channels/src/slack.rs` — SLACK_BOT_TOKEN /
  SLACK_SIGNING_SECRET
- `crates/argo-channels/src/imessage.rs` — BLUEBUBBLES_URL /
  BLUEBUBBLES_PASSWORD

Connector crates (one env::var per crate, varies):
- `crates/argo-mcp-connector-*/src/auth.rs` (~14 crates) —
  per-connector token env name

LLM clients (whichever read ANTHROPIC_API_KEY directly):
- run `rg "env::var.*ANTHROPIC" crates/` to find all sites.

## Files you may touch
- `crates/argo-channels/src/{email,sms,push,slack,imessage}.rs`
  — narrowly: replace env::var calls with argo_config::cred::get
- `crates/argo-mcp-connector-*/src/auth.rs` — narrowly: same
- Any crate found by `rg "env::var" crates/` that reads operator
  tokens (gmail/gcal/asana access tokens, etc.) — narrowly:
  same one-line substitution
- `crates/argo-channels/Cargo.toml` — narrowly: dep on
  argo-config if not already present
- `crates/argo-mcp-connector-*/Cargo.toml` — narrowly: same

DO NOT touch:
- Any function body other than the env::var → cred::get line
- Tests (these set env vars in tests; should keep using std::env)

## Acceptance criteria
- [ ] Every operator-token env::var call in argo-channels routed
      through argo_config::cred::get
- [ ] Every operator-token env::var call in argo-mcp-connector-*
      routed through argo_config::cred::get
- [ ] LLM clients (anthropic) routed through argo_config::cred::get
- [ ] `cargo test --workspace` passes (no behaviour regression;
      all existing env-var tests still pass since cred::get
      tries env first)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] No new tests required (mechanical refactor; behaviour
      covered by the new helper's unit tests in #credentials-management-cli)

## Branch + PR
- Branch: `codex-credentials-fallback-wiring`
- PR title: `cred: route operator-token env::var calls through credentials.toml fallback (codex-credentials-fallback-wiring)`
