# Stream J17 — Compliance HIPAA disclosure log generator

**Owner**: claude. **Depends on**: existing compliance-export infrastructure (`mission-compliance-soc2-audit-trail` MERGED + `mission-compliance-gdpr-data-export` MERGED).
**Authored against**: origin/main `9210e333` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J17-compliance-hipaa-disclosure-log`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 17 of 18 — unblocks the HIPAA format option in E5-e's exporter dialog.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| HIPAA schema alignment | Schema follows 45 CFR § 164.528 Accounting of Disclosures (date / recipient / purpose / brief description) | Same; v1 covers the regulatory minimum | n/a |
| PHI identification | Manual operator-marked rows in audit_record (`phi_accessed = true` flag); v1 doesn't auto-classify PHI | LLM-based PHI classification on every audit row | `mission-enterprise-phi-auto-classification` |
| Disclosure recipient | From audit_record's existing `recipient_meta` JSON field | Same | n/a |
| 6-year retention | Schema supports it; retention sweep is the operator's responsibility | Per-tenant retention policy enforced server-side | `mission-enterprise-compliance-retention-policy` |
| Bundle signing | Reuses existing Ed25519 signing (per E5-f verifier) | Per-tenant signing keys | `mission-enterprise-export-kms-signing` (already named E5-e) |

v1 is acceptable: HIPAA disclosure-log schema is well-
defined by 45 CFR; we ship it for tenants that have
opted into HIPAA workflows (healthcare-intake vertical).

## Why

E5-e's evidence bundle exporter offers SOC2 / GDPR /
HIPAA / Generic formats. SOC2 + GDPR are MERGED.
HIPAA is the new format added by this brief. Without
it, E5-e's HIPAA option returns 400 "format not yet
supported".

For enterprise: HIPAA compliance is non-negotiable
for healthcare customers. Without a HIPAA-shaped
disclosure log, the platform isn't deployable in
hospitals / clinics / health-tech vendors.

## Scope

NEW HIPAA disclosure log generator in
`argo-compliance` (existing crate):
- Function `generate_hipaa_disclosure_log(tenant_id,
  from, to) -> Result<Bundle>`.
- Pulls audit_record rows where `phi_accessed = true`
  within the time window.
- Composes a 45 CFR § 164.528-aligned disclosure
  log per row:
  - Date of disclosure
  - Recipient (name + role)
  - Purpose of disclosure
  - Brief description of the PHI disclosed
- Aggregates into a signed Bundle (matches the
  existing SOC2/GDPR bundle shape from E5-e).
- Wires into the E5-e exporter's
  `generate_export(format = "hipaa", ...)` path.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `crates/argo-compliance/`
  (extend, do not rewrite).

## Files you may touch

- `crates/argo-compliance/src/hipaa.rs` — NEW.
  Disclosure log generator. ≤ 350 LOC.
- `crates/argo-compliance/src/hipaa_tests.rs` —
  NEW. ≥ 8 tests. ≤ 350 LOC.
- `crates/argo-compliance/src/lib.rs` — narrowly
  scoped: register the new module + add
  `Format::Hipaa` variant + dispatch arm. ≤ 250
  LOC after.
- `crates/argo-store/migrations/NNNN_phi_accessed_flag.sql`
  — NEW. ALTER TABLE audit_record ADD COLUMN
  `phi_accessed boolean NOT NULL DEFAULT false`.
  ≤ 30 LOC.

## Files you must NOT touch

- `argo-compliance::soc2`, `argo-compliance::gdpr`
  — locked (extend pattern from these but don't
  modify).
- `crates/argo-audit/**` — read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] Migration applies cleanly; existing audit_record
      rows default to `phi_accessed = false`.
- [ ] `generate_hipaa_disclosure_log(tenant, from, to)`
      returns a `Bundle` with:
      - `format: "hipaa-disclosure-log"`
      - `manifest.json`: tenant + time window + count
      - `disclosures.jsonl`: one disclosure per line
      - `signature.bin`: Ed25519 over the bundle
      - `README.md`: HIPAA-aligned documentation
- [ ] Each disclosure JSON shape:
      ```json
      {
        "id": "act_01H...",
        "date_of_disclosure": "<ISO>",
        "recipient": { "name": "...", "role": "..." },
        "purpose": "...",
        "brief_description": "...",
        "audit_record_id": "..."
      }
      ```
- [ ] Only rows with `phi_accessed = true` included.
- [ ] Hash chain anchor included in manifest (matches
      E5-f verifier expectations).
- [ ] Bundle is verifiable by `@skycot/verify`
      (E5-f CLI).
- [ ] Wired into E5-e's exporter; selecting "HIPAA"
      format in the dialog generates the bundle.
- [ ] 8 unit tests pass.
- [ ] `cargo build -p argo-compliance` clean.
- [ ] `cargo clippy -p argo-compliance --all-targets
      -- -D warnings` clean.
- [ ] `cargo test -p argo-compliance` passes.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J17-compliance-hipaa-disclosure-log`
- PR title: `compliance: HIPAA disclosure log generator (J17)`

## Notes for the implementer

- **45 CFR § 164.528 fields.** The Accounting of
  Disclosures rule requires:
  - Date of disclosure
  - Name + address (when reasonably ascertainable)
    of who received the PHI
  - Brief description of the PHI disclosed
  - Brief statement of the purpose of the disclosure

  v1 maps these from existing audit_record fields:
  - `audit_record.created_at` → date
  - `audit_record.recipient_meta` → recipient (JSON
    with name + role + optional address)
  - `audit_record.verb` (PII-redacted via J2) → brief
    description
  - `audit_record.purpose_code` → purpose (operator-
    set per action type, e.g. "treatment",
    "operations", "billing")

  If `recipient_meta` or `purpose_code` columns
  don't exist yet, add them in the same migration
  with sensible defaults.

- **`phi_accessed` flag wiring.** v1: operators
  manually mark which action_types are PHI-touching
  via a per-tenant `phi_action_types` config. The
  audit-emission code on those action_types sets
  the flag. v1.5 LLM classifier replaces the
  config.

- **Bundle structure reuses SOC2 pattern.** Read
  `argo-compliance::soc2::generate_audit_trail`
  for the template; copy its structure with HIPAA-
  appropriate field mapping.

- **README content.**
  ```markdown
  # HIPAA Disclosure Log
  
  This bundle is a 45 CFR § 164.528-aligned
  Accounting of Disclosures for tenant {tenant_id}
  covering {from} to {to}.
  
  ## Verification
  
  Verify bundle integrity:
  
      npm install -g @skycot/verify
      skycot-verify ./bundle.zip
  
  See https://docs.skycot.com/verify for details.
  
  ## Contents
  
  - `manifest.json`: bundle metadata + hash chain
    anchor
  - `disclosures.jsonl`: one disclosure per line
  - `signature.bin`: Ed25519 signature over the
    final hash
  ```

- **Retention.** HIPAA requires 6-year retention
  of disclosure logs. v1 doesn't enforce this
  server-side; the operator is responsible. v1.5
  hardening brief adds tenant retention policy
  enforcement.

- **No new auth.** Generation happens server-side
  triggered by E5-e's call; auth already bound by
  the parent endpoint.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-phi-auto-classification` —
  LLM-based PHI classification per audit row.
- `mission-enterprise-compliance-retention-policy`
  — per-tenant retention policy enforcement.
