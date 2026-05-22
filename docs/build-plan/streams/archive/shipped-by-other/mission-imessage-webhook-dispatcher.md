# Mission load-bearing wire — `is_imessage_webhook` predicate in gateway dispatcher

**Owner**: claude. **Depends on**: `argo-channels::imessage` exists (verified at `crates/argo-channels/src/imessage.rs`); webhook dispatcher pattern established (verified — `is_email_webhook` at `routes_channels.rs:32`, `is_slack_webhook` at `routes_channels_slack.rs`).
**Authored against**: origin/main `9fe4c612` on `2026-05-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-imessage-webhook-dispatcher`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 item #116 ("Add `is_imessage_webhook` to gateway dispatcher"). The gateway has a webhook-dispatch fan-out at `routes_channels.rs` that routes inbound POSTs to `is_email_webhook` / `is_slack_webhook` handlers based on payload shape. iMessage inbound (Sendblue / Loop / similar) is NOT recognized — the channel ships outbound but inbound webhooks fall through. Mission Claim 6 ("multi-channel approvals") promises a round-trip: operator approval reply via iMessage must reach the orchestrator, not 404 at the dispatcher.

## Scope

Mirror the `slack_webhook` pattern for iMessage:

1. NEW sibling module `crates/argo-cli/src/handlers/gateway/routes_channels_imessage.rs` exporting:
   - `pub(super) fn is_imessage_webhook(req: &str) -> bool` — matches header signature + payload shape from the chosen iMessage provider
   - `pub(super) async fn imessage_webhook_response(...) -> ...` — verifies signature, decodes payload, routes inbound message to the existing approval-reply handler
2. Wire into `routes_channels.rs`: add `|| is_imessage_webhook(req)` to the dispatcher and an `if is_imessage_webhook(req) { return imessage_webhook_response(...).await }` arm matching the `is_slack_webhook` pattern at line 45.

Provider choice: pick whichever iMessage provider is referenced in `argo-channels::imessage` (verify in first 5 minutes via `rg -n 'sendblue|loop_message' crates/argo-channels/src/imessage.rs`). If the channel today supports outbound only and provider isn't pinned, default to **Sendblue** (largest US-OSS-friendly footprint) and document the choice in module rustdoc; switching providers is a follow-up brief.

This is NOT in scope: building a new iMessage provider integration; extending `argo-channels::imessage` itself; supporting multiple providers simultaneously; webhook signing-key rotation UI.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_channels_imessage.rs` (NEW, ≤ 200 LOC) — `is_imessage_webhook` + `imessage_webhook_response` + helper `verify_signature` + tests module sibling.
- `crates/argo-cli/src/handlers/gateway/mod.rs` — narrowly scoped: ONE line `pub(super) mod routes_channels_imessage;` at the same scope as existing `routes_channels_slack` declaration.
- `crates/argo-cli/src/handlers/gateway/routes_channels.rs` — narrowly scoped: ONE `use super::routes_channels_imessage::{is_imessage_webhook, imessage_webhook_response};` import, ONE `|| is_imessage_webhook(req)` arm in the dispatcher predicate, ONE `if is_imessage_webhook(req) { ... }` arm in the dispatch body. NO other edits.
- `crates/argo-cli/src/handlers/gateway/routes_channels_imessage_tests.rs` (NEW, sibling) — 5 unit tests: predicate-recognises-imessage-payload; predicate-rejects-slack-payload; signature-valid-passes; signature-invalid-rejects; payload-decoded-routes-to-approval-reply.
- `crates/argo-cli/tests/imessage_webhook_smoke.rs` (NEW) — integration smoke: POST a valid signed iMessage payload to the gateway, assert the inbound approval-reply was processed.

## Files you must NOT touch

- `crates/argo-channels/src/imessage.rs` — outbound channel implementation; reuse, don't modify
- `crates/argo-cli/src/handlers/gateway/routes_channels_slack.rs` — Slack-specific; orthogonal
- Any other gateway route handler beyond `routes_channels.rs`
- `crates/argo-store/migrations/**` — no schema change

## Acceptance criteria

- [ ] `is_imessage_webhook(req: &str) -> bool` recognises the chosen provider's webhook shape (Sendblue by default; document in module rustdoc)
- [ ] `imessage_webhook_response` verifies the provider's signature header against a configured shared secret (env var `ARGO_IMESSAGE_WEBHOOK_SECRET`); on mismatch returns 401 with empty body
- [ ] On valid signature, decodes payload, extracts `from` (phone) + `text`, routes through the existing approval-reply handler (same path Slack uses; verify in first 5 minutes)
- [ ] `routes_channels.rs` dispatcher arm matches the `is_slack_webhook` pattern exactly (predicate fan-out + dispatch arm)
- [ ] Predicate is FAST (string-prefix or single-header check) — does NOT JSON-parse the body unless the Slack pattern does (it doesn't)
- [ ] 5 new unit tests cover the predicate + signature + dispatch matrix
- [ ] 1 integration smoke exercises end-to-end POST → 200 with valid signed payload
- [ ] When `ARGO_IMESSAGE_WEBHOOK_SECRET` is unset, the dispatch arm returns 503 with a one-line "imessage webhook secret not configured" body (do NOT crash, do NOT 401 — the operator hasn't enabled the channel)
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` passes (skip pre-existing flakes per KNOWN_FLAKES.md)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-imessage-webhook-dispatcher`
- PR title: `gateway: is_imessage_webhook + dispatcher arm for inbound iMessage approval replies (mission-imessage-webhook-dispatcher)`

## Notes for the implementer

- The Slack pattern at `routes_channels_slack.rs` is the template. Match it exactly — same module shape, same `pub(super) fn` visibility, same env-var-for-secret approach.
- For Sendblue specifically, the webhook signature is HMAC-SHA256 over the raw body, header `X-Sendblue-Signature: sha256=...`. Verify against current Sendblue docs at implementation time — don't trust this brief if it's stale.
- The "approval-reply handler" is the existing path Slack uses to find the matching pending `Decision::Ask` and apply Granted/Denied based on text content. Find it via `rg -n 'process_inbound_approval|reply_to_pending_ask' crates/argo-cli/src/handlers/`.
- If the predicate is genuinely ambiguous (an iMessage payload could match `is_slack_webhook`'s shape), prefer header-based discrimination over body-based. Header `X-Sendblue-Signature` only appears on iMessage; that's the cheap discriminator.
- If `argo-channels::imessage` does NOT pin a provider today and the brief's "default to Sendblue" feels speculative, BLOCK with reason "imessage provider not pinned; needs upstream `mission-imessage-channel-provider-pin` brief" rather than guessing.
