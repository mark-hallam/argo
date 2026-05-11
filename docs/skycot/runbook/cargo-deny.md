# SKYCOT Operator Runbook — cargo-deny

Workspace supply-chain / license / advisory hygiene via cargo-deny.
Sibling tool to `cargo-audit` (see `security-audit.md`): `cargo-audit`
checks one thing (advisories); `cargo-deny` checks four
(advisories + bans + licenses + sources).

The CI workflow (`.github/workflows/deny.yml.disabled`) is
intentionally inert until GitHub Actions billing is restored. The
local script (`scripts/cargo-deny.sh`) and the opt-in pre-push hook
(`scripts/precommit-cargo-deny.sh`) deliver value today.

---

## Why cargo-deny

PRs that introduce a banned-license / yanked-crate / known-CVE
dependency would otherwise ship silently. cargo-deny gates four
classes of regression at once:

- **advisories** — known CVEs in transitive deps (`RUSTSEC-*` IDs).
  The most frequent firing reason; new advisories land daily.
- **bans** — explicitly forbidden crate names. Configured per-project
  (e.g. `openssl` if the policy is "ring only").
- **licenses** — every dep's SPDX license must appear in
  `[licenses].allow`. SKYCOT is Apache-2.0 / MIT; GPL / AGPL /
  proprietary fail closed.
- **sources** — registry / git-source allowlist. Prevents accidental
  pulls from a random git repo.

## Run locally

```sh
scripts/cargo-deny.sh
```

First invocation installs `cargo-deny` via `cargo install --locked`
(several minutes one-time). Subsequent invocations skip the install
and only run the scan (a few seconds, modulo advisory-DB fetch on
cold cache).

Exit codes:

- `0` — all four checks passed.
- non-zero — one or more checks failed; the summary names which.

Use `scripts/cargo-deny.sh --help` for argument summary.

## Interpret each check kind

### advisories

```text
error[vulnerability]: a vulnerable crate was found ...
   ┌─ /path/to/Cargo.lock
   │
N  │ tokio 1.30.0 registry+https://github.com/...
   │ ^^^^^ ID: RUSTSEC-2025-NNNN
```

Triage:

1. **Read the advisory page.** Many are scoped to specific feature
   flags or platforms and may not be reachable from `argo-*` code.
2. **Check exploit reachability.** `cargo tree -i <crate>` shows
   reverse deps.
3. **Pick a remedy:**
   - **Upgrade now** — preferred. Open a Cargo.lock-only PR bumping
     the dep, re-run `scripts/cargo-deny.sh` to confirm clean, ship.
   - **Allowlist with revisit** — only when no upgrade exists or the
     finding is unreachable. Follow the "Add an advisories.ignore
     entry" process below.

### bans

A `bans` failure means a crate explicitly listed in `[bans].deny`
appeared in the dependency graph. Either remove the dep, replace it
with the policy-approved alternative, or open a brief to change the
ban policy. Do not whitelist past a `bans.deny` entry — the entry
exists because someone made a policy decision.

### licenses

A `licenses` failure means a transitive dep carries an SPDX license
not present in `[licenses].allow`. Options in order of preference:

1. **Replace the dep** with a permissively-licensed alternative.
2. **Add the license to `[licenses].allow`** — only when the license
   is genuinely compatible with the SKYCOT distribution model. Bring
   to legal/security review first; document the addition in the PR
   body.
3. **Add a `[licenses].clarify`** entry — only when cargo-deny's
   detection is wrong (rare). Cite the dep's repo / SPDX identifier
   evidence.

GPL / AGPL / proprietary licenses MUST NOT be added.

### sources

A `sources` failure means a dep was pulled from a registry or git
URL not in the allowlist. Typically caused by `[patch.crates-io]`
or a `git = "..."` dep snuck into a `Cargo.toml`. Either remove the
override or, if the override is load-bearing, add the source to
`[sources].allow-registry` / `[sources].allow-git` with a comment
explaining why.

## Add an `[advisories].ignore` entry

Every ignored advisory is a known risk SKYCOT is accepting. The
process below keeps that honest.

1. **Open a discussion** — PR comment or issue describing the
   advisory, the affected crate's reachability from `argo-*`, and
   why we can't upgrade today.
2. **Security review.** A security lead (or, while the team is one
   person, the operator wearing the security hat) signs off in the
   discussion.
3. **2 business day soak.** Leave the discussion open at least 2
   business days for additional eyes. Skip the soak only for active
   exploitation, with operator approval in the PR body.
4. **Land the ignore.** Edit `deny.toml` `[advisories].ignore`:

   ```toml
   ignore = [
       # <one-line summary of why> — link to discussion.
       "RUSTSEC-YYYY-NNNN",
   ]
   ```

5. **Schedule the revisit.** Add a calendar reminder for the
   upstream patch ETA (or 90 days, whichever is sooner). When the
   patch lands, drop the ignore in the same PR that bumps the dep.

## Add a `[licenses].allow` entry

1. Confirm the license is compatible with Apache-2.0 / MIT
   distribution (legal review — same security-lead surrogate while
   single-operator).
2. Add the SPDX identifier alphabetically to `[licenses].allow` in
   `deny.toml`.
3. Document the addition in the PR body, citing the dep that forced
   the addition and the compatibility analysis.

## Pre-push hook (opt-in)

`scripts/precommit-cargo-deny.sh` runs the full four-check sweep on
push. It is opt-in because cargo-deny fetches the advisory DB on each
run (cached, but ~5s cold) and the pre-push hook needs to stay fast.
Wire it manually:

```sh
git config core.hooksPath .githooks
mkdir -p .githooks
ln -s ../scripts/precommit-cargo-deny.sh .githooks/pre-push
```

The hook is no-op when cargo-deny is not installed, so peers without
the tool will not be blocked from pushing.

## Re-enable CI when billing returns

```sh
git mv .github/workflows/deny.yml.disabled .github/workflows/deny.yml
git commit -m "ci: enable cargo-deny workflow"
git push
```

The workflow then runs on:

- every PR to `main` / `master`
- every push to `main`
- daily at 06:00 UTC (catches newly-disclosed advisories against
  otherwise-unchanged deps)
- on-demand via `workflow_dispatch`

The matrix runs the four checks (advisories, bans, licenses,
sources) independently so one failure doesn't mask the others.

## Out of scope

- **Upgrading vulnerable deps.** One brief per advisory; this
  runbook covers the gate, not the fix.
- **Automatic ignore aging.** `[advisories].ignore` revisits are
  operator-driven calendar reminders, not an automated sweep.
- **cargo-vet / cargo-supply-chain.** Tracked separately.
- **Per-crate `[licenses].clarify` tracking.** Operators add as
  needed; not a default deliverable.
