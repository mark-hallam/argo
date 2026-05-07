# V1.x — Architectural prep: split routes_channels.rs

**Owner**: claude. **Depends on**: nothing (pure refactor).

## Scope

`crates/argo-cli/src/handlers/gateway/routes_channels.rs` is at
exactly 499 LOC, plus the parent `gateway/mod.rs` is also at 499.
Both are at the 500-LOC project invariant ceiling. This blocks
new approval channel decision routes (whatsapp #44b11e7 already
hit this ceiling, future channels like discord-approvals will
hit it too).

Pure refactor. Mirror the proven pattern from
`claude-worker-actions-split` (#851) and
`claude-cli-subcommands-split` (#882): split the per-channel
decision routes into focused sibling files, keep public API
stable via `pub use` re-exports.

## Approach

`routes_channels.rs` likely has separate handler functions for
each channel decision callback (slack/sms/push/voice/imessage).
Cleanest split:

- Keep router registration + shared types in
  `routes_channels.rs` (target ≤200 LOC)
- Move per-channel handlers to sibling files:
  - `routes_channels_slack.rs` (slack decision)
  - `routes_channels_sms.rs` (sms decision)
  - `routes_channels_push.rs` (push decision)
  - `routes_channels_voice.rs` (voice DTMF)
  - `routes_channels_imessage.rs` (iMessage decision)
- Each sibling ≤150 LOC; headroom for adding a new channel
  arm without touching the central router

For `gateway/mod.rs`: similar — it likely has a big match arm
dispatching to per-route handlers. Extract sub-mods if
beneficial; if it's just declarations, mainly trim docstrings.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs` —
  reduce to ≤200 LOC; keep public re-exports
- `crates/argo-cli/src/handlers/gateway/routes_channels_*.rs`
  (NEW, 5 files)
- `crates/argo-cli/src/handlers/gateway/mod.rs` — trim if
  possible; if not splittable, that's OK as long as
  routes_channels.rs is under 500
- Any handler that imports from
  `gateway::routes_channels::` — narrowly: update import
  paths if a symbol moved (mechanical; should be zero edits
  if re-exports are right)

## Acceptance criteria
- [ ] `routes_channels.rs` ≤ 200 LOC
- [ ] Each new sibling file ≤ 150 LOC
- [ ] No public-API change (every existing call site still
      compiles unchanged)
- [ ] `cargo build --workspace` clean
- [ ] `cargo test --workspace` — same set of tests pass as
      before (no test deletions; no skips)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Headroom for next 5+ channel additions before the
      central router approaches the 500 ceiling

## Branch + PR
- Branch: `claude-routes-channels-split`
- PR title: `refactor: split routes_channels.rs under 500 LOC budget (claude-routes-channels-split)`
