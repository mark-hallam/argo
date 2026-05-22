# Stream D — Slack approval channel

**Owner**: codex. **Depends on**: D-email merged (defines
`NotifyChannel` trait).

## Scope

`SlackApprovalChannel` impl. Posts an approval prompt to a
configured Slack channel or DM with interactive buttons
(approve / reject); receives the click via Slack's
`interactivity` payload at the inbound webhook.

This is DISTINCT from the Slack MCP connector (C-slack). The
MCP connector lets a worker post / read messages as part of
its work. This channel impl handles the operator's approval
flow.

## Files you may touch
- `crates/argo-channels/src/lib.rs` — narrowly scoped: add ONE
  line `pub mod slack;` (or `mod slack;`). No other edits.
- `crates/argo-channels/src/slack.rs` (NEW file in the existing
  crate)
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  (extend with `POST /channels/slack/webhook`)
- `manifests/channels/slack.toml`

## Files you must NOT touch
- `crates/argo-mcp-connector-slack/**` (different brief,
  different concern)
- Other channel impls
- `NotifyChannel` trait itself (locked by D-email)

## Acceptance criteria
- [ ] `SlackApprovalChannel` impl of `NotifyChannel`
- [ ] Posts a message with two buttons (approve, reject) using
      Slack Block Kit; carries the HMAC-signed token in the
      `value` field of each button
- [ ] Inbound webhook validates Slack's request signature AND
      our HMAC; settles approval queue
- [ ] Bot token via `SLACK_APPROVALS_BOT_TOKEN` (separate from
      the C-slack connector's token; different scopes)
- [ ] Per-operator config: which Slack workspace / channel /
      user to DM
- [ ] Integration test (gated) hits a test channel, asserts
      message posted with correct button payloads
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-D-slack-approvals`
- PR title: `channels: Slack approval channel with Block Kit buttons (D-slack-approvals)`
