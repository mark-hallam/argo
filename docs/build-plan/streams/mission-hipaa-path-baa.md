# Mission load-bearing wire — HIPAA-compatible deployment path (BAA-ready)

**Owner**: claude. Design + scoping brief, partial implementation.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #212. Healthcare tenants need HIPAA-compatible deployment. BAA-ready means: (a) PHI flows audited, (b) at-rest encryption, (c) access-logged transmission, (d) breach-notification workflow. Several pieces ship separately; this brief is the design + the gaps.

## Scope

Two artifacts:

1. **Design doc** `docs/skycot/security/hipaa-deployment.md` (≤ 500 LOC) covering:
   - HIPAA Security Rule technical safeguards mapped to SKYCOT features (access control, audit controls, integrity controls, transmission security)
   - What ships TODAY (audit chain, PII redaction, healthcare-intake PHI fence)
   - What needs to ship: BAA-compatible HSP list (when SKYCOT runs in cloud — only certain providers offer BAAs)
   - Breach-notification workflow (60-day rule); operator-facing playbook
   - Disclosure log requirement (J17 brief — separate)
   - Recommendation: only deploy SKYCOT for PHI when (a) `[security.hipaa] enabled = true` is set, (b) the deployment passes a documented checklist, (c) the operator has executed BAAs with their LLM provider

2. **Enforcement scaffold** — `[security.hipaa]` config section + a `hipaa_compliance_check` audit-row event emitted at gateway startup describing the deployment's compliance posture (encryption-at-rest active? audit-chain enabled? known PHI fences wired?). Operators check this row as their compliance evidence.

This is NOT in scope: BAA paperwork (operator-side); LLM-provider HIPAA review (operator picks compliant providers); full audit by a third-party assessor (different workstream).

## Files you may touch

- `docs/skycot/security/hipaa-deployment.md` (NEW, ≤ 500 LOC)
- `crates/argo-cli/src/handlers/gateway/hipaa_startup_check.rs` (NEW, ≤ 280 LOC) — startup audit event
- `crates/argo-cli/src/handlers/gateway/hipaa_startup_check_tests.rs` (NEW, sibling, ≤ 200 LOC) — 5 tests
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: invoke startup check
- `argo-config/src/sections.rs` — `[security.hipaa]` section
- `docs/skycot/RUNBOOK.md` — "HIPAA deployment" subsection cross-referencing the design doc

## Files you must NOT touch

- The PII redaction code (orthogonal); the audit chain logic; tenant-side BAA paperwork

## Acceptance criteria

- [ ] Design doc covers Security Rule mapping + today-vs-needed + named follow-up briefs
- [ ] Startup check emits `hipaa_compliance_check` audit row listing the deployment's posture (8-10 named features with status)
- [ ] `[security.hipaa] enabled = false` (default) — startup check still runs but in "not-claimed" mode
- [ ] 5 unit tests
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-hipaa-path-baa`
- PR title: `security: HIPAA deployment design + startup compliance check (mission-hipaa-path-baa)`

## Notes

- Don't oversell. The design doc explicitly says "this gets you to BAA-ready; full HIPAA compliance also requires operator-side controls". We provide technical primitives; operators apply them.
- Startup check is the load-bearing evidence trail — auditors want to see "this deployment claimed HIPAA mode on date X with features Y, Z, W". The audit row is that evidence.
- Pair with the SCIM brief (provisioning for healthcare staff), SSO brief (no shared accounts), encryption-at-rest briefs, audit-chain (already shipped). HIPAA is the integration test of the security stack.
