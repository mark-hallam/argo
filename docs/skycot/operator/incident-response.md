# Post-launch incident response — operator runbook

Operator-facing playbook for incidents that surface **after** launch
week — customer-reported issues, billing webhook failures, magic-link
delivery problems, audit-chain alarms, isolation alerts. Read end-to-
end once; keep open during an active incident.

Companions:

- [`docs/skycot/runbook/incident-response.md`](../runbook/incident-response.md) — runtime failure modes (worker OOM, migration apply, gateway 5xx) with diagnose/contain/mitigate primitives.
- [`docs/skycot/launch/war-room-runbook.md`](../launch/war-room-runbook.md) — launch-week war-room (Mon-Wed PT), different cadence + roles.
- Per-class playbooks under [`incident-playbooks/`](./incident-playbooks/).
- [`post-incident-review-template.md`](./post-incident-review-template.md) — blameless PIR.
- [`customer-comms-templates.md`](./customer-comms-templates.md) — status page, email, in-product banner.

## Severity

Pick from observed impact, not suspected defect. When in doubt, take
the higher tier — downgrade is cheap.

| Sev | Definition | Pager | Customer comms |
|-----|------------|-------|----------------|
| P0 | Cross-tenant data exposure, audit-chain break, total outage | on-call now | status page within 15 min |
| P1 | Revenue-impacting (billing, magic-link, signups blocked) | business hours | status page within 30 min if user-visible |
| P2 | Single tenant or single connector degraded | log + queue | direct email to affected tenant |

P0 and P1 supersede normal merge/deploy cadence. P2 batches into the
next scheduled deploy.

## Detection signals

Five sources. An incident usually fires from one; runbook entry from
any one is valid.

