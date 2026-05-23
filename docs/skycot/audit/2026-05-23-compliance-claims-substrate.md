# Compliance-claims substrate audit (HIPAA / GDPR / SOC 2)

**Brief**: `docs/build-plan/streams/eval-compliance-claims-substrate.md`
**Audited at**: origin/main `a3e7ae55a` on 2026-05-23
**Author**: claude worker
**Verdict**: 🟡 **AUDITED — substrate is mostly real but four claim
classes overshoot.** Substrate IS shipped for: per-tenant cascade
export (Article 15), tenant cascade-delete + audit tombstoning
(Article 17), per-user data-export job + 1-per-24h rate limit, SOC 2
audit-trail report (Common Criteria 6.x), PHI / PII redaction fence
(`argo-redact`), HIPAA deployment-posture breadcrumb at gateway boot,
and the hash-chained tamper-evident audit log. Over-claims that need
copy or wire fixes: (a) the SPA's four compliance-shaped export
format strings don't parse on the gateway; (b) "SOC 2 Type II — Audit
on request" badge on the marketing landing strip implies an
attestation that does not yet exist; (c) the Beacon customer
testimonial says "HIPAA-compliant out of the box"; (d) GDPR Article
30 (records-of-processing) is named in onboarding copy but has no
platform-owned ROPA primitive.

## Gap-manifest row cited verbatim

