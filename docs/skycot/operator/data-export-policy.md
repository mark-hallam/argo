# Data export policy (GDPR Art. 15 / CCPA right-to-know)

Customer-facing self-serve data export landed via
`mission-data-export-customer-flow`. This runbook covers the operator
side: timelines, escalation paths, and how to handle a request
manually if the auto-flow fails.

## Customer flow (auto)

1. The user lands on `app.skycot.com/settings/data-export` and
   clicks **Request export**.
2. The SPA proxies `POST /api/v1/me/export` to the gateway. The
   gateway:
   - Resolves the user from `X-Argo-Principal: human:<email>` (set
     by the SPA edge route from the magic-link session cookie).
   - Refuses with **HTTP 429** if the user already requested an
     export in the last 24 hours (`RATE_LIMIT_WINDOW_SECS`).
   - Inserts a `data_export_job` row + assembles the tarball
     (manifest + `user_account.json` + `tenants.json` +
     `workers.jsonl` + `audit/chain.jsonl` + `messages/outbound.jsonl`).
   - Emits `AuditKind::DataExportRequested` with the user's
     principal stamp.
   - Returns `202 Accepted` with a single-use, SHA-256-hashed
     download URL valid for 7 days.
3. The user clicks **Download .tar.gz** and the SPA serves the
   bundle from `GET /api/v1/me/export/download/<token>`.

## Legal SLA

- **GDPR Article 12.3** — respond within **30 days** of the
  request (extendable to 60 with notice).
- **CCPA §1798.130** — respond within **45 days** (extendable to 90).
- The auto-flow handles every well-formed request in **seconds**,
  well inside both windows.
- If the auto-flow has been broken for >24h, the SLA clock is
  ticking against operators — escalate per the manual section
  below.

## Scope (what's in / what's NOT in)

In (per requesting user):

- The user's own `user_account` row.
- Every `user_tenant` membership row (id, role, joined_at).
- Workers in tenants where the user has `Owner` role.
- Audit rows whose `principal_id` is `human:<email>`.
- Outbound messages from tenants the user owns.

Not in (per GDPR Article 15 boundaries):

- Other users' data (audit rows attributed to other principals,
  messages received from them, memory rows authored by them).
- Tenant-level admin data when the user is `Member`, not `Owner`.
- Memory + skill rows in the v1 schema (tenant-scoped, not
  user-attributed — documented as a known limitation on the
  manifest). The v1.x schema widen will close this gap.
- The `argo gdpr delete` surface is **distinct**; this flow is
  export-only.

## Manual handling (auto-flow degraded)

If the customer reports the auto-flow is broken — or the operator
needs to honour a Subject Access Request received outside the SPA
(email, support ticket, legal letter) — assemble the bundle by
hand:

1. Resolve the user's `id` + `email` from the `user_account` table:
   ```bash
   argo db query "SELECT id, email FROM user_account WHERE email = '<email>'"
   ```
2. Mint the bundle using the same code path the gateway uses:
   ```bash
   argo dev export-user --user-id <uuid> --out /tmp/export.tar.gz
   ```
   (CLI follow-up; until that lands, call the gateway endpoint
   directly with operator bearer + `X-Argo-Principal: human:<email>`.)
3. Verify the bundle's `manifest.json` lists the expected files
   and the `body_bytes` total is non-zero.
4. Deliver the tarball over a customer-trusted channel (the
   support ticket they opened, the email they wrote from). Do
   **not** email the raw bundle to a third party.
5. Record the manual fulfilment in the audit log so the chain
   carries one `data_export.requested` row per real request:
   ```bash
   argo audit append --kind data_export.requested \
     --principal human:<email> \
     --payload '{"manual":true,"reason":"<incident>"}'
   ```

## Retention + cleanup

- Completed bundles persist in `data_export_job.bundle_bytes`
  until `expires_at` (7 days post-completion). A cleanup sweep is
  a v1.x follow-up; until then, prune via:
  ```bash
  argo db query "DELETE FROM data_export_job \
    WHERE status = 'ready' AND expires_at < strftime('%s','now')"
  ```
- Audit rows (`data_export.requested`) are append-only and
  intentionally **not** pruned — they evidence Article 15
  compliance for the regulator.

## Failure modes + responses

- **HTTP 401** on `POST /api/v1/me/export`: the SPA didn't stamp
  `X-Argo-Principal`. Check magic-link session is still valid.
- **HTTP 429** within 24h: expected. The customer must wait.
- **HTTP 500** with `gather:`: the user is missing a substrate
  row. Re-run after `argo db migrate`.
- **HTTP 404** on the download URL: token expired (>7 days) or
  consumed. The user must request a fresh export.

## References

- Brief: `docs/build-plan/streams/mission-data-export-customer-flow.md`
- Backend handler: `crates/argo-cli/src/handlers/gateway/routes_me_export.rs`
- Data gathering: `crates/argo-identity/src/data_export.rs`
- Job store: `crates/argo-store/src/data_export_job_query.rs`
- SPA page: `web/app/(app)/settings/data-export/page.tsx`
- Existing erasure flow: `argo gdpr delete` (#866) — different surface.
