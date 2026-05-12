# Mission load-bearing wire — Secret-scanning pre-commit hook

**Owner**: claude.
**Authored against**: origin/main `d454edb3` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 7 (security ops). Today nothing prevents an accidental commit of `OPENAI_API_KEY=sk-...` or `ARGO_GATEWAY_BEARER_TOKEN=...` into a brief / test fixture / sample config. Github's secret scanning catches some but not custom token formats. Add a local pre-commit hook with tighter regex coverage.

## Scope

NEW `scripts/git-hooks/pre-commit-secret-scan.sh` + a Rust verifier `argo-cli` subcommand for CI parity:

1. **Regex set** covering: OpenAI sk-/ANTHROPIC_API_KEY/AWS access key ids/Stripe live keys/Slack bot tokens/GitHub PATs/SSH private key headers/JWT-shaped strings >100 chars/our own `ARGO_*` token patterns
2. **Pre-commit hook** — runs against staged files; if any match → fail commit with file:line + redacted match preview
3. **CI parity command** — `argo security scan-secrets <paths>` runs the same scan in CI (catches anything that slipped past the pre-commit hook)
4. **Allowlist** — `.skycot/secret-scan-allowlist.toml` for known-safe matches (test fixtures with synthetic-looking secrets)
5. **Install helper** — `argo doctor install-hooks` copies the hook to `.git/hooks/pre-commit`

This is NOT in scope: history rewriting if a secret already leaked (use `git-filter-repo` separately); IDE-side integration; secret-rotation if detected (use the rotation brief downstream).

## Files you may touch

- `scripts/git-hooks/pre-commit-secret-scan.sh` (NEW, ≤ 250 LOC bash)
- `crates/argo-cli/src/handlers/security_scan_secrets.rs` (NEW, ≤ 350 LOC) — Rust parity for CI
- `crates/argo-cli/src/handlers/security_scan_secrets_tests.rs` (NEW, sibling, ≤ 300 LOC) — 8 tests with embedded fixtures
- `crates/argo-cli/src/handlers/doctor_install_hooks.rs` (NEW, ≤ 120 LOC)
- `crates/argo-cli/src/cli_security_cmds.rs` + `cli_doctor_cmds.rs` — new subcommand variants
- `docs/skycot/security/secret-patterns.md` (NEW, ≤ 200 LOC) — documents the regex set + how to add a new pattern
- `.skycot/secret-scan-allowlist.toml.example` (NEW, ≤ 30 LOC) — operator-template

## Files you must NOT touch

- The existing `cargo audit` integration (different brief, different threat model)
- Existing test fixtures (they may need to be added to the allowlist; that's expected)

## Acceptance criteria

- [ ] Hook fails on a staged file containing a fake `sk-1234567890abcdef...` pattern
- [ ] Allowlist entries skip matches by `(file_glob, regex_id, match_hash)` tuple
- [ ] `argo security scan-secrets <path>` exits non-zero on match, 0 on clean
- [ ] Match output is redacted (shows first 4 chars + last 4, masks middle)
- [ ] 8 unit tests covering: each regex category + allowlist suppression + redaction format
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-secret-scanning-pre-commit-hook`
- PR title: `security: pre-commit secret-scan hook + CI parity (mission-secret-scanning-pre-commit-hook)`

## Notes

- Bias toward false-positives — better to occasionally trip on a fake-looking string than to miss a real one. Allowlist is the operator escape valve.
- Redacted match preview: `sk-1234...wxyz` is enough for the operator to identify the location without further leaking.
- Pair this with the vuln-management workflow brief — both produce machine-readable findings that feed an ops dashboard.
