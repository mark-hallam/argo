# Mission load-bearing wire — consolidate voice route under channels dispatcher

**Owner**: claude. **Depends on**: `routes_voice.rs` exists (verified at `crates/argo-cli/src/handlers/gateway/routes_voice.rs`); `routes_channels.rs` dispatcher pattern stable.
**Authored against**: origin/main `6da4af77` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-voice-route-consistency`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 item #117 ("Move voice route into channels dispatcher consistency"). Today voice has its own dedicated routes (`routes_voice.rs` + `routes_voice_auth.rs`) while every other channel routes through the unified `routes_channels.rs` predicate-fanout pattern (`is_email_webhook` / `is_slack_webhook`). The asymmetry confuses operators looking for "where do channel webhooks live" — it's mostly under `routes_channels.rs`, except voice is special. Mission Claim 6 ("multi-channel approvals") promises a uniform surface; the dispatcher should look uniform.

## Scope

Refactor the voice path to match the established channels dispatcher pattern:

1. NEW `crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs` exporting `pub(super) fn is_voice_webhook(req: &str) -> bool` + `pub(super) async fn voice_webhook_response(...)` — same shape as the Slack module.
2. Migrate the voice-recognition + dispatch code from `routes_voice.rs` into the new module, keeping authentication-specific logic (e.g. Twilio signature verification) intact.
3. Wire into `routes_channels.rs`: add `|| is_voice_webhook(req)` arm + `if is_voice_webhook(req) { ... }` dispatch branch.
4. Reduce `routes_voice.rs` to a thin re-export shim if any external module imports from it (verify in first 5 minutes); preferably delete the file once nothing imports from it.
5. `routes_voice_auth.rs` stays put — it's the auth-specific helper and keeping it separate is fine; just have the new `routes_channels_voice.rs` import from it.

This is NOT in scope: rewriting Twilio integration; changing voice transcription logic; touching `argo-channels::voice` (the channel implementation); modifying `boot.rs` voice initialization beyond what's needed to register the new dispatcher arm.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_voice.rs` (NEW, ≤ 200 LOC) — `is_voice_webhook` predicate + `voice_webhook_response` handler. Imports auth helpers from `routes_voice_auth.rs`.
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs` — narrowly scoped: ONE import line + ONE predicate arm + ONE dispatch arm matching the Slack pattern.
- `crates/argo-cli/src/handlers/gateway/routes_voice.rs` — REMOVE the bulk of dispatch logic; either delete file entirely (preferred) or reduce to ≤ 30 LOC re-export shim. NO change to behavior.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: REMOVE `routes_voice` module declaration if file deleted; ADD `routes_channels_voice` module declaration. NO other edits.
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: IF voice has a registration call in boot, update it to point at the new module. Otherwise no change.
- `crates/argo-cli/src/handlers/gateway/routes_channels_voice_tests.rs` (NEW, sibling) — 5 unit tests: predicate-recognises-twilio-payload; predicate-rejects-slack-payload; signature-valid-passes; signature-invalid-rejects; payload-routes-to-existing-voice-handler.
- `crates/argo-cli/tests/voice_webhook_consistency_smoke.rs` (NEW) — integration smoke: assert pre-refactor and post-refactor behavior are byte-identical for a recorded valid Twilio payload (use a stub auth verifier).

## Files you must NOT touch

- `crates/argo-channels/src/voice.rs` / `voice_format.rs` — channel impl is upstream
- `crates/argo-cli/src/handlers/gateway/routes_voice_auth.rs` — Twilio auth helper stays put; only the dispatcher relocates
- `crates/argo-store/migrations/**` — no schema change
- Any non-gateway handler

## Acceptance criteria

- [ ] `routes_channels_voice.rs` exists with `is_voice_webhook` + `voice_webhook_response` matching Slack module shape
- [ ] `routes_channels.rs` dispatcher recognises voice via the predicate fanout
- [ ] `routes_voice.rs` is either deleted or reduced to ≤ 30 LOC re-export shim
- [ ] No public-API change observable from outside `argo-cli` (any external consumer that imported from `routes_voice` continues to compile via re-export OR the file's removal is documented as a deliberate breaking change)
- [ ] `routes_voice_auth.rs` is unchanged (auth surface stable)
- [ ] Pre/post-refactor behavior is byte-identical for a recorded valid Twilio payload (integration smoke asserts this)
- [ ] 5 unit tests cover the predicate + dispatch matrix
- [ ] 1 integration smoke verifies behavior continuity
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-voice-route-consistency`
- PR title: `gateway: consolidate voice route under channels dispatcher pattern (mission-voice-route-consistency)`

## Notes for the implementer

- The `is_voice_webhook` predicate likely keys on the Twilio header `X-Twilio-Signature` — that's the cheap discriminator (same shape as iMessage's `X-Sendblue-Signature`). Verify against current Twilio docs; the predicate should be HEADER-based, not body-based.
- This is a refactor PR, not a feature PR. The integration smoke is the load-bearing acceptance — it ensures the move doesn't subtly change response codes or body shape.
- If `routes_voice.rs` exports anything that's imported externally (e.g. by an integration test elsewhere in `argo-cli/tests/`), keep it as a thin re-export shim rather than deleting. The 30 LOC budget is the cap.
- Don't merge `routes_voice_auth.rs` into the new module — keeping auth helpers as a sibling matches how Slack does it.
- The migration is mechanical. If you find yourself rewriting voice dispatch logic mid-refactor, you've drifted scope.
