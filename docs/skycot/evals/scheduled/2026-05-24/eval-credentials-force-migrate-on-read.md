# eval-credentials-force-migrate-on-read

Verdict: PASS

`argo-credentials` no longer leaves legacy plaintext entries in place after a
normal crate-local read. `read_encrypted` still returns the same plaintext
values for backward compatibility, but when it sees `EncryptedEntry::Plain` it
rewrites the file through the existing wrapped-entry format before returning.
Already wrapped entries are decrypted for the returned map and preserved
byte-for-byte when a mixed document is rewritten.

What changed:

- `crates/argo-credentials/src/lib.rs:104` now tracks whether a parsed
  credentials document contains legacy plaintext entries during
  `read_encrypted`.
- Plain entries are encrypted with the supplied workspace master key and written
  back as `[credentials.<KEY>].value` wrapped entries.
- Existing wrapped entries keep their original blob when a mixed file is
  rewritten.
- The shared document writer now writes to a same-directory temporary file,
  fsyncs it, renames it over the destination, and applies `0600` permissions on
  Unix.

Pinned tests:

- Normal read of a legacy plaintext file returns the original values, removes
  plaintext from disk, leaves no temp file behind, and produces mode `0600` on
  Unix.
- Normal read of an already encrypted file does not rewrite or corrupt the file
  body.
- Normal read of a mixed plaintext/encrypted file encrypts only the plaintext
  entries and preserves the pre-existing wrapped blob.
- Existing explicit `migrate_plaintext_in_place` coverage still verifies the
  operator command path and loose-permission warning contract.

Workspace boundary:

- This crate stores a workspace credentials document at
  `<workspace>/.argo/credentials.toml`; the workspace path remains the boundary
  for this legacy wrapped-entry format.
- `crates/argo-config/src/cred.rs:5-29` documents the higher-level tenancy model
  for consumers: tenant-scoped sections are read first, with legacy root-level
  fallback only as a migration ramp.
- `crates/argo-config/src/cred.rs:274-315` shows file reads stay scoped to the
  caller-provided workspace path.
- `crates/argo-cli/src/handlers/credentials_encrypt_all.rs:27-33` remains the
  explicit operator migration command over the same crate-local helper.

Caveats:

- No CLI, gateway, connector, keyring, SQL, or tenant-column callsite was
  changed; those were outside this brief's may-touch list.
- `argo-config::cred` has its own tenant-aware document parser and legacy
  fallback policy. This brief only closes the `argo_credentials::read_encrypted`
  `EncryptedEntry::Plain` acceptance path called out by the evaluator residue.

Verification:

- `bash scripts/check-brief-freshness.sh eval-credentials-force-migrate-on-read`
  reported only the authored-SHA distance warning; structural checks were clean.
- `CARGO_TARGET_DIR=/tmp/eval-credentials-force-migrate-on-read-target cargo test -p argo-credentials`
- `CARGO_TARGET_DIR=/tmp/eval-credentials-force-migrate-on-read-target cargo clippy -p argo-credentials --all-targets -- -D warnings`
