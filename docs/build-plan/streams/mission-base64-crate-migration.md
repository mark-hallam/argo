# Mission load-bearing wire — Replace hand-rolled base64 encoders with the `base64` crate

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `01dc04cd` on `2026-05-13`.

> SOURCE: MISSION_BACKLOG #259. Argo has at least **four** hand-rolled `base64_encode` / `base64_decode` functions spread across `argo-cli`, `argo-executor-computer-use`, and `argo-tool`. The `base64` crate (already in the dep graph via `argo-mcp-connector-image-gen`) is well-maintained, constant-time, and significantly faster. Replace the hand-rolls. **Tier-Refactor (zero behaviour change).**

## Scope

Pure refactor — for each hand-rolled encoder/decoder, replace the body with a thin wrapper over `base64::engine::general_purpose::STANDARD.encode()` / `.decode()`:

1. `crates/argo-executor-computer-use/src/anthropic.rs` (≈ line 240) — "Minimal base64 encoder. We don't pull in the `base64` crate." → delete the hand-roll, depend on `base64` (it's already in workspace via image-gen, just need to add to this crate's Cargo.toml).
2. `crates/argo-cli/src/handlers/gateway/routes_voice_auth.rs` (≈ line 193) — `fn base64_encode(data: &[u8]) -> String`. Replace with `base64::STANDARD.encode(data)`.
3. `crates/argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs` (≈ line 231) — same shape; same replacement.
4. `crates/argo-cli/src/handlers/trajectory_export_render.rs` (≈ line 188) — `pub(super) fn base64_encode(input: &[u8]) -> String`. Replace.
5. `crates/argo-tool/src/tools/vision.rs` (≈ line 369) — `fn base64_decode(input: &str) -> Result<Vec<u8>, String>`. Replace with `base64::STANDARD.decode(input).map_err(|e| e.to_string())`.

For each call site:
- Delete the helper function and its inline tests (if any).
- Update the import.
- If the local helper allowed embedded newlines or used `URL_SAFE`, decide the right `base64` engine variant at the call site (default to `STANDARD`; document the variant choice in a code comment).

NOT in scope: hashing / signing helpers (HMAC, SHA*); functions that *use* base64 as a sub-step but expose a richer API (e.g. `signed_decision_url`); changing the wire-format of any existing audit / token / signed-URL (the byte-for-byte output must remain identical, which is exactly what `STANDARD` engine gives you).

## Files you may touch

- `crates/argo-executor-computer-use/Cargo.toml` — add `base64 = "0.22"` (workspace-pinned to match image-gen's version)
- `crates/argo-executor-computer-use/src/anthropic.rs` — delete hand-roll, update use
- `crates/argo-cli/Cargo.toml` — verify `base64` is a direct dep; add if missing
- `crates/argo-cli/src/handlers/gateway/routes_voice_auth.rs` — delete hand-roll
- `crates/argo-cli/src/handlers/gateway/routes_channels_whatsapp.rs` — delete hand-roll
- `crates/argo-cli/src/handlers/trajectory_export_render.rs` — delete hand-roll
- `crates/argo-tool/Cargo.toml` — verify; add if missing
- `crates/argo-tool/src/tools/vision.rs` — delete hand-roll

## Files you must NOT touch

- Any signed-URL / HMAC helper (out of scope; those are byte-correct against external systems and need separate review)
- The wire format of any audit row, voice-auth token, WhatsApp message body — verify with round-trip test before/after
- The `base64` version pin (workspace-level — touching it would imply a cross-cutting dep bump)

## Acceptance criteria

- [ ] All 5 hand-rolled encoder/decoder functions deleted from source
- [ ] Each call site now calls `base64::engine::general_purpose::STANDARD.encode()` / `.decode()`
- [ ] `cargo build --workspace` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] All existing tests pass unchanged (this is a pure-rename refactor)
- [ ] Diff-size: <100 LOC NET (deletions outweigh additions); the 5 hand-rolls together are >150 LOC
- [ ] BUILD_STATUS.md row updated

## Branch + PR

- Branch: `claude-mission-base64-crate-migration`
- PR title: `refactor: replace 5 hand-rolled base64 encoders with the base64 crate (mission-base64-crate-migration)`

## Notes

- The `base64` crate's `general_purpose::STANDARD` is byte-for-byte compatible with the canonical RFC 4648 STANDARD alphabet (the one all the hand-rolls implement). The output is identical.
- A pre-existing test in `argo-tool` (`base64_decode_round_trip_smoke`) verifies the decode path — keep it; it now exercises the new code path.
- If the worker finds a hand-roll that's subtly *not* RFC-4648-STANDARD (e.g. uses `-` and `_` instead of `+` and `/`), use `URL_SAFE` instead and document. Round-trip-test before merge.
- The PR diff should be predominantly deletions. If you find yourself adding meaningful logic, you've scope-crept.
