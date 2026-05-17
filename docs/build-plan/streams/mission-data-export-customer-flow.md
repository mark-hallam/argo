# mission-data-export-customer-flow — customer-facing data export (GDPR/CCPA Article 15) (P1)

**Owner**: claude. **Depends on**: `mission-tenant-substrate-tables` MERGED (#1777) + `claude-magic-link-auth-backend` MERGED (#1796 pending).
**Authored against**: origin/main `37f268a6` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-data-export-customer-flow`.
**Source**: 2026-05-17 audit blind-spot — customer-ops gap. `argo gdpr delete` exists (#866) but `argo gdpr export` / customer-facing self-serve data download doesn't. GDPR Article 15 / CCPA "right to know" requires this before public launch in EU/CA.

## Mission alignment

- **Commercial usability** (CLAUDE.md priority #5) — enterprise procurement + EU/CA compliance.
- **Security** (#4) — exposing data export through proper RBAC, not as a side-channel.

## Scope

P1. Three deliverables:

### 1. Customer-facing export endpoint

`GET /api/v1/me/export` — authenticated user can download all their data as a single tar.gz:

```
my-skycot-export-<date>.tar.gz
├── manifest.json              # version, tenant_id, user_id, exported_at, what's-included
├── user_account.json          # email, display_name, created_at, last_login_at
├── tenants.json               # tenants the user belongs to (id, name, role, joined_at)
├── workers.jsonl              # all workers the user owns
├── audit/
│   └── chain.jsonl            # all audit rows scoped to this user (NOT cross-user)
├── memory/
│   └── rows.jsonl             # memory rows where user is author or owner
├── skills/
│   └── *.json                 # skills authored by this user
└── messages/
    └── outbound.jsonl         # outbound messages where user is sender (#1807 outbound_message)
```

Cross-user data (e.g. messages received from other users) NOT included — that's the OTHER user's data per GDPR Article 15 boundaries.

### 2. SPA `/settings/data-export` page

User clicks "Download my data" → POST request → polls until ready (job pattern) → download link → 7-day expiry on download.

### 3. Audit + rate limit

Each export request:
- Emits `AuditKind::DataExportRequested` with `user_id` + `tenant_id` + `requested_at`.
- Rate-limited: 1 export per user per 24h (configurable).
- Token-based download link, single-use, 7-day expiry.

LOC: ~500.

## Files you may touch

**Backend:**
- `crates/argo-cli/src/handlers/gateway/routes_me_export.rs` (NEW) — endpoint + job pattern. ≤150 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_me_export_tests.rs` (NEW). ≤100 LOC.
- `crates/argo-store/migrations/115__data_export_job.sql` (NEW) — export job tracking. ≤25 LOC. (Verify next-free slot at claim — last MERGED was 114.)
- `crates/argo-store/src/data_export_job_query.rs` (NEW) — CRUD. ≤80 LOC.
- `crates/argo-identity/src/data_export.rs` (NEW) — pure logic: gather user data from existing stores. ≤180 LOC.
- `crates/argo-identity/src/data_export_tests.rs` (NEW). ≤100 LOC.
- `crates/argo-cli/src/handlers/gateway/boot_router.rs` — mount new route. ≤8 modified LOC.

**SPA:**
- `web/app/(app)/settings/data-export/page.tsx` (NEW). ≤120 LOC.
- `web/lib/data-export/use-export-request.ts` (NEW) — polling hook. ≤60 LOC.
- `web/__tests__/data-export/*.test.tsx` (NEW). ≤80 LOC.

**Operator runbook:**
- `docs/skycot/operator/data-export-policy.md` (NEW) — operator-facing GDPR/CCPA policy + how to handle requests + retention. ≤80 LOC.

## Files you must NOT touch

- `argo gdpr delete` (existing #866) — different surface (delete vs export). Don't conflate.
- Audit chain hash logic — read-only consumer.
- Other users' data — strict scope: only the requesting user's records.

## Acceptance criteria

- [ ] `GET /api/v1/me/export` returns 200 + a job ID for the requesting authenticated user.
- [ ] Polling endpoint returns status (queued/running/ready/expired) + download URL when ready.
- [ ] Downloaded tarball matches the documented layout; manifest.json has correct version + scope.
- [ ] Cross-user data NOT included (test: user A's export does NOT contain user B's audit/memory/message rows).
- [ ] Rate limit: 2nd export request within 24h → 429.
- [ ] Audit row `DataExportRequested` written on each request.
- [ ] Token-based download link expires after 7 days.
- [ ] `/settings/data-export` SPA page works in dark + light mode.
- [ ] `cargo test -p argo-cli -p argo-store -p argo-identity` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` clean for touched crates.
- [ ] `pnpm test && pnpm typecheck` clean.
- [ ] Operator runbook documents GDPR/CCPA timeline + how to handle a request manually if the auto export fails.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-data-export-customer-flow`
- Title: `feat(gdpr): customer-facing data export + /me/export endpoint + SPA page (mission-data-export-customer-flow)`
- Worker verification: crate-scoped + workspace clippy + web tests + per SWARM_PROTOCOL.md.

## Hidden constraints

- **Pre-claim check: confirm #1796 magic-link MERGED.** Without auth, /me/export has no `Principal`. Worker self-exits if magic-link not on main.
- **Tarball assembly happens in a background job.** Don't block the API request on multi-GB tarball generation. Use the existing replay-export streaming pattern (#1778) as reference.
- **Data scope is STRICT.** Only rows where the user is owner/author/sender. Tenant-level data (org-wide settings, etc.) NOT included unless the user is the tenant owner.
- **No PII redaction.** This is the USER's data — export it as-is. Sanitization is for replay-export's `--sanitize` flag, different use case.
- **30-day legal SLA.** GDPR Article 12 says respond within 30 days. This auto-flow handles it in minutes.
- **Customer-facing UX** — first time the user sees their own data. Use brand-aligned EmptyState if no data; show progress bar during generation; explain what's included + what's not.
- **Closes audit blind-spot** for customer-ops GDPR/CCPA compliance.
