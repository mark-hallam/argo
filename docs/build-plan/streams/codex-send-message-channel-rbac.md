# codex-send-message-channel-rbac — Per-channel capability + approval gate for send_message

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `25821ea8` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-send-message-channel-rbac`.
**Source**: F-007 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`.

## Scope

P1 RBAC. The `send_message` tool models high-risk outbound channels (`imessage`, `sms`, `slack`, `email`) in `crates/argo-tool/src/tools/send_message.rs:16-30`, but `required_capabilities()` at `:206` returns an empty `CapabilitySet`. The comment at `:207-209` says "Registry already enforces ToolDispatch{send_message}; no extra fine-grained caps." That's coarse — once an agent has `ToolDispatch{send_message}`, it can send to ANY channel and ANY recipient.

Scope:
- Add a new `Capability::MessageSend { channel: String, recipient_scope: String }` variant to `argo-types::Capability` (no equivalent variant currently exists — `rg 'Capability::MessageSend' crates` returns no hits).
- Update `send_message::required_capabilities` to extract `channel` and recipient pattern from the tool args and return `Capability::MessageSend { channel, recipient_scope }`.
- Update the capability engine in `argo-capability/src/lib.rs` to match the new variant.
- Update the default roles (`default_roles()` in `argo-capability/src/lib.rs`) to grant or deny per-channel scopes appropriately — e.g., admin role has full `MessageSend`, user role has scoped `slack`/email only, no sms/imessage by default.
- Route high-risk channels (`sms`, `imessage`, `voice`) through approval policy before dispatch (see `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` for the existing approval-gate pattern).
- Tests: per-channel capability check, approval-gate firing on high-risk channel, deny on missing cap.

LOC: ~220.

## Files you may touch

- `crates/argo-types/src/capability.rs` — add `Capability::MessageSend { channel, recipient_scope }` variant. ≤25 modified lines.
- `crates/argo-capability/src/lib.rs` — add match arm for the new variant; update default roles. ≤80 modified lines.
- `crates/argo-tool/src/tools/send_message.rs` — `required_capabilities` returns the new variant from args. ≤50 modified lines.
- `crates/argo-tool/src/tools/send_message_tests.rs` — per-channel + approval test coverage. ≤60 modified lines.
- `crates/argo-cli/src/handlers/worker_actions_auto_approval.rs` — narrowly: route `MessageSend` for high-risk channels through approval. ≤40 modified lines.

## Files you must NOT touch

- `crates/argo-channels/**` — channel dispatch internals are not the auth boundary; this brief only adds the capability gate before dispatch.
- The `ToolDispatch` capability check itself — keep as-is; the new `MessageSend` is additive, not replacement.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `Capability::MessageSend { channel, recipient_scope }` exists in `argo-types::Capability`.
- [ ] `send_message::required_capabilities` returns the new variant populated from args.
- [ ] Default user role does NOT have `MessageSend { channel: "sms" | "imessage" }` baked in.
- [ ] Default admin role has full `MessageSend`.
- [ ] High-risk channels (`sms`, `imessage`, `voice`) route through approval gate.
- [ ] Tests prove: cap denied → tool errors; high-risk channel → approval triggered; allowed channel → dispatch succeeds.
- [ ] `cargo test -p argo-tool -p argo-capability -p argo-types -p argo-cli` passes.
- [ ] `cargo clippy -p argo-tool -p argo-capability -p argo-types -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-send-message-channel-rbac`
- PR title: `feat(capability): per-channel MessageSend capability + approval gate (codex-send-message-channel-rbac)`

## Notes for the implementer

- `recipient_scope` is a string pattern like `"@company.com"`, `"#general"`, or `"*"` for unrestricted. Match it against args' recipient with a simple glob match (no need for full regex engine).
- The audit's F-007 specifically calls out `imessage`/`sms` as high-risk — those should always go through approval, regardless of capability grant.
- This brief is additive to ToolDispatch; do NOT remove `ToolDispatch{send_message}` from the registry. Both checks fire in series.
