# Mission load-bearing wire — MCP connector for WhatsApp Business Cloud API

**Owner**: claude. **Depends on**: SHIPPED `crates/argo-mcp-connector-signal/`, `crates/argo-mcp-connector-telegram/`, `crates/argo-mcp-connector-replicate/`, `crates/argo-mcp-connector-zoom/` as canonical 4-file connector patterns; `crates/argo-mcp-connector-shared/` for `McpClient` + JSON-RPC scaffolding.
**Authored against**: origin/main `c26e63e1` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-whatsapp-cloud`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #106 ("Add 5+ MCP connectors (telegram, discord, whatsapp-cloud, signal, matrix)" — telegram/signal/discord SHIPPED; matrix SHIPPED via #1363; whatsapp-cloud is the remaining unshipped item). **Tier A (tool).** WhatsApp Business Cloud API for send-message + template-message + media-message + delivery-status webhook. Outbound-only in v1; inbound webhook delivery routing is a v2 (gateway-route brief).

## Scope

NEW `crates/argo-mcp-connector-whatsapp-cloud/` crate (≤ 850 LOC across 4 files):

- `crates/argo-mcp-connector-whatsapp-cloud/Cargo.toml` (NEW, ≤ 25 LOC) — `reqwest`, `serde`, `serde_json`, `tokio`, `thiserror`, `tracing`, depend on `argo-mcp-connector-shared`. Dev-dep `wiremock`.
- `crates/argo-mcp-connector-whatsapp-cloud/src/lib.rs` (NEW, ≤ 60 LOC) — re-exports + JSON-RPC method declarations.
- `crates/argo-mcp-connector-whatsapp-cloud/src/client.rs` (NEW, ≤ 380 LOC) — `WhatsAppCloudClient` against `https://graph.facebook.com/v18.0/<phone_number_id>/messages`:
  - `send_text(to_e164, body) -> WhatsAppMessageId` — text messages
  - `send_template(to_e164, template_name, language_code, variables) -> WhatsAppMessageId` — pre-approved templates (24h outside-window requirement)
  - `send_media(to_e164, media_kind, media_url_or_id, caption) -> WhatsAppMessageId` — media (image/document/audio/video/sticker)
  - `mark_read(message_id) -> ()` — outgoing-read receipts
  - Bearer-token auth via env-indirect `ARGO_WHATSAPP_CLOUD_ACCESS_TOKEN`; long-lived (60-day) system-user token; refresh is operator-side (not auto)
  - Rate-limit handling: respect `X-Business-Use-Case-Usage` header; back-off w/ jitter on 429
  - Per-action audit-log integration (existing pattern from sibling connectors)
- `crates/argo-mcp-connector-whatsapp-cloud/src/types.rs` (NEW, ≤ 180 LOC) — `WhatsAppMessageId`, `WhatsAppMediaKind` enum (Image / Document / Audio / Video / Sticker), `WhatsAppTemplateVariable`, `WhatsAppRateLimitInfo`, `WhatsAppCloudError` enum (`AuthFailed`, `RateLimited { retry_after_secs }`, `TemplateNotApproved`, `RecipientNotOptedIn`, `OutsideMessagingWindow`, `MediaTooLarge`, etc.).
- `crates/argo-mcp-connector-whatsapp-cloud/src/client_tests.rs` (NEW, ≤ 205 LOC, included via `#[path]`) — 8 unit tests using `wiremock`: send-text-happy, send-template-w-3-variables, send-media-image-by-url, send-media-document-w-caption, 429-back-off-w-retry-after-honored, 401-bubbles-as-AuthFailed, template-not-approved-error, outside-24h-window-error.

