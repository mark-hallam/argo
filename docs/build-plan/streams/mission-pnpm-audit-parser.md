# mission-pnpm-audit-parser — extend `argo security audit` CLI with pnpm coverage

**Owner**: claude. **Depends on**: nothing (`argo security audit` CLI exists on main via #1458).
**Authored against**: origin/main `1e030e40` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pnpm-audit-parser`.

> **FOLLOW-UP 2026-05-14 by conductor (CEO-authorized after closing #1498 as STALE-DUPLICATE of #1458).** #1458 shipped the workflow doc + Rust-side parser (cargo-audit + cargo-deny). This brief adds the JS-side parser (pnpm audit) to the same CLI. ~80 LOC. Pure extension of the existing surface — drift-prevention rule #2 ("Grep before write") satisfied: extends `argo security audit` rather than creating a parallel surface.

## Scope

Extend `argo security audit` in `crates/argo-cli/src/handlers/security_audit/` to also call `pnpm audit --json` when a `web/package.json` (or `pnpm-lock.yaml`) is detected at repo root or under `web/`. Parse the JSON output, normalize advisories into the same envelope shape #1458 already emits for cargo-audit / cargo-deny, dedup by advisory id, merge into the single `vulnerabilities[]` array.

Envelope shape unchanged from #1458:

```json
{
  "vulnerabilities": [
    { "advisory_id": "GHSA-xxxx-yyyy-zzzz",
      "severity": "high",
      "affected_crate": "axios",
      "fixed_in": ">=1.7.4",
      "status": "unmitigated",
      "mitigation_note": null }
  ],
  "total_count_by_severity": { "critical": 0, "high": 1 }
}
```

If `pnpm audit` is not on PATH or no JS package found, silently skip — exit 0 if no Rust findings either. Tooling-error exit code `2` reserved for cargo-side malformed JSON (preserve #1458 contract).

**Not in scope**: changing the existing cargo-audit / cargo-deny parsing; introducing a new CLI surface; gating CI on JS findings (operator decides post-v1).

## Files you may touch

- `crates/argo-cli/src/handlers/security_audit/mod.rs` — narrowly: add `parse_pnpm_audit()` + call site. <=30 modified lines.
- NEW `crates/argo-cli/src/handlers/security_audit/pnpm.rs` — the parser. <=80 LOC.
- `crates/argo-cli/src/handlers/security_audit_tests.rs` — 2 new tests: empty pnpm output (no vulns) + populated pnpm-audit fixture (dedup + bucket counting confirmed).

## Files you must NOT touch

- `crates/argo-cli/src/handlers/security_audit/` cargo-side parsing — locked; reuse the envelope only.
- `docs/skycot/security/vuln-management.md` — locked; the workflow doc is canonical.
- Any pnpm config or `web/package.json` — read-only.

## Acceptance criteria

- [ ] `argo security audit` invoked at repo root detects `web/pnpm-lock.yaml` and shells out to `pnpm audit --json`.
- [ ] pnpm advisories normalized to the same envelope shape as cargo-audit / cargo-deny.
- [ ] Dedup by `advisory_id` preserves the existing cargo-side dedup; pnpm and cargo advisories merge into one `vulnerabilities[]`.
- [ ] If `pnpm` not on PATH OR no JS package file detected -> skip silently, no error.
- [ ] 2 new tests in `security_audit_tests.rs`.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli security_audit` passes (5 existing + 2 new).
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-mission-pnpm-audit-parser`
- PR title: `security: pnpm-audit parser extension to argo security audit (mission-pnpm-audit-parser)`

## Notes for the implementer

- ~80 LOC. Pure extension. ~30 min worker time.
- `pnpm audit --json` shape: array of advisories under `.advisories.*` keyed by advisory id; each entry has `severity`, `module_name`, `vulnerable_versions`, `patched_versions`, `recommendation`. Map to envelope as: `advisory_id` = the key, `severity` = lowercased value, `affected_crate` = `module_name`, `fixed_in` = `patched_versions`.
- Exit code 2 stays reserved for cargo-side tooling errors. pnpm tooling absence is silent skip (the JS ecosystem is optional for non-frontend deployments).
- This brief is the canonical pnpm-coverage path. Do NOT re-introduce the parallel surface that #1498 attempted (`mission-vulnerability-management-workflow`) — that PR was closed STALE-DUPLICATE on 2026-05-14.