From `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
line 93:

> | `argo-compliance` | ? | 🔴 NOT AUDITED | HIPAA + GDPR. Per pricing page: "HIPAA workflows" claim. Need to verify the substrate. | /pricing claim | 🟡 P2 | `eval-compliance-claims-substrate` — for each compliance claim on /pricing (HIPAA workflows, GDPR Article 30, SOC 2 Type II audit trail), find the substrate that backs it. |

Master gap-list row 28 references the same brief.

## Claim-vs-substrate matrix

Legend:
- 🟢 = claim is backed by shipped substrate, faithfully scoped.
- 🟡 = substrate exists but the claim is broader / softer / wider than what ships, or substrate is partial.
- 🔴 = claim has no substrate — pure marketing language.

### SOC 2

| # | Marketing claim | Where it appears | Substrate found | Verdict |
|---|---|---|---|---|
| 1 | "SOC 2-ready audit trail" | `web/app/pricing/page.tsx:99`, `web/app/security/page.tsx:83`, `web/app/landing/page.tsx:165`, `web/app/(setup)/onboarding/welcome/page.tsx:40` ("SOC 2-ready from day one") | Hash-chained `audit_record` table — `crates/argo-audit/src/sink_tx.rs:103-118` (`parent_hash` / `self_hash` per row, computed inside the insert tx); `argo-audit::verify_session` / `verify_all` — `crates/argo-audit/src/verify.rs:55-77`; CLI surface `argo audit verify` — `crates/argo-cli/src/handlers/audit_verify.rs:1-30`. | 🟢 |
| 2 | `argo compliance soc2` Common Criteria 6.x audit-trail report (Access / ChangeMgmt / DataMgmt / Security / Other) — categorises events in `[--from, --to)` for external-auditor sampling. | `web/components/skycot/audit/ExportFormatSelector.tsx:12-14` (UI surfaces a "SOC2 Type II" bundle option); pricing-tier copy | `crates/argo-compliance/src/soc2.rs:14-65` — `Soc2Category` enum + `ACTION_CATEGORY` static map (12 known audit kinds bucketed into 4 categories, unknown → `Other`); `build_report` returns category aggregates + per-event rows in JSON / CSV / Markdown. Wired in CLI dispatcher at `crates/argo-cli/src/main_dispatch_audit.rs:27`. | 🟢 |
| 3 | "SOC 2 Type II — Audit on request" trust-strip badge on landing. | `marketing/components/landing/ComplianceStrip.tsx:26-28`. | No SOC 2 Type II report exists. `web/app/security/page.tsx:84-85` (same-codebase truth) says: *"External SOC 2 Type II attestation is in progress."* The deployment-posture audit row at boot reports the readiness checklist, not an attestation. "Audit on request" in industry standard implies a Type II report under NDA — there is none to send. | 🔴 over-claim |
| 4 | "SOC 2 TYPE II" trust-strip pill on every vertical landing | `web/app/for/_components/VerticalLandingShell.tsx:627` | Same gap as #3. The 2026-05-20 wave-3 copy audit (`docs/skycot/audit/copy/2026-05-20-wave-3-copy-voice.md:158`) recommended `SOC 2-READY AUDIT TRAIL` here; the rewrite landed on `/landing` and `/pricing` but the per-vertical landing still ships the unsoftened pill. | 🔴 over-claim (residual after wave-3 copy pass) |
| 5 | Evidence-bundle `format="soc2-type-ii"` selector in `/audit` produces "SOC2 Type II evidence bundle (JSON + PDF + hash chain) suitable for attestation sampling." | `web/lib/sample-export.ts:34-43`; `web/components/skycot/audit/ExportFormatSelector.tsx` | Gateway `POST /api/v1/audit/export` accepts ONLY `json | csv | tar` per `crates/argo-cli/src/handlers/gateway/routes_audit_export.rs:224-231` (`parse_format`). The four SPA format strings (`soc2-type-ii`, `gdpr-article-15`, `hipaa-disclosure-log`, `generic`) are not recognised; against production gateway the POST returns 400 `bad_request: unknown format 'soc2-type-ii' (allowed: json | csv | tar)`. The SPA's `web/lib/api/audit-export.ts` falls back to `mockGenerate` on 404, NOT on 400 (`isMissingOrUnreachable` only matches `ApiError.status === 404`), so the dev/Playwright path lands on the gateway error envelope. | ✅ RESOLVED — `web-audit-export-format-strings-fix` ships client-side mapping `complianceFormatToBundleFormat()` in `web/lib/api/audit-export.ts`; all four compliance shapes now POST `format="tar"` to the gateway. Pin test in `web/lib/api/audit-export.test.ts`. |

### GDPR

| # | Marketing claim | Where it appears | Substrate found | Verdict |
|---|---|---|---|---|
| 6 | "GDPR workflow tools" | `web/app/pricing/page.tsx:100` | `argo compliance export` (Article 15 cascade-export by tenant) — `crates/argo-compliance/src/export.rs:1-76` + `tenant_tables.rs:21-70` (19 tenant-scoped tables walked via three scope dimensions: BLOB `tenant_id`, TEXT `tenant_id`, session-FK). `argo compliance delete` (Article 17 cascade-delete + audit tombstoning inside one tx) — `crates/argo-compliance/src/delete.rs:1-121`. Self-audit emission via `compliance.export` / `compliance.delete` audit kinds (legacy strings; NOT in `AuditKind` enum). | 🟢 |
| 7 | "GDPR Article 15 / CCPA right-to-know — one export every 24 hours" | `web/app/(app)/settings/data-export/page.tsx:44-48` | `POST /api/v1/me/export` — `crates/argo-cli/src/handlers/gateway/routes_me_export.rs:52-105`. Rate-limit constant `RATE_LIMIT_WINDOW_SECS: i64 = 24 * 60 * 60` and download TTL `DOWNLOAD_TTL_SECS: i64 = 7 * 24 * 60 * 60` at `crates/argo-store/src/data_export_job_query.rs:37-40`. `data_export_job` schema (migration 116). Authenticated `X-Argo-Principal: human:<email>` required. Audit kind `AuditKind::DataExportRequested` emitted per accepted request. | 🟢 |
| 8 | "GDPR Article 17 right-to-erasure" — `argo gdpr delete --subject <email>` | `crates/argo-cli/src/handlers/gdpr_delete.rs:1-100` | Per-subject (email-keyed) erasure — `gdpr_redact_payload` rewrites PII fields in `payload_json` across `audit_record` rows; non-audit table rows are deleted via `gdpr_apply_non_audit`. Audit hash-chain is reconciled by writing replacement rows in-place rather than deleting (preserves `verify_chain`). Confirmation: `--confirm <email>` (typed prompt on stdin), `--bundle-out` requires `--confirm`. | 🟢 |
| 9 | "GDPR Article 30 trails" — onboarding "Enterprise-grade" pillar | `web/app/(setup)/onboarding/welcome/page.tsx:52` | **No Article-30 substrate.** Article 30 is "Records of processing activities" — a documented register of what data the controller processes, why, recipients, retention, transfers. SKYCOT has the **audit substrate that supports** a controller's ROPA (every action is logged with kind / actor / tenant), but does not ship a ROPA primitive (no table or doc-generator that emits the article-30 form). The marketing phrase "Article 30 trails" is non-standard — it conflates the audit-log primitive with the ROPA register. | 🟡 partial — substrate supports it but no first-class primitive |
| 10 | `argo audit export --gdpr-redact-emails` operator CLI flag | `crates/argo-cli/src/handlers/audit_export.rs:50-51` + `audit_export_redact.rs:1-6` | Working flag; rewrites email-shaped strings to sha256-hashed tokens in payloads at export time. Independent from the `argo-redact` runtime fence. | 🟢 |
| 11 | "GDPR Article 15 access export (redaction-aware)" — SPA evidence-bundle format option | `web/lib/sample-export.ts:36, 44-45` | Same broken-wire as #5 — the `format="gdpr-article-15"` string does not parse on the gateway. The per-tenant Article-15 surface IS shipped via `argo compliance export` (CLI) and the per-user Article-15 surface IS shipped via `POST /api/v1/me/export`. The evidence-bundle dialog is a *third* surface that doesn't map cleanly to either. | ✅ RESOLVED via `web-audit-export-format-strings-fix` — see row #5. The compliance label remains cosmetic; the underlying `tar` envelope is what actually ships. |
| 12 | DPA template references Article 28 (controller-processor) | `marketing/content/legal/dpa-template.md:2-14` | Operator-authored template, not platform substrate. Disclaims itself as "reasonable boilerplate"; not a software claim. | 🟢 (no claim that SKYCOT itself executes Article-28 obligations) |

### HIPAA

| # | Marketing claim | Where it appears | Substrate found | Verdict |
|---|---|---|---|---|
| 13 | "HIPAA-aligned redaction" — `/pricing` Pro+Enterprise feature row | `web/app/pricing/page.tsx:101` | `argo-redact` crate — `crates/argo-redact/src/lib.rs:1-46`. Five regex-driven PII categories (email / phone / Luhn-valid card / IBAN / NZ IRD / US-shape address) + per-category override via `ViewerPermissions` and per-row `GrantSet`. Wired in 5+ gateway handlers — `routes_workers_hire.rs:308-405`, `routes_approvals_json.rs:36-104`, `routes_worker_activity.rs:99-203`, `routes_audit_feed.rs:41-49`, etc. Sentinel set (`[EMAIL]`, `[PHONE]`, `[CARD]`, `[IBAN]`, `[IRD]`, `[ADDR]`) is byte-aligned with the client-side fence (E1-e). | 🟢 (with note: claim is "aligned" not "compliant" — accurate hedge) |
| 14 | "HIPAA workflows" — operator can run healthcare flows | `docs/verticals/healthcare-intake.md:78` ("HIPAA-compliant audit sink"); `docs/skycot/security/hipaa-deployment.md` (the BAA-ready deployment guide) | Gateway-boot deployment-posture audit row — `crates/argo-cli/src/handlers/gateway/hipaa_startup_check.rs:13-58`. Reads `[security.hipaa] enabled = true` from `argo.toml`, emits a `hipaa_compliance_check` audit row at boot with 9 feature checks (access_control / audit_controls / integrity_controls / transmission_security / at_rest_encryption / phi_fence / disclosure_log / breach_notification / baa_provider_review). Default mode is `not_claimed`. Disclosure-log feature reports `follow_up_required` (J17 brief not yet shipped — flagged below as fix-brief). | 🟡 — five of nine HIPAA-safeguard cells require operator action; disclosure_log substrate not yet implemented |
| 15 | "SOC 2 Type II, HIPAA, ISO 27001, AFSL — on day one" | `docs/skycot/operator/demo-runbook.md:113` (operator demo script) | No SOC 2 Type II attestation (see #3). No ISO 27001 cert. HIPAA is a deployment posture, not a software cert. AFSL is an operator licence, not a software-attestation primitive. The 2026-05-20 wave-3 copy audit (`docs/skycot/audit/copy/2026-05-20-wave-3-copy-voice.md:147-152`) flagged this exact register as a highest-severity overclaim. | 🔴 over-claim |
| 16 | "HIPAA-compliant out of the box" — Beacon Health Ops customer card | `web/lib/content/customers.ts:111-112` | Fiction (sample-content quote). The platform supplies BAA-ready substrate; "compliant" requires the BAA + workforce policy + provider-chain BAAs that the platform cannot self-attest. Wave-3 copy audit (`...copy-voice.md:165`) recommended the rewrite `"HIPAA-workflow-ready out of the box."` — not yet applied. | 🔴 over-claim (residual) |
| 17 | "HIPAA disclosure log" SPA export format | `web/lib/sample-export.ts:37, 46-47`; `web/components/skycot/audit/ExportFormatSelector.tsx:12-14` | Same broken-wire as #5. The `hipaa_compliance_check` posture row at boot reports `disclosure_log` status as `follow_up_required` with note "J17 disclosure-log brief remains separate" — meaning there is no shipped HIPAA disclosure-log generator anywhere. The SPA exposes the option as if it works. | ✅ RESOLVED (wire only) via `web-audit-export-format-strings-fix` — `hipaa-disclosure-log` now POSTs as `tar`, no 400. The deeper substrate gap (no actual disclosure-log generator) remains tracked under `mission-hipaa-disclosure-log-substrate`. |
| 18 | "GDPR · HIPAA · AFSL" trust-strip pill on every vertical landing | `web/app/for/_components/VerticalLandingShell.tsx:631` | Same residual as #4 — wave-3 copy audit recommended `GDPR · HIPAA WORKFLOW-READY` and dropping AFSL; not yet applied on `_components/VerticalLandingShell.tsx`. | 🔴 over-claim (residual) |

### Cross-cutting substrate (shared across all three claim families)

| # | Substrate | What's shipped | What it backs |
|---|---|---|---|
| 19 | Hash-chained audit log | `crates/argo-audit/src/sink_tx.rs:103-118` — per-row `parent_hash` / `self_hash`, recomputed inside the write transaction. Per-session and per-global chains. 60+ `AuditKind` variants — `crates/argo-audit/src/event_kind.rs:17-447`. | SOC 2 audit-trail evidence (CC6.x), HIPAA `164.312(b)` audit controls, GDPR Article 30 process-of-record. |
| 20 | Chain-verify | `argo audit verify` / `verify-chain` — `crates/argo-cli/src/handlers/audit_verify_chain.rs:1-30`, `crates/argo-audit/src/verify.rs:55-77`. Walks every session chain; reports any broken link. | Tamper-evident integrity required by SOC 2 CC4 / HIPAA `164.312(c)(1)`. |
| 21 | Capability-based RBAC | `argo-capability` per-role permission engine. Role gate at gateway — `crates/argo-cli/src/handlers/gateway/role_gate.rs:85-87`. | SOC 2 CC6.1 access control; HIPAA `164.308(a)(4)` access management. |
| 22 | At-rest encryption | SQLCipher-ready store path; documented in `docs/skycot/security/at-rest-encryption.md`. The deployment-posture row at boot reports `at_rest_encryption: operator_required`. | HIPAA `164.312(a)(2)(iv)` / `164.312(e)(2)(ii)` — operator-owned. |
| 23 | PII redaction fence | `argo-redact` — see #13. | HIPAA `164.514` de-identification (partial; structural fence, not Safe Harbor / Expert Determination). |
| 24 | Per-tenant scoping | Every tenant-scoped table has `tenant_id` and every write/read binds it (round-12 / round-13 / `eval-argo-store-tenant-bind-sweep` audited 2026-05-23). | SOC 2 CC6.6 / HIPAA `164.308(a)(3)` workforce-clearance scoping. |
| 25 | TLS termination | Operator-required (reverse proxy or future native TLS) — documented in `docs/skycot/security/hipaa-deployment.md:48-50`. Gateway-boot posture row reports `transmission_security: operator_required`. | HIPAA `164.312(e)(1)` transmission security. |

## Summary verdict per claim family

- **SOC 2**: substrate is real (#1, #2, #20) and faithfully described where it says "SOC 2-ready audit trail". One "Type II attestation" trust-strip badge on the marketing landing strip (#3) plus the residual "SOC 2 TYPE II" pill on vertical-landing shells (#4) plus the broken `format="soc2-type-ii"` SPA option (#5) overshoot the substrate.
- **GDPR**: Article 15 (per-tenant + per-user), Article 17 (CLI cascade-delete + audit tombstoning), and Article 6/32-grade audit-trail substrate are all real and tested. Article 30 ("records of processing activities") is named in onboarding copy (#9) without a first-class ROPA primitive — the audit chain is the *raw material* for the controller's ROPA, not the ROPA itself. One broken-wire SPA option (#11).
- **HIPAA**: substrate is "BAA-ready", not "HIPAA-compliant". The deployment-posture audit row (`hipaa_compliance_check`) is the platform's honest self-report and correctly flags 5 of 9 safeguards as operator-required (#14). The Beacon customer card (#16) and the demo-runbook line (#15) and the per-vertical landing pill (#18) and the SPA's `hipaa-disclosure-log` format (#17) overshoot.

## Findings → fix-briefs (queue-replenishment for the conductor)

These rows belong on the gap manifest under their respective mission claims. Conductor: pick whichever are highest-leverage.

1. **`web-audit-export-format-strings-fix`** (P1, ~80 LOC). Two pick-one resolution paths for finding #5 / #11 / #17:
   - (a) **Server-side**: extend `BundleFormat` in `crates/argo-audit/src/export_bundle.rs:128-148` with the four compliance-shaped variants and document the format-to-bundle-shape mapping; OR
   - (b) **Client-side**: map the SPA's `ExportFormat` to one of `{json, csv, tar}` before POST in `web/lib/api/audit-export.ts:67-83` and use the compliance-name as cosmetic metadata only. Either pin a FINDINGS-ONLY test in `crates/argo-cli/src/handlers/gateway/routes_audit_export_tests.rs` that asserts the 4 SPA-shaped format strings round-trip cleanly through the gateway (currently they return 400). Recommendation: (b) is the smaller blast-radius fix; the compliance-name is more accurate as a label than as a server-binding selector.
2. **`web-soc2-attestation-strip-soften`** (P2, ~10 LOC). Replace `name: "SOC 2 Type II"` / `detail: "Audit on request"` in `marketing/components/landing/ComplianceStrip.tsx:26-28` with `name: "SOC 2-ready audit trail"` / `detail: "Hash-chained log"` until a Type II report exists. Same fix on `web/app/for/_components/VerticalLandingShell.tsx:627, 631` (`SOC 2 TYPE II` → `SOC 2-READY AUDIT TRAIL`; drop `· AFSL` token).
3. **`web-customers-content-hipaa-soften`** (P2, ~5 LOC). Replace the Beacon testimonial line at `web/lib/content/customers.ts:112` per wave-3 copy audit recommendation `"HIPAA-workflow-ready out of the box."` Adjust the Postgres reference at line 121 (SKYCOT is SQLite) → "audit-log table".
4. **`mission-hipaa-disclosure-log-substrate`** (P2, ~150-300 LOC). The deployment-posture row's `disclosure_log: follow_up_required` is the J17 brief that has not landed. Either build the substrate (an `audit_record`-scoped `hipaa.disclosure` kind + sampling query + report renderer) OR remove the SPA's `hipaa-disclosure-log` selector option until the substrate ships.
5. **`mission-article-30-ropa-doc-or-primitive`** (P3, ~50 LOC + doc). Either ship a `argo compliance ropa` subcommand that renders the audit-derived Article-30 form (controller / categories / recipients / retention), OR rewrite the onboarding pillar copy at `web/app/(setup)/onboarding/welcome/page.tsx:52` from "GDPR Article 30 trails" to the more accurate "GDPR-compliant audit chain" — the audit chain IS the substrate; Article 30 is a controller-side document, not a platform feature.
6. **`mission-demo-runbook-trust-language-soften`** (P3, ~5 LOC). Rewrite `docs/skycot/operator/demo-runbook.md:113` to drop the "SOC 2, ISO 27001, GDPR, HIPAA, AFSL on day one" register; this surface is internal but it leaks (operators recite the demo script live).

## Footnote on the `compliance.export` / `compliance.delete` audit kinds

The `argo-compliance` crate writes audit rows with kind = `"compliance.export"` (`crates/argo-compliance/src/export.rs:23`) and `"compliance.delete"` (`crates/argo-compliance/src/delete.rs:27`). Neither string is a variant of `argo_audit::AuditKind` (see `event_kind.rs:17-447`); they bypass the typed surface and are written via raw `INSERT INTO audit_record` SQL with a `compute_chain_hash` call (`export.rs` / `delete.rs:append_delete_audit`). This is benign for chain integrity (the hash inputs include the kind string regardless), but the typed `AuditSink::write` surface is the SOC 2 audit-trail report's classifier — the `soc2.rs:ACTION_CATEGORY` map binds `"compliance.export"` and `"compliance.delete"` as plain strings (lines 53-54), so the report still picks them up. Worth a follow-up brief at low priority: `mission-compliance-audit-kind-promote-to-enum` — promote the two strings to first-class `AuditKind` variants so the typed surface gains 100% coverage. Not load-bearing for v1.

## Test coverage on the substrate

- `argo-compliance`: 200+ tests pass — see `crates/argo-compliance/src/{delete_tests,export_tests,soc2_tests,tenant_tables}.rs`.
- `argo-audit`: 5820 LOC across 22 modules, end-to-end + per-module tests.
- `argo-redact`: 7+ tests, all passing; 381 LOC of `lib_tests.rs`.
- `routes_me_export`: integration tests at `crates/argo-cli/src/handlers/gateway/routes_me_export_tests.rs`.
- `hipaa_startup_check`: 4 tests in `hipaa_startup_check_tests.rs` covering `[security.hipaa]` toml read + audit-row append.

No cross-tenant leak surfaced in this audit pass. No `#[ignore]`'d FINDINGS-ONLY test required — the broken-wire findings are at the SPA→gateway protocol layer (not crate code), and the may-touch list for this brief does not include `crates/` or `web/`. The pin-tests are part of the fix-briefs above.

---

**End of report.**
