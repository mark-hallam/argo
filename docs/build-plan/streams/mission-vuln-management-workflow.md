# Mission load-bearing wire — Vulnerability management workflow

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 #218. Enterprises need a documented process for triaging vulnerabilities (CVEs in deps, security advisories, RustSec audit findings). Today: ad-hoc. Define the workflow + tooling.

## Scope

Two artifacts:
1. **Workflow doc** `docs/skycot/security/vuln-management.md` (NEW, ≤ 450 LOC) — SEV 1-4 vulnerability taxonomy, triage SLAs (P0: 4h, P1: 24h, P2: 7d, P3: 30d), advisory sources (RustSec, GitHub Security Advisories, NVD, vendor-specific), mitigation patterns, disclosure policy, named owners.
2. **NEW `argo security audit` CLI** that runs `cargo audit` + `cargo deny check advisories` + parses results into a structured JSON envelope: `{ vulnerabilities: [{advisory_id, severity, affected_crate, fixed_in, status: unmitigated|mitigated|wontfix, mitigation_note?}], total_count_by_severity }`. Operator-tunable `--severity-floor` filter.

This is NOT in scope: actual CVE scanning of binaries (that's container-scan territory — separate brief); web-CVE scanning (handled by GitHub's Dependabot in repo); secret-scanning (separate brief).

## Files you may touch

- `docs/skycot/security/vuln-management.md` (NEW, ≤ 450 LOC) — workflow doc
- `crates/argo-cli/src/handlers/security_audit.rs` (NEW, ≤ 350 LOC) — CLI handler
- `crates/argo-cli/src/handlers/security_audit_tests.rs` (NEW, sibling, ≤ 250 LOC) — 5 tests
- `crates/argo-cli/src/cli_security_cmds.rs` — `Audit { ... }` variant (verify file exists)
- `docs/skycot/RUNBOOK.md` — append "Vuln management" subsection cross-referencing the workflow doc

## Files you must NOT touch

- `Cargo.toml` (workspace) deny / audit configs already exist; the CLI handler shells out to existing `cargo audit` + `cargo deny`

## Acceptance criteria

- [ ] Workflow doc covers SEV 1-4 taxonomy + SLAs + advisory sources + named owners + disclosure policy
- [ ] `argo security audit` invokes existing tooling + parses JSON output
- [ ] `--severity-floor` filter works
- [ ] Exit codes: 0 = no unmitigated above floor, 1 = unmitigated found, 2 = tooling error
- [ ] 5 unit tests on the JSON-parsing logic (use embedded fixtures from `cargo audit --json` output)
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vuln-management-workflow`
- PR title: `security: vuln-management workflow + audit CLI (mission-vuln-management-workflow)`

## Notes

- This brief is process-heavy by design; the CLI is a thin parser around existing tools. The workflow doc is the load-bearing deliverable for SOC2 / ISO27001 evidence.
- Document the named-owner pattern: every CVE class has a default owner (security@ for P0/P1; eng-leads@ for P2/P3) operators override per their org chart.
- Pair with the cron-driven `audit-key-age` from the rotation brief — both produce machine-readable JSON consumable by ops dashboards.
