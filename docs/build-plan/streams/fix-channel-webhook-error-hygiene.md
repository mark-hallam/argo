# fix-channel-webhook-error-hygiene

**Owner**: claude.
**Authored against**: origin/main `f1ea8b09a` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-channel-webhook-error-hygiene`.

## Why (e2e finding R3-F3 / R1-F1, VERIFIED live 2026-06-09)
Channel approval webhooks return **HTTP 500 and leak the name of the unset server secret** to UNAUTHENTICATED callers when a channel secret is unconfigured — wrong status class (server-fault for a config-absence) + minor information disclosure. Verified live (demo gateway has no channel secrets set):
- `POST /channels/push/decision/<tok>` → 500 `{"error":"missing ARGO_CHANNEL_HMAC_SECRET"}`
- `POST /channels/email/webhook` (Bounce) → 500 `{"error":"missing ARGO_CHANNEL_HMAC_SECRET"}`
- `POST /twilio/{sms,whatsapp,voice}/decision` → 500 `missing TWILIO_AUTH_TOKEN`
- `POST /channels/slack/webhook` → 500 `missing SLACK_APPROVALS_SIGNING_SECRET`
- `POST /channels/imessage/webhook` → 503 `imessage webhook secret not configured`
- **Discord does it right**: `POST /channels/discord/decision` → 401 `missing Discord signature headers` (pre-checks signature presence BEFORE the secret lookup).
- Also: `POST /api/v1/approvals/bundles/<non-uuid>/decide` → 500 leaking `bundle id parse: invalid character ...`.

## Grep-before-write
- `rg -n 'fn hmac_secret' crates/argo-cli/src/handlers/gateway/routes_channels.rs` — the shared secret getter; its `Err("missing ARGO_CHANNEL_HMAC_SECRET")` is echoed by callers.
- `rg -n 'hmac_secret\(\)|TWILIO_AUTH_TOKEN|SLACK_APPROVALS_SIGNING_SECRET|server_error' crates/argo-cli/src/handlers/gateway/routes_channels*.rs crates/argo-cli/src/handlers/gateway/sso_*.rs` — the ~10 callers (sms, whatsapp, voice, slack, push, imessage, webinbox, email, sso_saml, sso_oidc).
- `crates/argo-cli/src/handlers/gateway/routes_channels_discord.rs` — the CORRECT pattern (pre-check signature headers → 401 before secret lookup); mirror it.
- `routes_approval_bundles.rs:155` `decide_response` / `get_bundle` — non-UUID id → server_error; a well-formed-unknown UUID already 404s.

## Fix
1. A missing server secret reachable by an unauthenticated caller should NOT be a 500 that names the var. Make `hmac_secret()` (and the Twilio/Slack equivalents) map the unset/empty case to a **503** "channel not configured" with a GENERIC message (no env-var name). Better still, mirror Discord: pre-check the request carries the expected signature header(s) and return **401** before the secret lookup, so an unauthenticated probe can't even reach the config-state.
2. `decide_response` (bundle): a malformed (non-UUID) id → **400/404** with a generic message, not a 500 echoing the parse error. (A well-formed-unknown UUID already 404s — only the malformed-shape leaks.)
3. Do NOT weaken any signature verification or change business logic — status-mapping + message hygiene only.

## Acceptance (truth-bounded)
1. Tests: each channel webhook, with the secret UNSET + an unauthenticated body, returns 401/503 (NOT 500) and the body does NOT contain the env-var name. Non-UUID bundle id → 400/404, no parse-string leak.
2. Live re-probe (demo gateway, no secrets set): the curls in "Why" return 401/503 with generic messages.
3. `cargo test -p argo-cli` green; clippy clean on touched files (boundary-note pre-existing routes_settings_credentials).

## On ship (same commit)
Note it closes e2e finding R3-F3/R1-F1.

## Effort
M (cross-cutting ~10 handlers, mechanical: pre-check + status/message hygiene + tests).
