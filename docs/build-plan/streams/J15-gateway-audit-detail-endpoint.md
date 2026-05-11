# Stream J15 — Gateway audit detail endpoint (`GET /api/v1/audit/<actionId>`)

**Owner**: claude. **Depends on**: J2 MERGED; J14 MERGED.
**Authored against**: origin/main `5906f806` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J15-gateway-audit-detail-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 15 of 18 — unblocks frontend E5-c.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Hash chain delivery | Pre-computed by gateway + included in response | Same | n/a |
| Screenshot delivery | CU action screenshots served via signed-URL from object storage | Plus per-screenshot retention policy aligned to compliance | `mission-enterprise-screenshot-retention` |
| Cross-system jump-link payload | URL list returned; client renders | Server-validated jump-link allowlist per tenant | `mission-enterprise-jump-link-allowlist` |
| Output redaction | J2 fence applied to output body | Same | n/a |
| Stripe-style external-ID detection | Server pre-scans output for known external IDs and returns them as structured `external_refs[]` | Same; cleaner than client-side detection | n/a |

v1 is acceptable: detail endpoint is read-only over
audit_record + linked rows; the redaction fence is
the only mutation point.

## Why

E5-c's frontend brief calls `GET /api/v1/audit/<id>`
to render the drill-in. Until this exists, E5-c
falls back to mock data. This brief lands the
detailed payload + screenshot serving.

## Scope

- 1 JSON endpoint:
  - `GET /api/v1/audit/<actionId>` →
    `200 { detail: AuditDetail }`.
- Returns the full `AuditDetail` shape per E5-c's
  locked type:
  - Summary (worker / verb / approver /
    timestamp / status / channel / cost)
  - Inputs (trigger + predecessor)
  - Decision chain (8 steps)
  - Provenance (5 hashes + signing info)
  - Output (text / json / CU-screenshots)
- Tenant-scoped + J2 redaction applied.
- 404 on unknown id or cross-tenant.
- Screenshots served via signed-URL pattern:
  the detail response includes `screenshots[i].url`
  pointing at
  `/api/v1/audit/<id>/screenshots/<idx>?signature=...&exp=...`
  that the client follows.
- External-ref pre-scan: server detects known
  patterns (Stripe / Slack / GitHub / etc.) in the
  output body and returns them as a structured
  `external_refs[]` array so client can render
  jump-links without re-scanning.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_get.rs`
  (1 arm).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs`
  — NEW. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_audit_screenshot.rs`
  — NEW. Signed-URL screenshot endpoint. ≤ 250
  LOC.
- `crates/argo-cli/src/handlers/gateway/routes_audit_detail_tests.rs`
  — NEW. ≥ 12 tests. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 2-line `pub mod` additions.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 2 arms (detail + screenshot).
  ≤ 370 LOC after.

## Files you must NOT touch

- `crates/argo-redact/**` — read-only.
- `crates/argo-audit/**` — read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/audit/<actionId>` returns 200 +
      `{ detail: AuditDetail }`.
- [ ] `AuditDetail` shape (matches E5-c locked
      type):
      ```json
      {
        "summary": { ... },                  // same as J14 AuditRow
        "inputs": {
          "trigger_kind": "message|cron|dependent_action",
          "trigger_summary": "...",
          "predecessor": { "id": "...", "verb": "..." } | null
        },
        "decision_chain": [
          { "n": 1, "title": "Memory loaded", "body": "...", "details": { ... } },
          ...
        ],
        "provenance": {
          "model": { "id": "claude-opus-4-7", "version": "..." },
          "prompt_template": { "id": "...", "hash": "..." },
          "skill": { "id": "...", "version": "..." },
          "memory_snapshot": { "id": "..." },
          "policy": { "id": "...", "version": "...", "hash": "..." },
          "hash_chain_anchor": "<final hash>",
          "signature": "<ed25519>"
        },
        "output": {
          "kind": "text|json|computer-use",
          "body": "..." | { ... },
          "screenshots": [
            { "url": "/api/v1/audit/<id>/screenshots/0?...", "timestamp": "...", "description": "..." }
          ] | null,
          "delivery": { "sent_at": "...", "received_at": "..." } | null,
          "follow_up": { "id": "...", "verb": "..." } | null
        },
        "external_refs": [
          { "system": "stripe", "id": "re_3O...", "deep_link": "https://dashboard.stripe.com/refunds/re_3O..." }
        ]
      }
      ```
- [ ] J2 redaction applied to summary / inputs /
      decision-chain bodies / output body.
- [ ] Screenshot endpoint
      `/api/v1/audit/<id>/screenshots/<idx>`:
      - HMAC-signature in URL: validates token
        before serving.
      - Token expires 10 minutes after issuance.
      - Returns the raw PNG bytes.
      - 401 on invalid/expired signature; 404 on
        unknown idx.
- [ ] External-ref pre-scan: server detects up to
      30 references per output (cap to bound
      compute); allowed systems: stripe / slack /
      github / linear / salesforce / hubspot /
      zendesk / jira.
- [ ] Cross-tenant access → 404 (don't leak
      existence).
- [ ] Unknown actionId → 404.
- [ ] Bearer auth required (401 otherwise).
- [ ] Audit row written per detail view
      (`audit.detail.viewed`).
- [ ] Response time budget: p99 < 250ms (cold);
      p99 < 50ms (warm — most data comes from the
      same query).
- [ ] 12 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J15-gateway-audit-detail-endpoint`
- PR title: `gateway: GET /api/v1/audit/<id> detail + screenshot endpoints (J15)`

## Notes for the implementer

- **Decision chain composition.** The audit_record
  table doesn't directly store an 8-step chain;
  this brief composes it from:
  - `memory_loaded_snapshot_id` → step 1
  - `skill_id` + `skill_version` + `skill_trust`
    → step 2
  - `plan_candidates_json` → step 3
  - `plan_selected_idx` → step 4
  - `policy_id` + `policy_outcome` → step 5
  - `approver_id` + `approval_channel` → step 6
    (if applicable)
  - `executor_kind` + `executor_meta` → step 7
  - `outcome_status` + `downstream_meta` → step 8

  If some columns don't exist in audit_record yet,
  this brief adds them via migration as part of the
  acceptance criteria.

- **Hash chain anchor.** Each audit_record row
  carries `hash_chain_anchor` (the running
  SHA-256 chain hash after this row). Already
  computed at write-time; just emit in response.

- **External-ref pre-scan rules.** Use the same
  regex set as E5-h's frontend detector:
  - `re_<id>`, `ch_<id>`, `cus_<id>`, etc. for
    Stripe
  - `https://*.slack.com/archives/*` for Slack
  - `https://github.com/<owner>/<repo>/issues|pull/<n>` for GitHub
  - etc.
  Server runs once, emits structured array;
  client renders without re-scanning.

- **Screenshot signed URL.**
  ```
  /api/v1/audit/<actionId>/screenshots/<idx>
    ?exp=<unix_ts>
    &sig=<hmac_sha256(actionId || idx || exp, server_secret)>
  ```
  10-minute TTL. On request: validate `exp > NOW`
  + signature matches. Return PNG bytes.

- **Audit-the-audit.** Detail view emits an
  audit row with `actor_id`, `viewed_action_id`,
  `viewer_role`. Used by enterprise compliance to
  detect anomalous viewing patterns.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-screenshot-retention` —
  per-screenshot retention policy.
- `mission-enterprise-jump-link-allowlist` —
  server-validated jump-link allowlist per
  tenant.
