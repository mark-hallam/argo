# Stream J18 — Compliance generic export (JSON + PDF) (LAST in Track J)

**Owner**: claude. **Depends on**: existing compliance-export infrastructure; J17 MERGED (extends the format dispatch pattern).
**Authored against**: origin/main `9a875f4d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J18-compliance-generic-export`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 18 of 18 (LAST). Closes Track J author phase.

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED**: Same issue as J17. `crates/argo-compliance/` does NOT exist on current main. Compliance functionality lives in `crates/argo-cli/src/handlers/compliance_*.rs`. **Recommended conductor rewrite**: re-target J18 to add `crates/argo-cli/src/handlers/compliance_generic.rs` (NEW, ≤350 LOC) + `compliance_pdf.rs` (NEW, ≤300 LOC, using `printpdf`) sibling to existing `compliance_export.rs`. Extend the existing `ReportFormat` enum in `compliance_soc2.rs` (Json/Csv/Markdown) with a `Generic` variant + dispatch arm. Cargo.toml `printpdf` dep goes on `argo-cli`, not on a nonexistent `argo-compliance` crate.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| PDF rendering | Pure-Rust PDF generation via `printpdf` (no headless-Chrome) | Same; v1 keeps it simple | n/a |
| Schema | Vendor-neutral: documented schema in the bundle's README | Operator-defined custom-format templates per tenant | `mission-enterprise-export-custom-templates` (already named in E5-e) |
| Bundle signing | Reuses existing Ed25519 (E5-f verifier) | Per-tenant signing keys | `mission-enterprise-export-kms-signing` (named) |
| Audit-row coverage | All audit_record rows in window (no filtering) | Per-row RBAC filtering | `mission-enterprise-export-row-rbac` |

v1 is acceptable: generic export is the fallback
when no specific compliance regime applies; clean
documented schema is the value-add.

## Why

E5-e's evidence bundle exporter offers 4 formats:
SOC2 (existing), GDPR (existing), HIPAA (J17), and
Generic. This brief lands the Generic format —
useful when:
- An auditor wants ad-hoc data without a specific
  regime.
- A tenant doesn't fall under SOC2/GDPR/HIPAA
  (e.g. internal tooling, dev environments).
- Customers want a documented JSON dump they can
  parse themselves.

## Scope

NEW Generic export generator in
`argo-compliance`:
- Function `generate_generic_export(tenant_id, from,
  to, filters?) -> Result<Bundle>`.
- Pulls all audit_record rows in scope (no PHI
  filter, no GDPR-user filter).
- Emits 2 paired files:
  - `bundle.json`: vendor-neutral JSON dump of
    all rows.
  - `bundle.pdf`: human-readable PDF summary (1
    page of stats + 5 pages of sampled rows).
- Both signed via the same Ed25519 chain.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `crates/argo-compliance/` (extend
  pattern from J17 and SOC2/GDPR).

## Files you may touch

- `crates/argo-compliance/src/generic.rs` — NEW.
  Generic export generator. ≤ 350 LOC.
- `crates/argo-compliance/src/pdf.rs` — NEW. PDF
  renderer using `printpdf`. ≤ 300 LOC.
- `crates/argo-compliance/src/generic_tests.rs` —
  NEW. ≥ 8 tests. ≤ 350 LOC.
- `crates/argo-compliance/src/lib.rs` — narrowly
  scoped: register the new module + add
  `Format::Generic` variant + dispatch arm.
  ≤ 300 LOC after.
- `crates/argo-compliance/Cargo.toml` — narrowly:
  add 1 dep `printpdf`.

## Files you must NOT touch

- Other Track-J handler files.
- The existing SOC2 / GDPR / HIPAA generators.
- `crates/argo-audit/**` — read-only.

## Acceptance criteria

- [ ] `generate_generic_export(tenant, from, to,
      filters?)` returns a Bundle with:
      - `format: "generic-json-pdf"`
      - `manifest.json`: tenant + window + count +
        hash chain anchor
      - `bundle.json`: vendor-neutral dump
      - `bundle.pdf`: 6-page summary
      - `signature.bin`: Ed25519 over the final
        hash
      - `README.md`: schema documentation
- [ ] `bundle.json` shape:
      ```json
      {
        "schema_version": "1.0",
        "tenant_id": "...",
        "window": { "from": "...", "to": "..." },
        "row_count": 1247,
        "rows": [
          { /* AuditRow JSON */ },
          ...
        ]
      }
      ```
