# Mission load-bearing wire — MCP connector for Resend (transactional email)

**Owner**: claude. **Depends on**: SHIPPED `crates/argo-mcp-connector-sendgrid/`, SHIPPED `crates/argo-mcp-connector-mailgun/` as canonical 4-file transactional-email connector patterns; `crates/argo-mcp-connector-shared/` for `McpClient` + JSON-RPC scaffolding.
**Authored against**: origin/main `84e47150` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-mcp-connector-resend`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 5 #112 ("Add MCP connector SendGrid/Mailgun/Resend" — SendGrid SHIPPED; Mailgun SHIPPED; Resend is the remaining unshipped item). **Tier A (tool).** Resend transactional-email REST API for send + get + domain-listing. Outbound-only in v1; inbound webhook delivery routing (open / click / bounce / complaint) is a v2 gateway-route brief.

## Scope

NEW `crates/argo-mcp-connector-resend/` crate (≤ 700 LOC across 4 files):

- `crates/argo-mcp-connector-resend/Cargo.toml` (NEW, ≤ 25 LOC) — `reqwest`, `serde`, `serde_json`, `tokio`, `thiserror`, `tracing`, depend on `argo-mcp-connector-shared`. Dev-dep `wiremock`.
- `crates/argo-mcp-connector-resend/src/lib.rs` (NEW, ≤ 60 LOC) — re-exports + JSON-RPC method declarations.
- `crates/argo-mcp-connector-resend/src/client.rs` (NEW, ≤ 320 LOC) — `ResendClient` against `https://api.resend.com/`:
  - `send_email(from, to, subject, html?, text?, reply_to?, cc?, bcc?, tags?) -> ResendMessageId` — endpoint `POST /emails`
  - `get_email(message_id) -> ResendEmailStatus` — endpoint `GET /emails/<id>` (returns `last_event: sent | delivered | bounced | complained | opened | clicked`)
  - `list_domains() -> Vec<ResendDomain>` — endpoint `GET /domains` (verification status of operator's registered senders)
  - `send_batch(emails: Vec<ResendEmail>) -> Vec<ResendMessageId>` — endpoint `POST /emails/batch` (up to 100 per call)
  - Auth: Bearer API key via env-indirect `ARGO_RESEND_API_KEY`; key starts with `re_` prefix.
  - Rate-limit handling: Resend returns `429` w/ `Retry-After`; back-off w/ jitter `[0, 1s]`. Resend's default limit is 10 req/s; document this.
  - Per-action audit-log integration (existing pattern from sibling connectors).
- `crates/argo-mcp-connector-resend/src/types.rs` (NEW, ≤ 170 LOC) — `ResendMessageId(String)`, `ResendEmail { from, to, subject, html?, text?, reply_to?, cc?, bcc?, tags? }`, `ResendEmailStatus { id, from, to, subject, last_event, created_at }`, `ResendDomain { id, name, status, region, created_at, records: Vec<DnsRecord> }`, `ResendError` enum (`AuthFailed`, `RateLimited { retry_after_secs }`, `DomainNotVerified`, `InvalidEmail`, `RecipientBlocked`, `BatchTooLarge`, etc.).
- `crates/argo-mcp-connector-resend/src/client_tests.rs` (NEW, ≤ 180 LOC, included via `#[path]`) — 8 unit tests using `wiremock`: send-email-happy-html, send-email-text-only, send-batch-3-emails, get-email-status-delivered, list-domains-2-verified-1-pending, 429-back-off-honors-retry-after, 401-bubbles-as-AuthFailed, domain-not-verified-typed-error.

NEW `crates/argo-mcp-connector-resend/tests/resend_smoke.rs` (NEW, ≤ 95 LOC) — env-gated smoke (`ARGO_TEST_RESEND_API_KEY` + `ARGO_TEST_RESEND_FROM_EMAIL` + `ARGO_TEST_RESEND_TO_EMAIL`). Sends one HTML email; polls `get_email` until `last_event == "delivered"` or 10s timeout; verifies typed fields populated.

EXTEND `Cargo.toml` (workspace) — add `argo-mcp-connector-resend` to `members`.

NO migration. NO Rust changes in core crates. Connector is fully self-contained.

This is NOT in scope: inbound webhook routing (separate gateway-route brief — operator subscribes to Resend's webhook, gateway dispatches to argo-channels); broadcast / marketing list management (Resend's `/audiences` + `/broadcasts` endpoints — out-of-scope for v1 transactional shape; v2 if operators ask); template management via Resend's `/templates` (operator manages templates in Resend dashboard); contact opt-out / suppression list management (operator manages in Resend dashboard); email-validation API (`/email-validation`) — operator can call separately, not part of this connector.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-connector-resend/Cargo.toml` (NEW, ≤ 25 LOC)
- `crates/argo-mcp-connector-resend/src/lib.rs` (NEW, ≤ 60 LOC)
- `crates/argo-mcp-connector-resend/src/client.rs` (NEW, ≤ 320 LOC)
- `crates/argo-mcp-connector-resend/src/types.rs` (NEW, ≤ 170 LOC)
- `crates/argo-mcp-connector-resend/src/client_tests.rs` (NEW, ≤ 180 LOC)
- `crates/argo-mcp-connector-resend/tests/resend_smoke.rs` (NEW, ≤ 95 LOC)
- `Cargo.toml` (workspace EXTEND — add to `members`)

## Files you must NOT touch

- Other connector crates (sibling pattern stays)
- `crates/argo-store/migrations/**` (no migration)
- `crates/argo-channels/**` (Resend is transactional email, not a channel)

## Acceptance criteria

- [ ] `cargo build -p argo-mcp-connector-resend` succeeds
- [ ] `cargo test -p argo-mcp-connector-resend` passes all 8 unit tests
- [ ] 429 back-off honors `Retry-After` header w/ jitter
- [ ] 401 → typed `AuthFailed` (never bubbles raw status)
- [ ] `DomainNotVerified` distinguishes from generic 4xx (operator sends from a domain not yet verified in Resend dashboard)
- [ ] `send_batch` enforces `≤ 100` recipients per call → typed `BatchTooLarge`
- [ ] Smoke test (env-gated) sends one HTML email + verifies delivery via `get_email` polling
- [ ] All file sizes within budget; per-file ≤ briefed; total ≤ 700 LOC
- [ ] No migration; no Rust changes outside the new crate + workspace `members`
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-mcp-connector-resend`
- PR title: `mcp-connector: Resend (transactional email: send / get / list-domains / batch; outbound v1) (mission-mcp-connector-resend)`

## Notes for the implementer

- Treat just-shipped `crates/argo-mcp-connector-sendgrid/` and `crates/argo-mcp-connector-mailgun/` as 1:1 templates for crate layout + JSON-RPC + auth + rate-limit + test shape.
- Resend's API is the cleanest of the three (SendGrid / Mailgun / Resend) — the `POST /emails` request body is a single flat JSON object. Don't over-engineer the request builder.
- Resend keys are `re_xxxxxxxxxxxx`-prefixed; do NOT validate the prefix in the connector (operators rotating keys may use sandbox vs production prefixes the connector shouldn't gate on).
- The `tags` field accepts up to 10 `{ name, value }` pairs — useful for operator analytics. Don't enforce the limit in the connector (Resend returns a clear error); do document it in rustdoc.
- HTML + text bodies: at least one of `html` or `text` must be present (Resend enforces this server-side). The connector can pass through; let Resend's error surface naturally. Typed error: `InvalidEmail { reason: "missing html or text" }`.
- DO NOT cache the API key — read fresh from env on each call (cheap; env-var read is O(1)).
- DO NOT implement broadcasts / audiences / templates — those are v2 if operators ask.
- DO NOT auto-verify domains. Domain verification is a one-time DNS-record setup the operator does in the Resend dashboard.
- DO NOT implement Resend's webhook signature verification in this brief — that's the gateway-route inbound brief.
- The smoke test's `from` address must be a verified domain in the operator's Resend account. Document this in the smoke's header comment with a clear error message if the env var is unset.
- Resend's default region is `us-east-1`; operators on Resend's EU edition use a different base URL (`https://api.eu.resend.com/`). Support a `ARGO_RESEND_REGION` env (`us` | `eu`) overriding the base URL; default `us`.
