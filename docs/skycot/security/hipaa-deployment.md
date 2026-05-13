# HIPAA-Compatible Deployment Path

SKYCOT can be deployed for healthcare workflows only when the
operator controls the full deployment posture. The platform supplies
technical safeguards and evidence trails; the operator still owns
BAAs, workforce policy, incident response, and provider selection.

This document is a BAA-ready deployment guide, not a declaration that
every SKYCOT installation is HIPAA compliant.

## Security Rule Mapping

| HIPAA safeguard | SKYCOT control | Status |
|---|---|---|
| Access control | Gateway bearer auth, RBAC denials, elevated-view grants, worker identity | Available |
| Unique user identification | Principal stamping on gateway audit rows | Available |
| Emergency access procedure | Elevated-view grants with reason and expiry | Available |
| Audit controls | Hash-chained `audit_record`, watchdog verification, export tooling | Available |
| Integrity controls | Append-only migrations, audit self-hashes, skill provenance | Partial |
| Transmission security | TLS-terminating reverse proxy or future native TLS path | Operator required |
| Encryption at rest | SQLCipher or encrypted volume for SQLite and evidence blobs | Operator required |
| Person/entity authentication | SSO/SCIM follow-ups for staff provisioning | Follow-up required |

## What Ships Today

SKYCOT already has a hash-chained audit log, principal-aware gateway
rows, bearer-token auth, PHI-oriented redaction primitives, worker
identity, and a startup health gate. These give operators a concrete
evidence trail for who accessed what, when, and through which surface.

Gateway startup now emits a `hipaa_compliance_check` audit row. The
row records whether `[security.hipaa] enabled = true` was present and
lists the deployment posture for access control, audit controls,
integrity controls, transmission security, encryption at rest, PHI
fences, disclosure logging, breach notification, and BAA provider
review.

Default mode is `not_claimed`. That means the check runs, but the
deployment is not presenting itself as HIPAA-ready.

## Required Operator Controls

Before processing PHI, the operator must complete these controls:

1. Execute BAAs with every hosted service that can receive PHI,
   including cloud infrastructure, LLM providers, observability
   vendors, email/SMS/voice providers, and backup storage.
2. Terminate TLS at a reverse proxy or approved ingress and keep the
   gateway bound to loopback unless the deployment runbook explicitly
   justifies otherwise.
3. Enable encryption at rest for SQLite, attached blobs, screenshots,
   backups, and exported audit bundles.
4. Configure staff identity with SSO/SCIM or an equivalent unique-user
   process. Shared accounts are not acceptable for PHI.
5. Define a breach-notification owner and escalation path before
   production traffic starts.

## Breach Notification

HIPAA breach notification is operator-owned. SKYCOT should support the
workflow by preserving audit rows, exporting evidence bundles, and
recording incident-review decisions. The operator must assess whether
an incident is a breach and, when required, notify affected parties
without unreasonable delay and no later than 60 days after discovery.

The incident playbook should include:

- Time of discovery and reporter.
- Systems, tenants, workers, and channels involved.
- PHI categories potentially exposed.
- Audit-query commands used to scope access.
- Containment steps and follow-up control changes.

## Follow-Up Briefs

The startup audit row names several gaps deliberately:

- `mission-encryption-at-rest-sqlcipher` for database encryption.
- J17 disclosure-log work for accounting of disclosures.
- SSO and SCIM provisioning for healthcare staff.
- Native TLS or hardened reverse-proxy enforcement for public gateway
  deployments.
- SIEM export for healthcare security monitoring.

Deployments should not claim HIPAA mode until the operator can point
to concrete controls for each gap, either from merged SKYCOT features
or from external infrastructure.
