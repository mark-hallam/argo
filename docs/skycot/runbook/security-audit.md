# SKYCOT Operator Runbook — Vulnerability Audit

Run-on-demand vulnerability scanning of the workspace's pinned
dependencies, plus a CI workflow staged in disabled state.

The CI workflow (`.github/workflows/audit.yml.disabled`) is
intentionally inert until GitHub Actions billing is restored. The
local script (`scripts/cargo-audit.sh`) and the pre-commit hook
(`scripts/precommit-cargo-audit.sh`) deliver value today.

---

## Run locally

```sh
scripts/cargo-audit.sh
```

First invocation installs `cargo-audit` via `cargo install --locked`
(several minutes one-time). Subsequent invocations skip the install
and only run the scan (a few seconds).

Exit codes:

- `0` — clean (no advisories, or all hits allowlisted).
- non-zero — at least one advisory matched a workspace dependency.

Use `scripts/cargo-audit.sh --help` for argument summary.

## Interpret findings

A typical hit looks like:

```text
Crate:     tokio
Version:   1.30.0
Title:     ...
Date:      2025-08-12
ID:        RUSTSEC-2025-NNNN
URL:       https://rustsec.org/advisories/RUSTSEC-2025-NNNN
Severity:  6.5 (medium)
Solution:  Upgrade to >=1.32.0
```

Triage steps in order:

1. **Read the advisory page.** Not every advisory is exploitable in
   our context — many are scoped to specific feature flags or
   platforms.
2. **Check exploit reachability.** Is the affected code path used by
   any `argo-*` crate? `cargo tree -i <crate>` shows reverse deps.
3. **Pick a remedy:**
   - **Upgrade now** — preferred. Open a one-line Cargo.lock-only PR
     bumping the dep, run the local script again to confirm the
     advisory is gone, and ship.
   - **Allowlist with revisit date** — when no upgrade exists or the
     finding is unreachable. Add an entry to
     `.cargo-audit-ignore.toml` (template below).
   - **Open a remediation brief** — for non-trivial upgrades that
     touch our code. One brief per advisory keeps the audit trail
     clean.

## Add an allowlist entry

Edit `.cargo-audit-ignore.toml`:

```toml
[[advisory]]
id = "RUSTSEC-2025-NNNN"
reason = "Affected code path not reachable from argo-* — feature flag X disabled in our build."
revisit = "2026-08-01"
```

`scripts/cargo-audit.sh` extracts each `id = "..."` line into a
`--ignore` flag passed to `cargo audit`. The `reason` and `revisit`
fields are documentation for the next operator who reads the file —
preserve them.

Re-run `scripts/cargo-audit.sh` after editing to confirm the
advisory is suppressed.

## Re-enable CI when billing returns

```sh
git mv .github/workflows/audit.yml.disabled .github/workflows/audit.yml
git commit -m "ci: enable cargo-audit workflow"
git push
```

The workflow runs daily at 09:00 UTC and on any PR that touches
`Cargo.lock` or `.cargo-audit-ignore.toml`. Failures surface as a
red status check on the PR.

## Pre-commit hook (optional)

`scripts/precommit-cargo-audit.sh` is the fast-path variant — it
skips the scan unless `Cargo.lock` is staged. Wire it into your
local pre-commit framework manually; example:

```sh
ln -s "$(pwd)/scripts/precommit-cargo-audit.sh" .git/hooks/pre-commit
```

Or, with `pre-commit` (https://pre-commit.com):

```yaml
- repo: local
  hooks:
    - id: cargo-audit
      name: cargo-audit
      entry: scripts/precommit-cargo-audit.sh
      language: system
      pass_filenames: false
```

## Out of scope

- **Upgrading vulnerable deps.** One brief per advisory; the audit
  surface is read-only.
- **Auto-fix execution.** `scripts/cargo-audit.sh --fix` runs
  `cargo update` but does not commit; an operator must review the
  Cargo.lock diff.
- **Supply-chain attestation** (sigstore, cosign, SBOM signing).
  Tracked separately.
