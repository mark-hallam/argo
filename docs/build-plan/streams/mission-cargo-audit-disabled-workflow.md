# Mission load-bearing wire — cargo-audit CI workflow (disabled-state, ready to enable)

**Owner**: claude. **Depends on**: existing CI workflow pattern at `.github/workflows/*.yml.disabled` (verified: 5 disabled workflows present); cargo-audit CLI (commonly-available; brief installs in workflow); workspace `Cargo.lock` (verified present).
**Authored against**: origin/main `f8f51df4` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cargo-audit-disabled-workflow`.

> SOURCE: MISSION_BACKLOG.md Section 1 #14 — "Add cargo-audit to CI (no current vulnerability scan)." All CI is currently `.disabled` per the ledger header (GitHub Actions billing not restored). This brief adds the cargo-audit job in matching `.disabled` state so operator can flip-on with a single `git mv` once billing is restored. ALSO ships a local-runnable script so vulnerability scanning is available now.

## Scope

1. **NEW `.github/workflows/audit.yml.disabled`**:
   - `on: schedule` (daily at 09:00 UTC) + `on: pull_request` (only when `**/Cargo.lock` changes).
   - Steps: checkout, `cargo install cargo-audit --locked` (cached), `cargo audit --deny warnings`.
   - Allowlist file at `.cargo-audit-ignore.toml` (NEW, empty stub — operator populates).
   - Format mirrors existing `ci.yml.disabled` (use `dtolnay/rust-toolchain@stable`, `runs-on: ubuntu-latest`).

2. **NEW local script** `scripts/cargo-audit.sh`:
   - Installs cargo-audit if missing (idempotent: `cargo install --list | grep -q cargo-audit || cargo install cargo-audit --locked`).
   - Runs `cargo audit` honoring `.cargo-audit-ignore.toml`.
   - Exits non-zero on findings.
   - Adds `--fix` passthrough flag (calls `cargo update` with the suggested upgrades — operator reviews diff).

3. **NEW allowlist stub** `.cargo-audit-ignore.toml` (≤ 30 LOC):
   ```toml
   # Vulnerability advisories to ignore. Document EVERY entry with
   # a justification + revisit date.
   #
   # [[advisory]]
   # id = "RUSTSEC-XXXX-YYYY"
   # reason = "..."
   # revisit = "2026-XX-XX"
   ```

4. **Doc**: `docs/skycot/runbook/security-audit.md` (NEW, ≤ 150 LOC) — covers: how to run locally, how to interpret findings, how to add allowlist entries, how to enable CI when billing returns.

5. **Pre-commit hook (optional)**: NEW `scripts/precommit-cargo-audit.sh` — fast path that only runs `cargo audit` if `Cargo.lock` changed. Operator wires into their pre-commit framework manually.

This is NOT in scope: enabling CI (intentionally `.disabled` per ledger header); auto-fix execution (manual review required); upgrading vulnerable deps (separate brief per finding); supply-chain attestation (separate brief).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `.github/workflows/audit.yml.disabled` (NEW, ≤ 60 LOC)
- `scripts/cargo-audit.sh` (NEW, ≤ 80 LOC, executable bash)
- `scripts/precommit-cargo-audit.sh` (NEW, ≤ 40 LOC, executable bash)
- `.cargo-audit-ignore.toml` (NEW, ≤ 30 LOC, comment-only stub)
- `docs/skycot/runbook/security-audit.md` (NEW, ≤ 150 LOC)

## Files you must NOT touch

- Any other `.github/workflows/*.yml.disabled` file
- `Cargo.lock` / any `Cargo.toml`
- Any `crates/**` source
- `crates/argo-store/migrations/**`

## Acceptance criteria

- [ ] `.github/workflows/audit.yml.disabled` exists with daily schedule + pr-on-lock-change
- [ ] `scripts/cargo-audit.sh` runs locally (idempotent install, honors ignore file)
- [ ] `scripts/cargo-audit.sh --help` works
- [ ] `.cargo-audit-ignore.toml` stub exists with template entries (commented-out)
- [ ] `docs/skycot/runbook/security-audit.md` covers run-local + interpret + allowlist + ci-enable
- [ ] Local script verified by author running it and pasting `cargo audit` output excerpt in PR body
- [ ] No code changes, no migrations
- [ ] Every new file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-cargo-audit-disabled-workflow`
- PR title: `security: cargo-audit CI workflow (disabled) + local-runnable script + runbook (mission-cargo-audit-disabled-workflow)`

## Notes for the implementer

- This is **purely operator-value**: workflow stays disabled until CI billing returns. The local script delivers immediate value (run-on-demand vuln scan).
- The `.disabled` filename convention is intentional — matches the 5 other `.disabled` workflows. Don't rename.
- The local script should be idempotent — running it twice in a row shouldn't reinstall cargo-audit.
- If `cargo audit` finds vulnerabilities at run time, paste the output (sanitised — no operator-machine paths) into the PR body. Don't try to fix vulns in this brief.
- The runbook target audience: a security-conscious operator who wants to triage vuln findings during a release readiness pass. Keep it punchy.
