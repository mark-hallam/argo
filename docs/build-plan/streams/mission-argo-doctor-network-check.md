# mission-argo-doctor-network-check — provider + DNS connectivity in `argo doctor` (P2)

**Owner**: codex. **Depends on**: `mission-cli-version-doctor-extras` MERGED (#1768).
**Authored against**: origin/main `8a6029ac` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-argo-doctor-network-check`.
**Source**: post-v1 polish — operators report "argo doctor said OK but provider call still 500'd" — turns out DNS resolution / TLS handshake / proxy interception is the gap. Doctor needs to actually probe.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — surface network-layer failures before they hit user requests.
- **Reliability** (#1) — first port of call during incident triage. False-positive OK is corrosive.

## Scope

P2. Add three network checks to `argo doctor` that run by default (operator can `--skip-network` for offline scenarios):

1. **DNS resolution** — resolve `api.anthropic.com`, `api.openai.com`, `*.skycot.com` (apex). Report each as OK / FAIL with resolved IP or error string.
2. **TLS handshake** — connect to each on port 443 + read certificate fingerprint + verify validity (not expired, signed by trusted CA). Report cert expiry date.
3. **Provider auth probe** — if a provider API key is configured, fire a $0.001 minimum-cost request (e.g. `GET https://api.anthropic.com/v1/models` with auth header). Report status code + latency. If no key configured, SKIP.

Output (text mode):
```
Build Info
  version: 1.0.0+a1b2c3d
  rustc:   1.79.0 stable
  target:  aarch64-apple-darwin

Health checks
  ✅ workspace                   ~/.argo (writable)
  ✅ database                    62 migrations applied
  ✅ provider                    anthropic configured
Network checks
  ✅ DNS api.anthropic.com       104.18.42.103
  ✅ TLS api.anthropic.com       expires 2026-08-14
  ✅ AUTH api.anthropic.com      200 in 142ms
  ✅ DNS api.openai.com          104.18.42.234
  ✅ TLS api.openai.com          expires 2026-09-22
  ⚠️  AUTH api.openai.com         no key configured (SKIP)
  ✅ DNS skycot.com              cloudflare
  ✅ TLS skycot.com              expires 2026-08-30
```

`--json` output: nested `network_checks: { dns: [...], tls: [...], auth: [...] }`.

LOC: ~150.

## Files you may touch

- `crates/argo-cli/src/handlers/doctor.rs` — add `print_network_checks()` + helpers. ≤80 modified LOC.
- `crates/argo-cli/src/handlers/doctor_network.rs` (NEW) — DNS + TLS + auth-probe logic. ≤120 LOC.
- `crates/argo-cli/src/handlers/doctor_network_tests.rs` (NEW) — tests with offline-mode and stubbed responses. ≤80 LOC.
- `crates/argo-cli/Cargo.toml` — narrowly: add `reqwest` (already in workspace via providers — verify before adding). ≤5 modified LOC.
- `docs/skycot/cli-reference.md` (or `book/reference/cli.md` per #1767 finding) — update doctor section. ≤15 modified LOC.

## Files you must NOT touch

- Provider client code (`argo-provider`) — read-only consumer.
- Audit chain — doctor is read-only.
- Anything outside the doctor handler.

## Acceptance criteria

- [ ] `argo doctor` runs DNS+TLS+auth checks by default.
- [ ] `argo doctor --skip-network` skips all 3 (useful for airgapped operators or CI).
- [ ] `argo doctor --json` includes `network_checks` field with nested DNS/TLS/auth results.
- [ ] No provider key configured for openai → SKIP entry for AUTH, not FAIL.
- [ ] DNS lookup uses async resolver (`tokio::net::lookup_host`); 5-second timeout per host; FAIL with timeout msg if exceeded.
- [ ] TLS handshake uses `reqwest::Client` with 5-second timeout; cert expiry extracted via `openssl` or `rustls` API.
- [ ] Auth probe uses minimum-cost endpoint (`/v1/models` for Anthropic + OpenAI); doesn't actually invoke an LLM.
- [ ] Tests stub all network I/O (no real outbound calls in CI).
- [ ] Exit code 0 if all OK, 1 if any FAIL (DEGRADED), 2 if multiple FAIL (FAIL).
- [ ] `cargo test -p argo-cli` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-argo-doctor-network-check`
- Title: `feat(doctor): DNS + TLS + provider auth probes (mission-argo-doctor-network-check)`
- Worker verification: crate-scoped + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **No outbound calls in tests.** Use `wiremock` or hand-rolled stubs for the auth probe. Tests run offline in CI.
- **Cert expiry parsing varies by TLS lib.** `rustls` gives you `x509-parser`'s `Validity`; `native-tls` gives you raw bytes. Pick the cleaner path; document choice in PR body.
- **`--skip-network` is an env var too** (`ARGO_DOCTOR_SKIP_NETWORK=1`) for headless ops.
- **Operator-configured hosts**: read from `argo.toml` `[doctor] additional_hosts = ["api.example.com"]` for custom probes; default to the 3 listed above.
- **Don't auth-probe arbitrary URLs.** Only the operator-known set; otherwise a config-injection bug becomes an SSRF.
- **Cloudflare-proxied** Skycot apex resolves through CF; that's expected, document in PR body.
