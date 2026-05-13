# Secret-scan pattern catalogue

The pre-commit hook (`scripts/git-hooks/pre-commit-secret-scan.sh`)
and the CI parity command (`argo security scan-secrets`) share the
same regex catalogue. This document is the authoritative spec for
that catalogue — adding a new pattern means a 1-row tuple in
`crates/argo-cli/src/handlers/security_scan_secrets.rs::patterns`
plus a documentation row here.

## Operator workflow

1. **Hook trips on a commit.** The hook prints `file:line` plus a
   redacted preview (first 4 + last 4 chars of the match) and the
   `regex_id` / `match_hash` tuple.
2. **Rotate or allowlist.** If the match is a real credential,
   rotate it (use `argo cred rotate ...` for first-party keys) and
   remove it from the file. If it's a known-safe fixture, paste the
   tuple into `.skycot/secret-scan-allowlist.toml` and re-stage.
3. **CI re-runs the scan.** The CI parity command (`argo security
   scan-secrets`) reads the same allowlist, so a fixture flagged
   locally is suppressed in CI too.

## Catalogue

| regex_id            | Category                | Pattern (regex)                                                                |
|---------------------|-------------------------|--------------------------------------------------------------------------------|
| `openai_sk`         | OpenAI API key          | `sk-[A-Za-z0-9_\-]{20,}`                                                       |
| `anthropic_key`     | Anthropic API key       | `sk-ant-[A-Za-z0-9_\-]{20,}`                                                   |
| `aws_access_key_id` | AWS access key id       | `AKIA[0-9A-Z]{16}`                                                             |
| `stripe_live_secret`| Stripe live key         | `(?:sk\|rk)_live_[A-Za-z0-9]{20,}`                                              |
| `slack_token`       | Slack token             | `xox[abprs]-[A-Za-z0-9-]{10,}`                                                 |
| `github_pat`        | GitHub PAT              | `gh[pousr]_[A-Za-z0-9]{36,}`                                                   |
| `private_key_header`| PEM private-key header  | `-----BEGIN (?:RSA \|DSA \|EC \|OPENSSH \|PGP )?PRIVATE KEY-----`              |
| `jwt_long`          | JWT-shaped token        | `eyJ[A-Za-z0-9_\-]{20,}\.[A-Za-z0-9_\-]{20,}\.[A-Za-z0-9_\-]{20,}`             |
| `argo_bearer_token` | Argo gateway bearer     | `ARGO_GATEWAY_BEARER_TOKEN\s*[:=]\s*["']?[A-Za-z0-9_\-]{16,}`                  |
| `argo_admin_token`  | Argo admin token        | `ARGO_ADMIN_TOKEN\s*[:=]\s*["']?[A-Za-z0-9_\-]{16,}`                           |

## Design notes

- **Bias toward false-positives.** Better to occasionally trip on
  a fake-looking string than to miss a real credential. The
  allowlist is the operator escape valve, and the redacted preview
  is enough to triage a hit in <30 seconds.
- **Redaction format.** First 4 chars + `...` + last 4 chars, e.g.
  `sk-1...wxyz`. Strings under 12 chars are fully masked.
- **Match hash.** SHA-256 of the raw match, hex-encoded. Stable
  across machines; safe to paste into the allowlist file without
  re-leaking the secret.
- **What we don't scan.** Binary files (first 1 KiB contains NUL),
  `.git/` objects, common binary extensions (`.png`, `.zst`, `.gz`).
  The pre-commit hook also skips lockfiles by extension to keep
  noise down on dependency bumps.

## Adding a new pattern

1. Add the tuple to `patterns()` in
   `crates/argo-cli/src/handlers/security_scan_secrets.rs`.
2. Add a row to the catalogue table above.
3. Add a unit test in
   `crates/argo-cli/src/handlers/security_scan_secrets_tests.rs`
   that asserts the new `regex_id` fires on a synthetic match.
4. Run `cargo test -p argo-cli` to confirm.

## Out of scope

- **History rewriting.** If a secret already leaked to git history,
  use `git-filter-repo` separately — not the scanner's job.
- **IDE integration.** Editors with secret-scan support (the
  GitHub Codespaces secret-scanner, JetBrains hooks) are orthogonal;
  the pre-commit hook is the floor everyone shares.
- **Secret rotation.** When a hit is a real credential, rotate it
  via `argo cred rotate` (Mission `mission-automated-key-rotation`),
  not this tool.