- [ ] `bundle.pdf` content:
      - Page 1: Title + tenant + window + row count
        + signature digest + hash chain anchor.
      - Pages 2-6: Sampled rows (10 per page),
        rendered as a 5-column table: timestamp /
        worker / action type / status / approver.
- [ ] J2 redaction applied to all rows in the
      JSON dump (sentinels in place of raw PII).
- [ ] Bundle verifiable by `@skycot/verify`
      (E5-f CLI).
- [ ] Wired into E5-e's exporter; selecting
      "Generic JSON + PDF" produces the bundle.
- [ ] Empty window returns a valid bundle (zero
      rows + a "no rows in window" note in
      README).
- [ ] 8 unit tests pass.
- [ ] `cargo build -p argo-compliance` clean.
- [ ] `cargo clippy -p argo-compliance --all-targets
      -- -D warnings` clean.
- [ ] `cargo test -p argo-compliance` passes.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J18-compliance-generic-export`
- PR title: `compliance: generic JSON + PDF export (J18)`

## Notes for the implementer

- **`printpdf` dep choice.** Pure-Rust PDF
  generation. Lightweight; no headless-browser
  dep. The library handles basic text + tables;
  we don't need advanced typography for an
  audit summary.

- **PDF layout.**
  ```
  Page 1: Title page
  ┌────────────────────────────────────────┐
  │  SKYCOT Generic Export                 │
  │                                        │
  │  Tenant: {tenant_id}                   │
  │  Window: {from} to {to}                │
  │  Total rows: {count}                   │
  │                                        │
  │  Hash chain anchor: {hash}             │
  │  Signature digest:  {sig}              │
  │                                        │
  │  Verify integrity:                     │
  │    npm install -g @skycot/verify       │
  │    skycot-verify ./bundle.zip          │
  └────────────────────────────────────────┘

  Pages 2-6: Sampled rows
  ┌──────────┬─────────┬─────────────┬───────────┬──────────┐
  │ Time     │ Worker  │ Action      │ Status    │ Approver │
  ├──────────┼─────────┼─────────────┼───────────┼──────────┤
  │ 14:32    │ Mariah  │ reply_sent  │ approved  │ Sam      │
  │ ...                                                      │
  └──────────────────────────────────────────────────────────┘
  ```

- **Sampling strategy.** When > 50 rows, sample
  evenly across the window (first row of every
  N/50 bucket). Otherwise include all. Note in
  README which sampling was used.

- **JSON dump.** Full dump (not sampled). The
  PDF is the human-readable preview; the JSON is
  the machine-readable canonical artifact.

- **README content.**
  ```markdown
  # SKYCOT Generic Export

  Vendor-neutral export of audit records for
  tenant {tenant_id} covering {from} to {to}.

  ## Files

  - `bundle.json` — Complete machine-readable JSON
    dump of all audit records in the window.
    Schema version 1.0 documented at
    https://docs.skycot.com/schema/audit-row-v1.
  - `bundle.pdf` — Human-readable summary with
    sampled rows.
  - `manifest.json` — Bundle metadata + hash chain
    anchor.
  - `signature.bin` — Ed25519 signature over the
    final hash.

  ## Verification

      npm install -g @skycot/verify
      skycot-verify ./bundle.zip

  Or check the signature manually with the SKYCOT
  public key at https://docs.skycot.com/keys.
  ```

- **Performance.** For tenants with > 100K rows
  in the window, generation may take > 30s. The
  E5-e exporter routes large bundles through the
  queued-with-email-delivery flow.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-export-custom-templates`
  (already named E5-e).
- `mission-enterprise-export-row-rbac` — per-row
  RBAC filtering on the dump.

## TRACK J AUTHOR PHASE COMPLETE after this commits

- J1 (approvals JSON API)
- J2 (approvals PII server fence)
- J3 (inbox WS structured events)
- J4 (Genesis classify + draft-worker)
- J5 (workers hire)
- J6 (team list)
- J7 (worker detail)
- J8 (worker overview)
- J9 (worker activity)
- J10 (worker memory CRUD)
- J11 (worker settings + danger zone)
- J12 (retrain endpoints)
- J13 (duty endpoints)
- J14 (audit feed)
- J15 (audit detail + screenshots)
- J16 (elevated-view-request + grants)
- J17 (HIPAA disclosure log)
- J18 (Generic export)

= 18 backend briefs. Track J fully authored. Next:
conductor advisory for G + J (combined), then end the
loop.
