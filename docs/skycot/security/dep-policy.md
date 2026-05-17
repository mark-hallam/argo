# Dependency Policy

SKYCOT gates dependency risk with `cargo-deny`. The policy lives in
`deny.toml`; the command runbook lives in
`docs/skycot/runbook/cargo-deny.md`.

## Policy

- Registries: crates.io only. New registries or Git sources require a
  dedicated PR with the source, owning team, and removal plan.
- Licenses: only the SPDX identifiers in `[licenses].allow` pass.
  GPL, AGPL, proprietary, or unknown third-party licenses are not
  approved for distribution.
- Advisories: vulnerabilities are denied unless an explicit
  `[advisories].ignore` entry documents reachability, mitigation, and
  the follow-up. Unmaintained and unsound advisories use the same
  exception path when they affect the current graph.
- Bans: wildcard versions are denied for registry dependencies.
  Internal path dependencies are allowed because Cargo reports them as
  wildcard requirements even when no `Cargo.toml` contains `version =
  "*"`.

## Updating Allowlists

Use a stand-alone PR for every addition to `allow`, `ignore`, `deny`,
`skip`, `allow-registry`, or `allow-git`.

1. Run `cargo deny check advisories bans sources licenses`.
2. Identify the exact crate, version, and reverse dependency path.
3. Prefer upgrading or removing the dependency.
4. If an exception is still required, add a one-line reason in
   `deny.toml` and describe the operational risk in the PR body.
5. Re-run the four cargo-deny checks and include the result in the PR.

Temporary advisory ignores need a follow-up owner and revisit trigger.
Do not add blanket ignores, wildcard versions, or broad source
allowlists.
