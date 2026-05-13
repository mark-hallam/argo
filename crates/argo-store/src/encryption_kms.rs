//! Cloud-KMS helpers split out from [`crate::encryption`] under the
//! 500-LOC per-file ceiling (Stream `mission-byok-enterprise-tenants`).
//!
//! Holds the spec parser and the per-provider client-build glue. The
//! enum + sync resolve surface stay in [`crate::encryption`] so the
//! existing import path `argo_store::encryption::KeySource` is
//! unchanged.

use argo_types::ArgoError;

#[cfg(feature = "kms")]
use crate::encryption::KeySource;
use crate::encryption::{KmsKeyRef, WrappedDekSource};
use crate::StoreResult as Result;

/// Body of [`KeySource::parse`] for the three cloud-KMS prefixes.
/// Splits the spec into `key_id` + URL-style params and validates
/// the required `wrapped` pointer. Pure string parsing — no I/O.
///
/// # Errors
/// Returns [`ArgoError::Schema`] for an empty key id, malformed
/// query param, unknown param key, or missing `wrapped` pointer.
pub(crate) fn parse_kms_spec(body: &str, kind: &str) -> Result<KmsKeyRef> {
    let (head, query) = match body.split_once('?') {
        Some((h, q)) => (h.trim(), q),
        None => (body.trim(), ""),
    };
    if head.is_empty() {
        return Err(ArgoError::Schema(format!(
            "{kind} key_source needs a key id after '{kind}:'"
        )));
    }
    let mut region = String::new();
    let mut wrapped_spec: Option<&str> = None;
    let mut bearer_spec: Option<&str> = None;
    let mut endpoint = String::new();
    if !query.is_empty() {
        for kv in query.split('&') {
            if kv.is_empty() {
                continue;
            }
            let (k, v) = kv.split_once('=').ok_or_else(|| {
                ArgoError::Schema(format!("{kind} key_source param '{kv}' missing '='"))
            })?;
            match k {
                "region" => region = v.to_string(),
                "wrapped" => wrapped_spec = Some(v),
                "bearer" => bearer_spec = Some(v),
                "endpoint" => endpoint = v.to_string(),
                other => {
                    return Err(ArgoError::Schema(format!(
                        "{kind} key_source: unknown param '{other}'"
                    )));
                }
            }
        }
    }
    let wrapped_spec = wrapped_spec.ok_or_else(|| {
        ArgoError::Schema(format!(
            "{kind} key_source requires &wrapped=env:VAR or &wrapped=file:/path"
        ))
    })?;
    let wrapped_dek = WrappedDekSource::parse(wrapped_spec)?;
    let bearer = match bearer_spec {
        Some(s) => Some(WrappedDekSource::parse(s)?),
        None => None,
    };
    Ok(KmsKeyRef {
        key_id: head.to_string(),
        region,
        wrapped_dek,
        bearer,
        endpoint_override: endpoint,
    })
}

