# `argo worker channels` — runtime channel management

**Owner**: claude. **Depends on**: —

## Why

`Worker.channels: Vec<NotifyChannelRef>` is set at draft time and then
mostly frozen. Operators want to add an email channel mid-life, swap a
Slack handle, drop a stale WhatsApp number — without re-creating the
worker. Three subcommands, no schema change.

## Scope

- `argo worker channels add <worker-id> <channel-spec>` — appends.
- `argo worker channels remove <worker-id> <channel-id>` — removes
  by id.
- `argo worker channels list <worker-id>` — JSON or markdown.
- `<channel-spec>` parses as `kind:identifier` (e.g. `email:foo@bar.com`,
  `slack:U0123`, `whatsapp:+15551234567`). Validation per channel kind.
- Idempotent add: adding the same channel twice → second is a no-op
  (with a warning printed).
- Cap: ≤ 16 channels per worker (config constant).

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/worker_channels.rs` (~280 LOC):
   - `parse_channel_spec(s: &str) -> ArgoResult<NotifyChannelRef>`
   - `pub async fn add(store, id, spec) -> ArgoResult<()>` — read worker,
     check cap + dedupe, push, write back via existing repo.
   - `pub async fn remove(store, id, channel_id) -> ArgoResult<()>` —
     read worker, retain by id != target, write back. Error if no
     match found.
   - `pub async fn list(store, id, json: bool) -> ArgoResult<String>` —
     render.
2. **NEW** `crates/argo-cli/src/handlers/worker_channels_tests.rs` (~250 LOC):
   - parse: each kind happy path, malformed (no colon, unknown kind, bad email)
   - add: happy, idempotent dup, cap-exceeded
   - remove: happy, missing-id error
   - list: empty, populated, JSON shape
3. **Modify** `crates/argo-cli/src/cli.rs` — 1 sub-enum
   `WorkerChannels { Add { id, spec }, Remove { id, channel_id }, List { id, json } }`.
   EXHAUSTIVE-MATCH note: handlers/mod.rs dispatch.
4. **NEW** `crates/argo-cli/tests/worker_channels_smoke.rs` (~120 LOC) —
   round-trip add → list → remove → list.

## Files you may touch

- `crates/argo-cli/src/handlers/worker_channels.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/worker_channels_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/mod.rs` (1 pub mod + dispatch arm)
- `crates/argo-cli/src/cli.rs` (BLANKET CLI ALLOWLIST — 1 sub-enum)
- `crates/argo-cli/tests/worker_channels_smoke.rs` (NEW)

## Files you must NOT touch

- `crates/argo-identity/` — `NotifyChannelRef` shape unchanged
- The channel implementations themselves (email, slack, etc.) — this
  brief is metadata-only; we don't *send* anything
- `argo-store/` — re-use the existing worker repo's read+write
- Any new schema migration — channels live inside Worker JSON already

## Acceptance criteria

- [ ] `argo worker channels add <id> email:a@b.com` updates the worker
- [ ] `argo worker channels list <id> --json` emits the channel array
- [ ] `argo worker channels remove <id> <channel-id>` removes
- [ ] Adding the same channel twice → second is a no-op with stderr warning
- [ ] 17th channel → rejected before write
- [ ] Removing non-existent channel-id → exit 1 with clear message
- [ ] Bad spec (`foo`, `unknown:bar`, `email:not-an-email`) → exit 1
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-worker-channels-cli`
- PR title: `worker: runtime channel add/remove/list (claude-worker-channels-cli)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies.
- Channel-id assignment: re-use whatever id-assignment logic exists at
  worker draft time. Don't invent a new one. Search for "channel_id =".
- Email validation: lean on a regex that matches what the email
  channel send path validates — DO NOT introduce a new email-validation
  crate just for this.
- Phone-number validation for WhatsApp: E.164 format only;
  reject otherwise.
