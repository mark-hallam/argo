# codex-cargo-deny-supply-chain-audit

**Authored against**: `origin/main` `652a1825` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cargo-deny-supply-chain-audit`.

**Fast-mode eligible**: yes — mechanical run + fix.

## Summary

The disabled GHA workflow `.github/workflows/deny.yml.disabled` runs `cargo-deny check` against the workspace. While the workflow is operator-disabled (per [DISABLED_WORKFLOWS_POLICY.md](.github/workflows/DISABLED_WORKFLOWS_POLICY.md)), the supply-chain hygiene matters for v1.0 launch. Codex runs `cargo-deny check` locally and addresses any new findings since the last clean run.

## Acceptance

- [ ] Install or verify `cargo-deny` is available locally.
- [ ] Run `cargo deny check` (all categories: advisories, bans, licenses, sources). Capture output to PR body.
- [ ] For each finding, classify:
  - **MITIGATED** — already addressed in `deny.toml` (existing exception). Leave alone.
  - **NEW-DUPLICATE-CRATE** — workspace has two versions of the same crate; fix via patch/dependency unification if ≤30 LOC, else flag.
  - **NEW-VULNERABILITY** — CVE flagged in `cargo-audit`; bump the dep if a fixed version exists, else spawn follow-up brief with details.
  - **NEW-LICENSE-VIOLATION** — newly-introduced dep has a license not on the allowlist. Either add to allowlist if license is acceptable (with operator justification) or replace the dep.
- [ ] Update `deny.toml` if new exceptions are warranted (with comment citing the audit chain).
- [ ] PR body includes before/after `cargo deny check` output excerpts.
- [ ] No production-code change unless a dep version bump requires API adjustment.

## May touch

- `deny.toml` (additive exceptions with comments)
- `Cargo.toml` / `Cargo.lock` (dep version bumps to address advisories)
- Workspace dep references in `crates/*/Cargo.toml` if a unification is required

## Must NOT touch

- Production code semantics. Dep bumps that require code changes spawn follow-up briefs.
- Re-enable the GHA workflow — operator-locked per policy.

## Notes

- Pre-flight: `cat .github/workflows/deny.yml.disabled` to see the configured categories.
- Pre-flight: `cat deny.toml` for current allowlist + exception state.
- Sizing: 100–400 LOC depending on findings. If `cargo deny check` is already clean, the brief closes as RESOLVED-NO-FINDINGS with the output excerpt in PR body.
- v1.0 launch hygiene — this is the kind of finding that auditors and procurement teams ask for.
