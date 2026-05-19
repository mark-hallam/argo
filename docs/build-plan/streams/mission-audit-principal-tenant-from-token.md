# Stream Audit-P0-2 — principal + tenant from authenticated credential, reject mismatched headers

**Owner**: claude.
**Depends on**: none. Coordinates with PR #1835 (`__Host-session` cookie) but does not depend on it landing first.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-principal-tenant-from-token`.
**Source**: 2026-05-19 codex audit P1-2, validated by claude `VALIDATION.md` (CONFIRMED — zero cross-check between bearer token and supplied principal/tenant headers).

## Scope

Bearer auth proves only token possession. `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs:43-64` parses `X-Argo-Principal` and `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs:347-359` parses `X-Argo-Tenant-Id` with a `00000000-0000-0000-0000-000000000000` workspace fallback. Neither header is cross-checked against the bearer token. `crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs:17` mirrors the same pattern.

Impact: a shared or leaked bearer token can stamp arbitrary principals and request arbitrary-tenant-scoped operations by header alone. This breaks enterprise RBAC and forensic-audit assumptions.

This brief introduces a `RequestPrincipal` middleware-derived value that comes from the **authenticated credential** (the bearer token entry in a `service_token_binding` table, or — once PR #1835 lands — the `__Host-session` cookie's signed session). Caller-supplied `X-Argo-Principal` / `X-Argo-Tenant-Id` headers must either match the authenticated principal/tenant, or the request is rejected with `403 Forbidden`. Bearer-token-only callers without a binding entry get the workspace default tenant and a `principal=service:<token-fingerprint>` synthetic value — never a caller-supplied principal.

Approach (scope-minimum, not a full IAM redesign):

1. New table `service_token_binding (token_hash TEXT PRIMARY KEY, principal_id TEXT, tenant_id TEXT, label TEXT, created_at TEXT)` — opt-in. Operators bind tokens at provision time via a new CLI: `argo gateway token bind --token <T> --principal <P> --tenant <T>`.
2. `check_bearer_auth` returns `Option<AuthenticatedPrincipal { principal, tenant_id, source }>` instead of `Option<Response>`. The `source` is `Bound | TokenSyntheticPrincipal | DevBypass`.
3. New helper `enforce_header_match(req, auth_principal) -> Result<(), Response>`: if `X-Argo-Principal` is present, it must equal `auth_principal.principal` (else 403). Same for `X-Argo-Tenant-Id`.
4. The three routes (`routes_auth_principal.rs`, `routes_skill_ir.rs`, `routes_ghostwriter.rs`) call `enforce_header_match` before any business logic.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__service_token_binding.sql` (NEW) — schema for the binding table. Indexed by `token_hash`. Tenant column NOT NULL, principal_id NOT NULL.
- `crates/argo-store/src/service_token_binding.rs` (NEW) — `pub async fn lookup_binding(pool, token_hash) -> ArgoResult<Option<TokenBinding>>` + the struct. SHA-256 hash of the token string is what's stored (never the raw token).
- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` — change `check_bearer_auth` return type from `Option<Response>` to `AuthCheckOutcome { Pass(AuthenticatedPrincipal) | Reject(Response) }`. Existing call-sites that only checked for `None` now match the new enum. Token-source resolution: hash incoming token → lookup binding → if found, return `AuthenticatedPrincipal { source: Bound, .. }`; if not, return `AuthenticatedPrincipal { source: TokenSyntheticPrincipal, principal: "service:<hex-fingerprint>", tenant_id: WORKSPACE_DEFAULT }`.
- `crates/argo-cli/src/handlers/gateway/routes_auth_principal.rs` — keep `parse_principal_header` as a pure parser (no policy). Replace `derive_principal` semantics: header value (if any) is compared against `auth_principal.principal`; mismatch returns `403 Forbidden` with body `principal-header mismatch`.
- `crates/argo-cli/src/handlers/gateway/routes_skill_ir.rs` — `resolve_tenant` uses `auth_principal.tenant_id`; if `X-Argo-Tenant-Id` header is present, it must match (mismatch → 403). Remove the silent workspace-default fallback when an authenticated principal is bound to a non-default tenant.
- `crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs` — same treatment as `routes_skill_ir.rs`.
- `crates/argo-cli/src/handlers/gateway_token_bind.rs` (NEW) — `argo gateway token bind` subcommand. Reads token from env or `--token-file <path>` (never from argv directly), inserts into `service_token_binding`. Symmetric `argo gateway token unbind --token <T>`.
- `crates/argo-cli/tests/gateway_principal_header_negative.rs` (NEW, follows existing `cross_tenant_skill_discovery_negative.rs` shape) — five test cases: (1) bound token + matching headers → 200; (2) bound token + mismatched `X-Argo-Principal` → 403; (3) bound token + mismatched `X-Argo-Tenant-Id` → 403; (4) unbound token + caller-supplied principal header → 403; (5) no auth + headers → 401 (no bypass).

## Files you must NOT touch

- `crates/argo-capability/` — capability-engine semantics are unchanged. This brief is about deriving the principal cleanly; the engine's role-based checks already work once principal is honest.
- The audit chain (`crates/argo-audit/`) — separate brief covers permission-grant audit events.
- Anything in `web/` — the SPA already authenticates via the upcoming `__Host-session` cookie (PR #1835); when that lands, this brief's principal-derivation will compose with it.
- Other gateway routes that don't read principal/tenant from headers — only the three cited routes change behavior.

## Acceptance criteria

- [ ] New `service_token_binding` table + migration. Migration slot validation passes (`bash scripts/check-migration-slots.sh`).
- [ ] `check_bearer_auth` returns `AuthCheckOutcome` enum; all 3 cited routes consume the bound principal/tenant instead of header-supplied values.
- [ ] Headers are accepted ONLY when they match the authenticated principal/tenant; mismatch returns `403`.
- [ ] Tokens with no binding row get `principal=service:<fingerprint>` and `tenant_id=workspace_default`. Caller-supplied principal/tenant headers on such tokens are rejected.
- [ ] `argo gateway token bind|unbind|list` subcommand works; reads token from `--token-file` (not argv), persists to DB, surfaces via `argo gateway token list` (label + tenant only, never the raw token).
- [ ] Five new negative tests in `gateway_principal_header_negative.rs` pass.
- [ ] Migration is additive — no breaking changes to existing routes when token is unset (dev bypass remains the third `AuthCheckOutcome::Pass(source=DevBypass)` variant).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean — claim in PR body.
- [ ] `cargo test -p argo-cli -p argo-store` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-audit-principal-tenant-from-token`
- PR title: `gateway: derive principal+tenant from token binding, reject mismatched headers (mission-audit-principal-tenant-from-token)`

## Notes for the implementer

- This brief is **deliberately narrow**: it does not move to JWT-signed bearer tokens, does not introduce a session table, and does not refactor `Principal` upstream. The `service_token_binding` is the minimum-viable trust anchor.
- When PR #1835 lands, the `__Host-session` cookie path will compose naturally: cookie-derived principal/tenant is the `AuthCheckOutcome::Pass(source=Session)` case, and the same `enforce_header_match` logic applies.
- The token fingerprint for synthetic principals: `principal = format!("service:{}", &token_sha256[..16])`. Stable across requests with the same token. Never invertible.
- Audit emission for token-bind events should go through the same audit chain as the upcoming `RoleBindingChanged` kind from `mission-audit-capability-events` (not in this brief — coordinate via the audit kinds list).
- The `argo gateway token list` output must NEVER include the raw token or the full hash — only label, tenant, principal_id, created_at, and the first 8 chars of the fingerprint.

## Out of scope (follow-up briefs)

- JWT-signed bearer tokens with embedded claims (proper crypto-bound principal). Defer until v1.x.
- Per-token scopes / capabilities (today bound tokens get whatever the principal has). Capability-scoping is a separate brief.
- Web `__Host-session` cookie integration — that's PR #1835's scope.
- Multi-tenant admin tokens (principal that can act across tenants) — explicitly out; this brief enforces strict 1:1 binding.
