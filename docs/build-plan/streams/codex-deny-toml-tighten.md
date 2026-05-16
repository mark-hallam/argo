# codex-deny-toml-tighten — tighten `deny.toml` policy (P2 mechanical)

**Owner**: codex. **Sizing hint**: fast-mode candidate — single-file mechanical change.
**Depends on**: none.
**Authored against**: origin/main `de644311` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-deny-toml-tighten`.
**Source**: post-v1 polish — `cargo deny check` CI gate exists (deny.yml workflow), but the policy is permissive. Pre-launch enterprise hardening.

## Mission alignment

- **Security** (CLAUDE.md priority #4) — explicit license allowlist + advisory denylist + source allowlist.
- **Reliability** (#1) — supply-chain advisory checks gate merges.

## Scope

P2 mechanical. Update `deny.toml` at workspace root with:

### 1. Licenses — explicit allowlist (was: warn-mode)

```toml
[licenses]
# Confidence floor: require 80% confidence to detect a license
confidence-threshold = 0.8

# Explicit allowlist — anything NOT here errors
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",   # used by unicode-ident
    "Zlib",                # rare but BSD-compatible
    "CC0-1.0",             # public-domain-equivalent
    "OpenSSL",             # for openssl-sys subtree (if present)
    "MPL-2.0",             # rare; weak-copyleft
]
```

### 2. Advisories — block CRITICAL + HIGH; warn LOW/MEDIUM

```toml
[advisories]
vulnerability = "deny"          # block any unresolved CVE in dep tree
unmaintained  = "warn"          # warn on unmaintained crates
yanked        = "deny"           # block yanked versions
notice        = "warn"
severity-threshold = "medium"   # don't trip on LOW
```

### 3. Bans — disallow specific risky crates

```toml
[bans]
multiple-versions = "warn"
wildcards = "deny"               # no `dep = "*"` in Cargo.toml
highlight = "all"

# Specific deny: crates with known issues we don't want to permit
deny = [
    { name = "openssl", reason = "use rustls instead — already in workspace" },
    { name = "git2", reason = "FFI to libgit2 — prefer pure-Rust `gix` for build scripts" },
]
skip = [
    # Workspace duplicates we're aware of (one-line justification per entry)
]
```

### 4. Sources — only crates.io + workspace path deps

```toml
[sources]
unknown-registry = "deny"        # no random Git URLs in Cargo.toml
unknown-git      = "deny"
allow-registry   = ["https://github.com/rust-lang/crates.io-index"]
```

### Operator update path

Add a top-of-file comment:
```
# Tighten policy edits go through PR review. Adding to `allow = [...]`
# or `skip = [...]` is a stand-alone PR with justification in the
# commit message. Don't `*` anything.
```

LOC: ~80 (mostly TOML).

## Files you may touch

- `deny.toml` (workspace root, existing — extend). ≤70 modified LOC.
- `docs/skycot/security/dep-policy.md` (NEW) — operator runbook for adding licenses/skips. ≤40 LOC.

## Files you must NOT touch

- Any `Cargo.toml`. If tightening surfaces real issues, those are follow-up briefs.
- `.github/workflows/deny.yml` — the workflow itself is fine.
- Any source code.

## Acceptance criteria

- [ ] `cargo deny check advisories bans sources licenses` passes on current workspace.
- [ ] If any check fails, the failure is added to `skip` / `allow` with a justification comment, OR the PR explicitly defers it as a follow-up brief.
- [ ] `unknown-registry` = `deny` enforced — no Git URL deps possible.
- [ ] No wildcard versions in any Cargo.toml (verify by `rg '\s*=\s*"\*"' '**/Cargo.toml'`).
- [ ] Operator runbook documents the policy + how to add to allow/skip.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-deny-toml-tighten`
- Title: `chore(deny): tighten policy — explicit license allowlist + advisory denylist + source allowlist (codex-deny-toml-tighten)`
- Worker verification: `cargo deny check` clean per the new policy, per SWARM_PROTOCOL.md.

## Hidden constraints

- **Don't bump dep versions.** This brief tightens the policy; it doesn't fix any pre-existing violations. If `cargo deny check` fails on the tighter policy, ADD a justified skip OR open a follow-up brief.
- **`unmaintained = "warn"` not "deny"** — too many otherwise-fine crates are marked unmaintained in advisory DB.
- **Don't deny `openssl` if any current dep transitively requires it** — check first; if it's pulled by something we use, add to allow OR add to skip with a follow-up to migrate.
- **Operator decides.** If the brief surfaces a license-allow question (e.g. "is GPL-3.0 acceptable for X?"), comment in the PR body and let the operator decide before merging.
