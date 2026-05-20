# Release / build profiles

This doc lists the operator-facing build profiles SKYCOT ships. Each
profile is just a Cargo feature set + environment, not a separate
binary — the gateway boot path inspects the feature flags and runtime
config to decide which startup gates to enforce.

## Enterprise build profile

The enterprise profile is the fail-closed configuration for at-rest
encryption (`mission-audit-sqlcipher-enterprise-default`). It refuses
to boot in production mode unless the deployment is provably encrypted
end-to-end.

### Build

```bash
cargo build --release --features bundled-sqlcipher
```

`bundled-sqlcipher` links the SQLCipher engine via libsqlite3-sys'
bundled C source with vendored OpenSSL (no system-OpenSSL dependency,
cross-platform reproducible). It implies the `sqlcipher` Cargo feature
that `encryption::sqlcipher_linked()` reads at runtime — the single
compile-time source of truth for "is this binary SQLCipher-linked?".

### Run

The gateway boot path enforces three preconditions when production
mode is active. All three are required; any one missing aborts boot
with a `Schema`-class error and no listener is bound.

1. Set `ARGO_GATEWAY_PROFILE=production` (or bind to a non-loopback
   address — both flip `is_production_mode()` to `true`).

2. Configure the at-rest-encryption key source in `argo.toml`:

   ```toml
   [security.encryption]
   # Required in production. The default "none" is dev-only and the
   # production-mode gate refuses to boot when it sees it.
   key_source = "env:ARGO_DB_KEY"
   ```

   The supported `key_source` forms (see
   `crates/argo-store/src/encryption.rs::KeySource::parse`):

   - `"env:VAR_NAME"` — raw 64-hex-char key from the named env var.
   - `"file:/path"` — raw 64-hex-char key from disk (0600 perms).
   - `"aws-kms:<arn>?region=<r>&wrapped=<spec>"` — BYOK via AWS KMS.
   - `"gcp-cmek:<proj>/<loc>/<ring>/<key>?wrapped=<spec>"` — BYOK via
     GCP Cloud KMS.
   - `"azure-kv:<vault_url>#<key>?wrapped=<spec>"` — BYOK via Azure
     Key Vault.

3. Provide the key itself. For the `env:` form:

   ```bash
   export ARGO_DB_KEY=<64-hex-char-secret>
   argo gateway start --bind 0.0.0.0:8443
   ```

### Verifying

If any of the three preconditions are missing, boot fails with a
message that names BOTH possible causes so the operator does not need
to consult the runbook to triage:

```text
enterprise mode requires SQLCipher: rebuild with --features
bundled-sqlcipher AND set security.key_source != "none"
(got: feature_enabled=<bool>, key_source=<value>)
```

A successful production boot logs nothing extra — the absence of the
error above is the success signal. To smoke-test the gate without
flipping a real deployment into production:

```bash
ARGO_GATEWAY_PROFILE=production cargo run --release \
    --features bundled-sqlcipher -- gateway start --bind 127.0.0.1:8443
```

(If `argo.toml` still has `key_source = "none"`, the command exits
with the error above instead of binding the listener.)

### Notes

- The gate is intentionally fail-closed. There is no "warn + continue"
  mode — silently booting an unencrypted production database is the
  whole class of failure this gate exists to prevent.
- Dev mode (`ARGO_GATEWAY_PROFILE` unset and bind is loopback)
  preserves the historical `key_source = "none"` default so local-dev
  workflows are unchanged.
- The CI runner that exercises the full `bundled-sqlcipher` build
  needs cargo's `bundled-sqlcipher-vendored-openssl` C compile to
  succeed (~3 min cold). The audit-derived test for this gate
  (`crates/argo-store/src/store_open_enterprise_tests.rs`) runs
  independently of the linked SQLite driver because the gate is a
  pure config-validation check — full encrypted-disk verification
  waits on the CI billing unblock noted in
  `BUILD_STATUS.md`.
