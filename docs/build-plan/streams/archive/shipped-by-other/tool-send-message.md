# tool-send-message — Agent-callable `send_message` over argo-channels

**Owner**: claude. **Depends on**: nothing (`argo-channels` already
exists with slack/email/sms/imessage modules). **Tier A — Agent tool
surface.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Hermes parity: agents must be able to talk to the user/customer
mid-task — "I shipped your order, here's the tracking link" or "I
need to confirm the date before booking". This is **distinct** from
the outbound-on-Ask path (operator approval); `send_message` is the
agent itself communicating, capability-gated and risk-tagged per
channel.

## Approach (concrete)

1. Create `crates/argo-tool/src/tools/send_message.rs` exposing
   `send_message(channel: string, recipient: string, body: string,
   attachments?: string[])`. Valid `channel` values: `slack`,
   `email`, `sms`, `imessage` (matching `argo-channels` modules
   actually present — verify with
   `ls crates/argo-channels/src/*.rs`).
2. Implementation: thin wrapper that looks up the channel handler
   in `argo-channels` and calls its existing `send` entry point.
   `attachments` are workspace-relative paths; resolve through
   `path_guard`. Tool returns `{ channel, recipient, message_id,
   sent_at }`.
3. Register in `crates/argo-tool/src/tools/mod.rs`. Add `argo-channels`
   to `argo-tool/Cargo.toml` if not already present.
4. **Capability gating** uses existing
   `Capability::ToolDispatch{tool_name_glob: "send_message"}` for
   the dispatch decision. Per-channel risk tags live in the tool's
   rustdoc + a static `risk_tier(channel) -> &str` helper:
   - `imessage` / `sms` → `high` (real human, hard to recall)
   - `email` → `medium`
   - `slack` → `low` (most teams treat as ephemeral)
   The runtime's existing approval policy can read the risk tier off
   the tool result schema; no policy change in this brief.
5. Add to one vertical manifest (suggest `customer-support`)
   under `[tools].allowed`. Channel-level config (Slack token, SMTP)
   is operator-managed via existing env/config; this brief doesn't
   touch that.
6. Tests:
   - 3+ unit: slack happy path (mock dispatch), email happy path,
     unknown-channel error.
   - 1 integration: end-to-end with a stub channel dispatcher
     proving the message reaches the channel layer with the
     correct shape and risk tier.

## Files you may touch

- `crates/argo-tool/src/tools/send_message.rs` (NEW, ≤ 250 LOC)
- `crates/argo-tool/src/tools/mod.rs` (mod + register in two arrays)
- `crates/argo-tool/src/lib.rs` (re-export `SendMessage`)
- `crates/argo-tool/Cargo.toml` (add `argo-channels` dep if missing)
- One vertical manifest (single-line `allowed` edit)
- `crates/argo-tool/tests/send_message_integration.rs` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- `crates/argo-channels/` source (call existing entry points; do not
  refactor or extend)
- Any other tool file
- `crates/argo-types/src/capability.rs`
- Vertical config beyond a single allowlist line

## Acceptance criteria

- [ ] Capability gating + per-channel risk-tier table documented
- [ ] At least one vertical's manifest allows `send_message`
- [ ] 3+ unit tests
- [ ] 1 integration test
- [ ] Rustdoc with example
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` green
- [ ] Workspace clippy final-sanity clean
- [ ] Public API documented; risk tiers per channel listed

## Branch + PR

- Branch: `tool-send-message`
- PR title: `tool-send-message — agent-callable send_message over argo-channels`
