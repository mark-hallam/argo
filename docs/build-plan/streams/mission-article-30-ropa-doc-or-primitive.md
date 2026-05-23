# mission-article-30-ropa-doc-or-primitive

**Authored against:** `origin/main` 14f44211d on 2026-05-24.
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-article-30-ropa-doc-or-primitive`.

## Why this brief exists

Surfaced as broken-wire from PR #2125 (`eval-compliance-claims-substrate`, MERGED 2026-05-23). The marketing copy names "GDPR Article 30 Records of Processing Activities (ROPA)" as a supported substrate, but no ROPA primitive / endpoint / export exists in the codebase. The audit's finding was: *"GDPR Article 30 named without ROPA — either ship a substrate, or soften the marketing copy."*

This brief gives the worker the choice of two paths:

- **Path A (substrate)**: ship a minimal ROPA primitive — a `routes_compliance_ropa` handler that emits a JSON document conforming to Article 30's required fields (controller, processor, purposes, categories of data subjects, categories of personal data, recipients, third-country transfers, retention periods, technical/organizational measures). Wire it through `argo-cli` compliance handlers (sibling to `compliance_soc2.rs` / `compliance_export.rs`).
- **Path B (soften)**: remove "Article 30" + "ROPA" from marketing surfaces. Replace with general GDPR controls language matching the truth-source on `/security`.

Path B is the lower-cost shortcut and matches the operator's wave-3 trust-over-claim-soften pattern (sibling: `web-soc2-attestation-strip-soften` MERGED #2154, `web-customers-content-hipaa-soften` QUEUED).

Path A is the higher-substantive-value option — it gives demo enterprises an actual ROPA export, which is a real GDPR sales-cycle artifact.

## Acceptance criteria

Worker chooses Path A or Path B at claim time and documents the choice in PR body. Either path satisfies acceptance.

### Path A acceptance

1. New handler `crates/argo-cli/src/handlers/compliance_ropa.rs` (~150-250 LOC) emitting Article 30 JSON shape.
2. CLI subcommand `argo compliance ropa --tenant <id> --output <path>` registered in the dispatch.
3. RBAC capability gate (sibling to `compliance_soc2`).
4. Audit row `compliance.ropa.exported` carrying `payload[tenant_id, exported_by, output_path_hash]`.
5. 1 integration test pinning the wire shape + cross-tenant negative.
6. `pnpm typecheck` clean.

### Path B acceptance

1. Grep `web/app/` for "Article 30" + "ROPA" + "Records of Processing" references.
2. Replace with the operator-locked softened form (e.g. "GDPR controls in place — full ROPA available on request" or "GDPR data-processing transparency").
3. Confirm alignment with `/security` page truth-source.
4. `pnpm typecheck` + `pnpm lint` clean.

## Parallel-surface declarations

- Path A: `compliance_soc2.rs` / `compliance_export.rs` already exist as the canonical pattern. No new `pub struct` types — reuse `argo-audit::AuditKind` (add `ComplianceRopaExported` variant if absent, per HARD RULE 5 exhaustive-match allowance).
- Path B: no new code, copy change only.

## May-touch

### Path A
- `crates/argo-cli/src/handlers/compliance_ropa.rs` — new.
- `crates/argo-cli/src/handlers/mod.rs` or dispatch table — register.
- `crates/argo-audit/src/event_kind.rs` + `event.rs` + `event_tests.rs` — if variant absent.
- `crates/argo-cli/tests/compliance_ropa_test.rs` — new.

### Path B
- `web/app/landing/page.tsx` (or wherever the trust strip lives)
- `web/app/(marketing)/...` per-vertical or compliance-detail pages
- Snapshot tests if present

## Boundary

- Either path, NOT both — don't half-implement a primitive and half-soften the copy.
- Path A: do NOT bolt this onto the existing `compliance_export` bundle — ROPA is a structured document, not an audit-event export.
- Path B: do NOT remove all GDPR references — GDPR controls ARE in place; the over-claim is the specific "Article 30 ROPA" naming.

## Sizing

- Path A: ~150-250 LOC + tests.
- Path B: ~10-30 LOC copy change.

Worker should pick Path B unless they have time + appetite for the substrate work.

**🟡 P2 (TRUST OVER-CLAIM OR SUBSTRATE)** — operator-locked-soften register from 2026-05-20 wave-3 + PR #2125 finding (deferred residual).
