# pricing-byok-encrypt-keys-at-rest

**Owner**: claude or codex — **P2 SECURITY (tenant credentials).**
**Depends on**: #2861 (BYOK per-tenant provider keys — landed the write path).
**Authored against**: origin/main `c8b69418f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-byok-encrypt-keys-at-rest

## Problem (adversarial review of #2861, non-blocking finding)

The BYOK write path (`POST /api/v1/providers/keys` → `cred::set_tenant`)
stores the tenant's upstream provider key as `CredentialEntry::Plain` — so a
real Anthropic/OpenAI key lands **plaintext** in `.argo/credentials.toml`
(mode `0o600`). The store ALREADY supports `CredentialEntry::Encrypted` (the
read path handles both), and a master key exists (per-tenant master key,
#2090). This write path just doesn't encrypt.

Not a merge blocker (file is owner-only `0o600`, key never reflected in
responses/audit/logs — review CONFIRMED-SOUND on leak surfaces), but a real
provider credential at rest should be encrypted, matching the store's
existing decrypt-capable read path.

## Fix
Route the BYOK `set_tenant` write through the master-key **encrypt** path so
per-tenant provider keys are stored as `CredentialEntry::Encrypted`. Confirm
the read path (`cred::load_tenant`, used by `resolve_provider_api_key`)
transparently decrypts. Keep the no-reflection discipline (response stays
`masked_preview:null`, audit records only that a key was set + by whom).

## Acceptance
- A BYOK key written via the route is stored `Encrypted`, not `Plain`; a test
  asserts the on-disk TOML does not contain the plaintext secret.
- `resolve_provider_api_key` still resolves the tenant key correctly (existing
  #2861 resolution tests stay green).

~60-120 LOC. **🟡 P2 — encrypt the credential at rest.**