NEW `crates/argo-mcp-connector-whatsapp-cloud/tests/whatsapp_smoke.rs` (NEW, ≤ 95 LOC) — env-gated smoke (`ARGO_TEST_WHATSAPP_CLOUD_ACCESS_TOKEN` + `ARGO_TEST_WHATSAPP_CLOUD_PHONE_NUMBER_ID` + `ARGO_TEST_WHATSAPP_CLOUD_RECIPIENT_E164`). Sends one text + one template; verifies message-id returned + cleanup not needed (WhatsApp doesn't support outbound-message-delete).

EXTEND `Cargo.toml` (workspace) — add `argo-mcp-connector-whatsapp-cloud` to `members`.

NO migration. NO Rust changes in core crates. Connector is fully self-contained (matches sibling-connector pattern).

This is NOT in scope: inbound webhook routing (separate gateway-route brief — operator subscribes to WhatsApp Cloud's incoming-message webhook, gateway dispatches to argo-channels); media upload (the connector accepts media-by-URL or pre-uploaded-media-id; uploading raw bytes to WhatsApp's media endpoint is a v2 — large + complex); interactive button / list messages (v2 — operator can request); WhatsApp Business Platform on-premises API (deprecated by Meta — Cloud API only); product / catalog messages (out-of-scope; vertical-specific); template creation/management (operator manages templates in Meta Business Manager UI; the connector uses pre-approved template names only).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-whatsapp-cloud/Cargo.toml` (NEW, ≤ 25 LOC)
- `crates/argo-mcp-connector-whatsapp-cloud/src/lib.rs` (NEW, ≤ 60 LOC)
- `crates/argo-mcp-connector-whatsapp-cloud/src/client.rs` (NEW, ≤ 380 LOC)
- `crates/argo-mcp-connector-whatsapp-cloud/src/types.rs` (NEW, ≤ 180 LOC)
- `crates/argo-mcp-connector-whatsapp-cloud/src/client_tests.rs` (NEW, ≤ 205 LOC)
- `crates/argo-mcp-connector-whatsapp-cloud/tests/whatsapp_smoke.rs` (NEW, ≤ 95 LOC)
- `Cargo.toml` (workspace EXTEND — add to `members`)

## Files you must NOT touch

- Other connector crates (sibling pattern stays)
- `crates/argo-store/migrations/**` (no migration)
- `crates/argo-channels/**` (outbound only in v1)

## Acceptance criteria

- [ ] `cargo build -p argo-mcp-connector-whatsapp-cloud` succeeds
- [ ] `cargo test -p argo-mcp-connector-whatsapp-cloud` passes all 8 unit tests
- [ ] 429 back-off honors `Retry-After` header w/ jitter
- [ ] 401 → typed `AuthFailed` (never bubbles raw status)
- [ ] `OutsideMessagingWindow` distinguishes session-msg (24h-since-last-customer-msg) from outside-window template requirement
- [ ] Smoke test (env-gated) sends one text + one template to test recipient + returns message-ids
- [ ] All file sizes within budget; per-file ≤ briefed; total ≤ 850 LOC
- [ ] No migration; no Rust changes outside the new crate + workspace `members`
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-whatsapp-cloud`
- PR title: `mcp-connector: WhatsApp Business Cloud API (text / template / media; outbound v1) (mission-mcp-connector-whatsapp-cloud)`

## Notes for the implementer

- Treat just-shipped `crates/argo-mcp-connector-signal/` (#1350) or `crates/argo-mcp-connector-replicate/` (#1359) as 1:1 templates for crate layout + JSON-RPC + auth + rate-limit + test shape.
- WhatsApp Cloud API endpoint: `https://graph.facebook.com/v18.0/<phone_number_id>/messages` (POST). Auth: `Authorization: Bearer <access_token>`. Body shape per Meta's docs (the `messaging_product=whatsapp`, `recipient_type=individual`, `to=E164`, `type=text|template|image|...`).
- Template messages require pre-approval in Meta Business Manager. The connector accepts a `template_name` + `language_code` + `variables`; if the template isn't approved, Meta returns a clear error → typed `TemplateNotApproved`.
- 24-hour messaging window: outside a 24h window since the customer's last message, only TEMPLATE messages are deliverable (not free-form text). Meta returns an error → typed `OutsideMessagingWindow`. Document this prominently in `lib.rs` rustdoc.
- Media-by-URL is the v1 path: pass an HTTPS URL to Meta; they fetch + send. Document the URL must be publicly reachable (Meta's edge fetches it). Pre-uploaded-media-id is also supported (operator uploads via Meta's media endpoint out-of-band).
- Rate-limit handling: WhatsApp returns `X-Business-Use-Case-Usage` header w/ JSON describing current load. On 429, parse the header for `estimated_time_to_regain_access` or fall back to `Retry-After`. Sleep w/ jitter `[0, 1s]` to avoid thundering-herd retries.
- DO NOT cache the access-token in the connector — it's long-lived (60d), operator-managed. The connector reads it fresh from env on each request (cheap; env-var read is O(1)).
- DO NOT support the deprecated WhatsApp Business API (on-premises). Cloud API only.
- DO NOT auto-upload media. v1 accepts URL-or-media-id only. Media-blob upload via `multipart/form-data` is a v2 if operators ask.
- DO NOT implement webhook receipt verification in this brief — that's the gateway-route inbound brief.
- The test recipient for the smoke (`ARGO_TEST_WHATSAPP_CLOUD_RECIPIENT_E164`) should be the operator's own test number — WhatsApp Cloud has aggressive abuse-detection on cross-account messaging. Document this in the smoke's header comment.
- E164 format strictly: `+<country><number>` with no spaces or dashes. Connector should reject non-E164 input with a typed error before hitting the API.
