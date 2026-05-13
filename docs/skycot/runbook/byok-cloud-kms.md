# BYOK setup — bring your own cloud KMS key

Stream `mission-byok-enterprise-tenants`. Enterprise procurement
teams typically require that *their* KMS — not an operator-pasted
key — control the root of trust for at-rest encryption. SKYCOT
supports this via three feature-gated cloud KMS variants of the
`key_source` config: AWS KMS, GCP Cloud KMS (CMEK), and Azure
Key Vault.

See [`at-rest-encryption.md`](at-rest-encryption.md) for the
local-key (`env:` / `file:`) story; BYOK extends the same
`[security.encryption] key_source` config with cloud-KMS prefixes.

## Threat model

SKYCOT uses **envelope encryption**:

1. Operator generates one 32-byte data encryption key (DEK).
2. Operator wraps the DEK with their KMS key (`kms:Encrypt`) once
   at setup. The output is a *wrapped DEK* ciphertext blob.
3. The wrapped DEK lives on the SKYCOT host (file or env var).
4. At startup SKYCOT calls KMS to `Decrypt` the wrapped DEK. The
   plaintext DEK is what feeds `PRAGMA key`.
5. KMS sees only the wrapped DEK, never the plaintext database
   pages.

A stolen database file without KMS access is unreadable:
SQLCipher pages decrypt under the DEK, and the DEK only exists in
plaintext form inside an IAM-authorized SKYCOT process.

## Configuration

Each provider is compiled in behind its own Cargo feature:

```bash
cargo install --path crates/argo-cli --features kms-aws    # AWS KMS
cargo install --path crates/argo-cli --features kms-gcp    # GCP CMEK
cargo install --path crates/argo-cli --features kms-azure  # Azure KV
```

Default builds compile in none of these (no cloud HTTP stack
pulled). Operators can enable a subset — features are additive.

In `argo.toml`:

```toml
[security.encryption]
# AWS KMS:
key_source = """
aws-kms:arn:aws:kms:us-east-1:111122223333:key/abcd-1234?\
region=us-east-1&\
wrapped=file:/var/run/secrets/argo/wrapped.dek&\
bearer=env:AWS_KMS_BEARER
"""

# GCP Cloud KMS:
# key_source = "gcp-cmek:my-project/global/skycot-ring/db?wrapped=file:/var/run/secrets/argo/wrapped.dek&bearer=env:GCP_OAUTH"

# Azure Key Vault:
# key_source = "azure-kv:https://my-vault.vault.azure.net#db-key?wrapped=file:/var/run/secrets/argo/wrapped.dek&bearer=env:AZURE_AAD"
```

`wrapped=` and `bearer=` both follow the local `env:VAR` /
`file:/path` vocabulary so no secret material is ever inlined in
`argo.toml`. The bearer carries auth context: a SigV4 session
string for AWS, an OAuth access token for GCP, an AAD bearer
token for Azure. v1 expects an external sidecar (Workload
Identity, IRSA, AWS Vault, Managed Identity) to mint these.

## IAM policy per cloud

> **AWS KMS** — the IAM principal SKYCOT runs as must hold
> `kms:Decrypt` (always) and `kms:Encrypt` (only if you use
> rotation) on the KMS key ARN. A minimum policy:
>
> ```json
> {
>   "Version": "2012-10-17",
>   "Statement": [{
>     "Effect": "Allow",
>     "Action": ["kms:Decrypt", "kms:Encrypt", "kms:DescribeKey"],
>     "Resource": "arn:aws:kms:us-east-1:111122223333:key/abcd-1234"
>   }]
> }
> ```

> **GCP Cloud KMS** — grant `roles/cloudkms.cryptoKeyEncrypterDecrypter`
> on the CryptoKey resource:
>
> ```bash
> gcloud kms keys add-iam-policy-binding db \
>   --keyring skycot-ring --location global \
>   --member "serviceAccount:skycot@my-project.iam.gserviceaccount.com" \
>   --role roles/cloudkms.cryptoKeyEncrypterDecrypter
> ```

> **Azure Key Vault** — assign the **Key Vault Crypto User** role
> (or the legacy `wrapKey`/`unwrapKey` access-policy permissions)
> to the SKYCOT managed identity scoped to the vault.

## Bootstrap (one-time)

```bash
# 1. Generate a fresh 32-byte DEK.
openssl rand -out /tmp/dek.bin 32

# 2. Wrap it under your cloud KMS (AWS example).
aws kms encrypt \
  --key-id arn:aws:kms:us-east-1:111122223333:key/abcd-1234 \
  --plaintext fileb:///tmp/dek.bin \
  --output text --query CiphertextBlob | base64 -d \
  > /var/run/secrets/argo/wrapped.dek
chmod 0400 /var/run/secrets/argo/wrapped.dek

# 3. Securely destroy the plaintext DEK on disk.
shred -u /tmp/dek.bin

# 4. Point argo.toml at the wrapped DEK and restart SKYCOT.
```

Production bootstrap belongs in your provisioning pipeline (Terraform,
Pulumi, etc.) so step 1 never produces a key file that survives
the wrap step.

## Rotation — re-wrap, do not re-encrypt

Cloud KMS rotation in SKYCOT changes the *wrap* of the DEK, not
the DEK itself. Because the DEK is unchanged, SQLCipher pages do
not need re-encryption, so rotation is O(1) regardless of
database size:

1. Provision the new KMS key (and its IAM policy).
2. Wrap the existing DEK with the new key.
3. Atomically swap the wrapped DEK file.
4. Update `argo.toml`'s `key_source` to point at the new key.
5. Restart SKYCOT.

The cross-provider primitive [`argo_store::kms::rewrap_dek`]
performs steps 2-3 in one call:

```rust
let new_wrapped = argo_store::kms::rewrap_dek(
    &old_provider,
    &new_provider,
    &existing_wrapped,
).await?;
```

The plaintext DEK never leaves the function's stack frame. CLI
wiring for an `argo db rekey --to <new-kms-key>` rotation flow
that drives this primitive end-to-end lands in the BYOK
follow-up brief (`mission-byok-rekey-cli-wiring`); the v1 PR
ships the primitive + the parse/resolve wire surface so external
tooling can drive rotation today.

## Operational notes

- `key_source` strings accept the BYOK prefixes (`aws-kms:`,
  `gcp-cmek:`, `azure-kv:`) on every build. Stock builds without
  the matching `kms-{aws,gcp,azure}` Cargo feature surface a
  typed `ArgoError::Provider("...feature is off")` at resolve
  time — they do not silently fall back to plaintext.
- Bearer-token rotation is the operator's responsibility. AAD
  access tokens are short-lived (~1h); pair the bearer pointer
  with a sidecar that refreshes the file/env on a schedule.
- For wiremock-based testing in your CI, set the `endpoint=`
  query param to point at a localhost stub. SKYCOT will skip the
  default cloud endpoint and POST to the override instead. The
  same hook drives the `argo-store` unit tests under
  `crates/argo-store/src/kms_tests.rs`.
