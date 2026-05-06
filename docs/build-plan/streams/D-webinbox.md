# Stream D — Web inbox approval channel

**Owner**: codex. **Depends on**: D-email merged.

## Scope

`WebInboxChannel` impl. Operator opens a web URL
(`<gateway>/inbox`) and sees a list of pending approvals;
clicks approve or reject. Real-time updates via WebSocket from
the gateway.

The full polished SPA dashboard is v1.x. This brief delivers
the MINIMUM web surface: a pure-HTML `/inbox` route that lists
pending approvals as forms and POSTs back to the same gateway.
Good enough for operators-not-at-a-terminal in v1; will be
replaced by the proper SPA later.

## Files you may touch
- `crates/argo-channels/src/lib.rs` — narrowly scoped: add ONE
  line `pub mod webinbox;` (or `mod webinbox;`). No other edits.
- `crates/argo-channels/src/webinbox.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs`
  (extend: `GET /inbox`, `POST /inbox/approve/<id>`,
  `POST /inbox/reject/<id>`, `GET /inbox/ws` WebSocket)
- `manifests/channels/webinbox.toml`

## Files you must NOT touch
- Other channel impls
- `NotifyChannel` trait

## Acceptance criteria
- [ ] `WebInboxChannel` impl of `NotifyChannel`
- [ ] `send()` enqueues into a per-operator inbox table; does
      NOT push externally (web pull-only)
- [ ] `GET /inbox` renders a minimal HTML page (server-side
      `format!` template; no React for v1) listing every
      pending approval for the requesting operator
- [ ] WebSocket at `/inbox/ws` pushes new approval events as
      JSON; the HTML page polls every 5s as a fallback
- [ ] Operator auth via bearer token in cookie or header
- [ ] Integration test: enqueue a notification, GET /inbox,
      assert the row appears; POST approve, assert settle
- [ ] Clippy clean
- [ ] Every file < 500 LOC

## Branch + PR
- Branch: `codex-D-webinbox`
- PR title: `channels: minimal web inbox + WebSocket push for v1 (D-webinbox)`

## Notes
- The HTML can be ugly. v1.x replaces this with a proper SPA
  (Svelte + Tailwind). Don't gold-plate.
- Keep the route stable so the SPA can swap in cleanly.
