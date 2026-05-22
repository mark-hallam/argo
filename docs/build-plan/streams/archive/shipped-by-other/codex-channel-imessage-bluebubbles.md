# V1.4 — iMessage approval channel via Blue Bubbles

**Owner**: codex. **Depends on**: D-email (#810) merged.

## Scope

iMessage is the dominant text-messaging channel on Apple
devices. Operators want approval pings on iMessage. **Blue
Bubbles** (https://bluebubbles.app) is the standard
open-source bridge: a server runs on the operator's Mac and
exposes iMessage over REST + WebSocket.

Add it as both:
1. **NotifyChannel** — outbound approval pings ("approve action
   X? reply YES").
2. **MCP connector** — workers can send iMessages as
   actions (e.g., notify a teammate).

Auth: `BLUE_BUBBLES_URL` (server URL) + `BLUE_BUBBLES_PASSWORD`
(server password set during BB setup).

## Files you may touch

NotifyChannel side:
- `crates/argo-channels/src/imessage.rs` (NEW — IMessageChannel
  impl)
- `crates/argo-channels/src/lib.rs` — narrowly: re-export

MCP connector side (separate crate, mirror gmail/slack
pattern):
- `crates/argo-mcp-connector-imessage/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-imessage/src/{lib,main,auth,client,schemas,service}.rs`
  (NEW)
- `Cargo.toml` workspace — narrowly: add new member

Both share a `crates/argo-bluebubbles-client/` HTTP/WS client crate so
neither owns the wire shape:
- `crates/argo-bluebubbles-client/Cargo.toml` (NEW)
- `crates/argo-bluebubbles-client/src/lib.rs` (NEW —
  send_message, list_chats, list_messages, send_reaction,
  webhook validate-signature)

## Acceptance criteria
- [ ] NotifyChannel sends iMessage approval pings + receives
      replies via webhook
- [ ] MCP connector exposes 5 methods (send_message, list_chats,
      list_messages, get_message, send_reaction)
- [ ] Read methods autonomous-safe; sends approval-gated
- [ ] Bearer / password auth works
- [ ] 6 unit tests (mocked HTTP) + 1 wire test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean

## Branch + PR
- Branch: `codex-channel-imessage-bluebubbles`
- PR title: `channels: iMessage via Blue Bubbles (codex-channel-imessage-bluebubbles)`
