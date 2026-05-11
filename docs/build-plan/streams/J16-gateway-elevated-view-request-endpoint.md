# Stream J16 — Gateway elevated-view-request endpoint + grant table

**Owner**: claude. **Depends on**: J2 MERGED (redaction policy reads grant rows).
**Authored against**: origin/main `19eea823` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J16-gateway-elevated-view-request-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 16 of 18 — unblocks frontend E5-g from client-only grant mock.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Approval gate | Auto-grant in v1 (logs the request + grants immediately for the configured duration) | Operator-approval-gated workflow with queue + approver notification | `mission-enterprise-elevated-view-approval` (named in E5-g) |
| Duration cap | 24h max | Tenant-policy-defined max with default 4h | `mission-enterprise-elevated-view-duration-policy` |
| Audit emission | Every request + every grant emits an audit row | Same | n/a |
| Revocation | Operators may revoke active grants via PATCH/DELETE on the grant | Same; v1 ships this | n/a |
| Per-category scoping | Request specifies which categories to unlock; grant scoped accordingly | Same | n/a |
| Concurrent grants | Multiple grants per viewer allowed; J2 unions them | Same | n/a |

v1 is acceptable: auto-grant gets the UX flow working;
the approval-gated workflow is the obvious follow-up
when enterprise customers ask for it.

## Why

E5-g's frontend RBAC redaction layer fires
`POST /api/v1/audit/elevated-view-request` to request
temporary unmask of redacted categories. v1 client
manages grants in localStorage as a mock; this brief
makes the grant table real so J2's server-side fence
honors it consistently across sessions + devices.

## Scope

- NEW table `elevated_view_grant`:
  ```
  id          uuid PK
  tenant_id   text NOT NULL
  principal_id text NOT NULL
  categories  text[] NOT NULL  -- ["email", "phone", ...]
  reason      text NOT NULL
  granted_at  timestamptz NOT NULL DEFAULT NOW()
  expires_at  timestamptz NOT NULL
  revoked_at  timestamptz NULL
  ```
- 3 endpoints:
  - `POST /api/v1/audit/elevated-view-request` body
    `{ reason, categories, duration_hours }`
    → `201 { grant_id, expires_at }`.
  - `GET /api/v1/audit/elevated-view-grants?active=true`
    → `{ grants: [...] }` for current viewer.
  - `DELETE /api/v1/audit/elevated-view-grants/<id>`
    → `204` (operator-only OR self-revoke).
- J2 redact_policy resolver queries this table on
  every redaction decision (cached for the request
  lifetime to bound DB hits).
- Audit row emitted on request + grant + revoke.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: NEW migration; dispatch files
  for POST/GET/DELETE.

## Files you may touch

- `crates/argo-store/migrations/NNNN_elevated_view_grant.sql`
  — NEW. CREATE TABLE + index on
  `(principal_id, revoked_at)`. ≤ 80 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_elevated_view.rs`
  — NEW. 3 handlers. ≤ 380 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_elevated_view_tests.rs`
  — NEW. ≥ 12 tests. ≤ 400 LOC.
- `crates/argo-redact/src/grants.rs` — narrowly
  scoped: replace the mock with real DB lookup.
  ≤ 180 LOC after.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- 3 dispatch files: 1 arm each.

## Files you must NOT touch

- `crates/argo-redact/src/patterns.rs`,
  `redact.rs`, `policy.rs` — read-only except for
  `grants.rs`.
- Other Track-J handler files.

## Acceptance criteria

- [ ] Migration applies cleanly + creates indices.
- [ ] `POST /api/v1/audit/elevated-view-request`:
      - Validates reason ≥ 20 chars.
      - Validates categories ⊆ {email, phone,
        card, iban, ird, classification,
        approver, counterparty}.
      - Validates duration_hours ∈ [1, 24].
      - Inserts grant row with
        `expires_at = NOW() + duration_hours * 1h`.
      - Emits audit row (`elevated_view.requested`)
        + (`elevated_view.granted`) — v1 auto-
        grants so both fire on the same request.
      - 201 + `{ grant_id, expires_at }`.
- [ ] `GET /api/v1/audit/elevated-view-grants?active=true`:
      - Returns viewer's active grants (not
        expired + not revoked).
      - `?active=false` returns all (including
        expired).
- [ ] `DELETE /api/v1/audit/elevated-view-grants/<id>`:
      - Self-revoke: viewer may revoke their own
        grant.
      - Operator-revoke: operators may revoke
        any grant in their tenant.
      - Sets `revoked_at = NOW()`.
      - Emits audit row (`elevated_view.revoked`).
      - 204.
- [ ] J2 grants lookup queries the table:
      `WHERE principal_id = $1 AND revoked_at IS NULL AND expires_at > NOW()`.
- [ ] Per-request cache: J2 caches the query
      result for the duration of one request.
- [ ] Cross-tenant or cross-principal access on
      individual grants → 403.
- [ ] Bearer auth required (401 otherwise).
- [ ] 12 unit tests pass.
- [ ] `cargo build -p argo-cli -p argo-redact`
      clean.
- [ ] `cargo clippy --workspace --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J16-gateway-elevated-view-request-endpoint`
- PR title: `gateway: elevated-view-request endpoint + grant table (J16)`

## Notes for the implementer

- **Grant lifecycle.**
  ```
  Requested → (v1: auto-)Granted → Active → Expired
                                        ↘ Revoked
  ```
  v1.5 hardening brief adds the operator-approval
  step between Requested and Granted.

- **Auto-grant copy.** Response body explicitly
  includes a note about the auto-grant for v1:
  ```json
  {
    "grant_id": "...",
    "expires_at": "...",
    "auto_granted": true,
    "note": "v1 auto-grants elevated-view requests; v1.5 will require operator approval."
  }
  ```
  Frontend renders this note in the dialog
  confirmation toast — sets accurate expectations.

- **Per-request grant cache.** J2's redact policy
  computes once per request lifecycle:
  ```rust
  pub struct PolicyResolver {
      grant_cache: OnceCell<Vec<Grant>>,
  }
  impl PolicyResolver {
      fn grants(&self, principal_id: &str, db: &Db) -> &[Grant] {
          self.grant_cache.get_or_init(|| db.load_active_grants(principal_id))
      }
  }
  ```

- **Audit-row content.**
  - `elevated_view.requested`: actor + categories +
    reason + duration.
  - `elevated_view.granted`: grant_id + categories +
    expires_at.
  - `elevated_view.revoked`: grant_id + revoker_id.

- **Self-revoke convention.** Viewers can revoke
  their own active grants ("oh I shouldn't have
  asked for that"); the audit shows the self-
  revoke.

- **`Sweep job for expired grants.** Out of scope
  here. Grants remain in the table indefinitely
  (audit-trail). J2's query excludes them via
  `WHERE expires_at > NOW()`.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-elevated-view-approval`
  (named).
- `mission-enterprise-elevated-view-duration-policy`
  — tenant-policy-defined max duration.