/// Resolve a cloud-KMS [`KeySource`] to the unwrapped DEK as hex
/// (compatible with `PRAGMA key`). Dispatches to the per-provider
/// client. Plaintext DEK never leaves this stack frame.
///
/// # Errors
/// Per-provider HTTP / IAM / parse errors as [`ArgoError::Provider`].
#[cfg(feature = "kms")]
pub(crate) async fn resolve_kms(
    source: &KeySource,
    keyref: &KmsKeyRef,
) -> Result<Option<String>> {
    let plaintext = unwrap_dek_for(source, keyref).await?;
    let mut hex = String::with_capacity(64);
    for byte in &plaintext {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(Some(hex))
}

/// Build the right provider client and unwrap the wrapped DEK
/// referenced by [`KmsKeyRef::wrapped_dek`]. Returns the 32 raw
/// plaintext bytes.
#[cfg(feature = "kms")]
async fn unwrap_dek_for(source: &KeySource, keyref: &KmsKeyRef) -> Result<Vec<u8>> {
    use crate::kms::KmsProvider;
    let wrapped = keyref.wrapped_dek.resolve_bytes()?;
    let client: Box<dyn KmsProvider> = match source {
        #[cfg(feature = "kms-aws")]
        KeySource::AwsKms(_) => Box::new(build_aws_client(keyref)?),
        #[cfg(not(feature = "kms-aws"))]
        KeySource::AwsKms(_) => {
            return Err(ArgoError::Provider(
                "aws-kms key_source configured but `kms-aws` Cargo feature is off".into(),
            ));
        }
        #[cfg(feature = "kms-gcp")]
        KeySource::GcpCmek(_) => Box::new(build_gcp_client(keyref)?),
        #[cfg(not(feature = "kms-gcp"))]
        KeySource::GcpCmek(_) => {
            return Err(ArgoError::Provider(
                "gcp-cmek key_source configured but `kms-gcp` Cargo feature is off".into(),
            ));
        }
        #[cfg(feature = "kms-azure")]
        KeySource::AzureKeyVault(_) => Box::new(build_azure_client(keyref)?),
        #[cfg(not(feature = "kms-azure"))]
        KeySource::AzureKeyVault(_) => {
            return Err(ArgoError::Provider(
                "azure-kv key_source configured but `kms-azure` Cargo feature is off".into(),
            ));
        }
        // Local variants don't take the KMS path.
        _ => unreachable!("unwrap_dek_for called on local KeySource variant"),
    };
    client.unwrap_dek(&wrapped).await
}

#[cfg(feature = "kms-aws")]
fn build_aws_client(r: &KmsKeyRef) -> Result<crate::kms::aws::AwsKmsClient> {
    let bearer = match &r.bearer {
        Some(src) => Some(src.resolve_string()?),
        None => None,
    };
    if r.endpoint_override.is_empty() {
        crate::kms::aws::AwsKmsClient::new(r.key_id.clone(), r.region.clone(), bearer)
    } else {
        crate::kms::aws::AwsKmsClient::with_endpoint(
            r.key_id.clone(),
            r.region.clone(),
            bearer,
            r.endpoint_override.clone(),
        )
    }
}

#[cfg(feature = "kms-gcp")]
fn build_gcp_client(r: &KmsKeyRef) -> Result<crate::kms::gcp::GcpCmekClient> {
    let parts: Vec<&str> = r.key_id.splitn(4, '/').collect();
    if parts.len() != 4 {
        return Err(ArgoError::Schema(format!(
            "gcp-cmek key_id must be 'project/location/ring/key'; got '{}'",
            r.key_id
        )));
    }
    let bearer = match &r.bearer {
        Some(src) => Some(src.resolve_string()?),
        None => None,
    };
    if r.endpoint_override.is_empty() {
        crate::kms::gcp::GcpCmekClient::new(
            parts[0].to_string(),
            parts[2].to_string(),
            parts[3].to_string(),
            parts[1].to_string(),
            bearer,
        )
    } else {
        crate::kms::gcp::GcpCmekClient::with_endpoint(
            parts[0].to_string(),
            parts[2].to_string(),
            parts[3].to_string(),
            parts[1].to_string(),
            bearer,
            r.endpoint_override.clone(),
        )
    }
}

#[cfg(feature = "kms-azure")]
fn build_azure_client(r: &KmsKeyRef) -> Result<crate::kms::azure::AzureKeyVaultClient> {
    let (vault_url, key_name) = r.key_id.split_once('#').ok_or_else(|| {
        ArgoError::Schema(format!(
            "azure-kv key_id must be '<vault_url>#<key_name>'; got '{}'",
            r.key_id
        ))
    })?;
    let bearer = match &r.bearer {
        Some(src) => Some(src.resolve_string()?),
        None => None,
    };
    if r.endpoint_override.is_empty() {
        crate::kms::azure::AzureKeyVaultClient::new(
            vault_url.to_string(),
            key_name.to_string(),
            bearer,
        )
    } else {
        crate::kms::azure::AzureKeyVaultClient::with_endpoint(
            vault_url.to_string(),
            key_name.to_string(),
            bearer,
            r.endpoint_override.clone(),
        )
    }
}