| Source | What it tells you |
|--------|-------------------|
| Prometheus alerts (#1775) | `argo_request_errors_total`, `argo_audit_chain_breaks_total`, `argo_metric_cardinality_capped_total` thresholds |
| `argo doctor --json` | binary self-check: subsystems, provider, audit-chain continuity, queue health |
| status.skycot.com (#1760) | public status; external customers see this first |
| Support tickets | customer-reported symptoms ("didn't get email", "I see someone else's data") |
| Provider dashboards | Stripe, Postmark/SendGrid, Anthropic, Fly.io — upstream state |

## Incident classes

| Class | Detection signal | Playbook |
|-------|------------------|----------|
| Gateway 5xx spike | `argo_request_errors_total` > 1% / 5min OR `/livez` ≠ 200 | [gateway-5xx](./incident-playbooks/gateway-5xx.md) |
| Magic-link delivery | provider bounce rate > 5% OR support ticket | [magic-link-failure](./incident-playbooks/magic-link-failure.md) |
| Stripe webhook failures | `argo billing dlq list` unresolved OR Stripe dashboard | [stripe-webhook-failure](./incident-playbooks/stripe-webhook-failure.md) |
| Audit chain break (P0) | `argo audit verify-chain` non-zero OR `argo_audit_chain_breaks_total > 0` | [audit-chain-break](./incident-playbooks/audit-chain-break.md) |
| Quota / rate-limit | `argo_metric_cardinality_capped_total` non-zero OR rate-limit spike | [quota-rate-limit](./incident-playbooks/quota-rate-limit.md) |
| Tenant isolation alert (P0) | cross-tenant capability denial spike OR support ticket | [tenant-isolation-alert](./incident-playbooks/tenant-isolation-alert.md) |

Each playbook follows the same shape: **detection → triage →
mitigation → rollback → notes**.

## On-call rotation

- **Primary** — operator. Pager-eligible 24×7 for P0, business-hours
  for P1, queue-only for P2.
- **Alternate 1** — secondary on-call. Covers primary's off-hours and
  vacation.
- **Alternate 2** — backup. Engaged on P0 incidents lasting >1 hour,
  or when primary + alternate-1 are both unavailable.

Rotation schedule is maintained in the on-call calendar (linked from
the operations channel). Handoffs at 09:00 local on Mondays; outgoing
on-call notes any open incidents in the channel.

## Channels + status page flow

1. **Operations channel** — first post within 5 min of detection.
   Template in [customer-comms-templates.md](./customer-comms-templates.md).
2. **Pager** — P0 pages primary immediately. P1 pages during
   business hours; otherwise log + queue.
3. **status.skycot.com** — public status page (PR #1760). For P0,
   post the acknowledgement template within 15 min. For P1 user-
   visible incidents, post within 30 min. P2 does not update the
   public page unless multiple tenants report the same symptom.
4. **Customer email** — direct comms to affected tenants for P1/P2;
   broadcast for P0 once root cause is known. Template in
   [customer-comms-templates.md](./customer-comms-templates.md).
5. **In-product banner** — if the SPA is still serving traffic,
   render a SEV-tagged banner. Banner copy in the comms templates.

## Escalation

Operator may escalate at any point. Triggers:

- P0 still uncontained after 30 min.
- Audit-chain break or tenant-isolation alert — escalate immediately,
  do not wait for the 30-min clock.
- Any incident touching a regulated workload (HIPAA / SOC 2 scope).
- Loss of read-side observability (doctor + Prometheus both silent).

Escalation = engage alternate-2 + capture an
`argo incident bundle --since 1h` snapshot before further changes.

## Post-incident

Within 5 business days of resolution:

1. Author the PIR using
   [`post-incident-review-template.md`](./post-incident-review-template.md).
2. File action items into `docs/build-plan/BUILD_STATUS.md`
   (track-fit) or `docs/build-plan/MISSION_BACKLOG.md`
   (mission-level). No owner + no deadline = wish, not action item.
3. Publish a customer-facing summary on status.skycot.com (P0 / P1).
4. Schedule a 30-min review on the operations channel; outcomes
   appended to the PIR.

## Appendix — exporting a session for support

When handing a session to Skycot support (or to an internal SRE) for
offline analysis, use `argo replay export` to capture a self-contained
bundle.

```bash
# Full-fidelity bundle (includes secrets — handle the file accordingly).
argo replay export <session-id> --out /tmp/session-<session-id>.tar.gz

# Sanitized bundle — provider keys, JWTs, emails, phone numbers redacted.
argo replay export <session-id> --out /tmp/session-<session-id>.sanitized.tar.gz --sanitize

# Uncompressed tar (when the destination is already compressed).
argo replay export <session-id> --out /tmp/session-<session-id>.tar --uncompressed
```

Bundle layout:

```
session-<id>.tar.gz
├── manifest.toml        # session metadata, sha256, sanitized flag, audit_row_count
├── audit/
│   ├── chain.jsonl      # full audit rows in seq order (canonical JSON, one per line)
│   └── verify.txt       # `argo audit verify-chain` summary at export time
├── skills/<id>.json     # attached skill ids + attach timestamp
├── bundle/action-<seq>.json  # per-tool-call request/result payloads
└── README.md            # short replay instructions
```

`manifest.toml` records `sha256 = "..."`, the SHA-256 of every bundled
file's bytes in sorted-path order. A mismatch on re-hash means the
bundle was tampered with after export.

Re-verify the audit chain offline:

```bash
mkdir -p session-replay && tar xzf session-<session-id>.tar.gz -C session-replay/
argo audit verify-chain --from-file session-replay/audit/chain.jsonl
```

`--sanitize` walks every JSON entry and replaces string values
matching: provider keys (`sk-…`), JWTs (`eyJ…`), email addresses
(RFC-5322-ish), phone numbers (E.164-ish). Replacements are
deterministic — same input → same `<label>-redacted-<hmac>` placeholder
— so cross-referencing redacted bundles still works. Field names and
JSON structure are preserved; only string values are touched.

Full-fidelity bundles contain whatever the session captured, including
secrets and PII. Treat the file like the live database.
