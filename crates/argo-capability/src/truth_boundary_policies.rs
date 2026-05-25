//! TOML loader + validator for [`crate::truth_boundary::TruthBoundaryPolicy`]
//! and [`crate::truth_boundary::RefusalTemplate`] files.
//!
//! Two parallel pipelines:
//! - [`load_policy`] / [`load_policies_from_dir`] for policies.
//! - [`load_refusal_template`] / [`load_refusal_templates`] for templates.
//!
//! Both reject malformed input with [`argo_types::ArgoError::Schema`]
//! (per the workspace's `Schema = HTTP 400` convention), and both
//! refuse empty required fields with a clear message.

use crate::truth_boundary::{
    ChannelKind, ChannelScope, ClaimClass, ClaimDisposition, EscalationKind, EscalationTarget,
    RefusalTemplate, TruthBoundaryPolicy,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Deserialize;
use std::path::Path;
use uuid::Uuid;

/// Load a single [`TruthBoundaryPolicy`] from a TOML file.
///
/// Schema (see `policies/truth-boundary/skycot-public-pitch.toml`):
/// ```toml
/// id = "skycot-public-pitch"
/// tenant_id = "skycot"
///
/// [[claim_classes]]
/// name = "revenue"
/// disposition = "Forbid"
/// refusal_template_id = "defer-to-mark-financial"
/// channels = "AllChannels"
/// ```
///
/// `channels` is a string: `"AllChannels"`, `"Only:WebEmbed,LiveMeeting"`,
/// or `"Except:Rehearsal"`. Channel names match the [`ChannelKind`] enum.
///
/// `disposition` is `"Allow"`, `"Forbid"` (paired with
/// `refusal_template_id`), or `"Escalate"` (optionally paired with an
/// `[[claim_classes.escalate_to]]` inline table).
///
/// # Errors
/// Returns [`ArgoError::Io`] on read failure, [`ArgoError::Schema`] on
/// parse or validation failure.
pub fn load_policy(path: &Path) -> ArgoResult<TruthBoundaryPolicy> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Io(format!(
            "failed to read truth-boundary policy at {}: {e}",
            path.display()
        ))
    })?;
    parse_policy_str(&raw, path)
}

/// Load every `*.toml` file under `dir` as a [`TruthBoundaryPolicy`].
///
/// Returns the loaded policies in directory-entry order. A single bad
/// file aborts the load (matches the `default_roles` semantics —
/// boot-time config errors are not recoverable).
///
/// # Errors
/// Returns [`ArgoError::Io`] on directory-read failure, or whatever
/// [`load_policy`] returns for the first bad file.
pub fn load_policies_from_dir(dir: &Path) -> ArgoResult<Vec<TruthBoundaryPolicy>> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        ArgoError::Io(format!(
            "failed to read truth-boundary policy dir at {}: {e}",
            dir.display()
        ))
    })?;
    let mut out = Vec::new();
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    paths.sort();
    for path in paths {
        out.push(load_policy(&path)?);
    }
    Ok(out)
}

/// Load a single [`RefusalTemplate`] from a TOML file.
///
/// Schema:
/// ```toml
/// id = "defer-to-mark-financial"
/// variants = [
///   "I'll defer to Mark on financials.",
///   "That number's not public — happy to put you on a call with Mark.",
/// ]
/// ```
///
/// # Errors
/// Returns [`ArgoError::Io`] on read failure, [`ArgoError::Schema`] on
/// parse failure or empty `variants`.
pub fn load_refusal_template(path: &Path) -> ArgoResult<RefusalTemplate> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        ArgoError::Io(format!(
            "failed to read refusal template at {}: {e}",
            path.display()
        ))
    })?;
    parse_refusal_template_str(&raw, path)
}

/// Load every `*.toml` file under `dir` as a [`RefusalTemplate`].
///
/// # Errors
/// Returns [`ArgoError::Io`] on directory-read failure, or whatever
/// [`load_refusal_template`] returns for the first bad file.
pub fn load_refusal_templates(dir: &Path) -> ArgoResult<Vec<RefusalTemplate>> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        ArgoError::Io(format!(
            "failed to read refusal-template dir at {}: {e}",
            dir.display()
        ))
    })?;
    let mut out = Vec::new();
    let mut paths: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("toml"))
        .collect();
    paths.sort();
    for path in paths {
        out.push(load_refusal_template(&path)?);
    }
    Ok(out)
}

fn parse_policy_str(raw: &str, path: &Path) -> ArgoResult<TruthBoundaryPolicy> {
    let parsed: TomlPolicy = toml::from_str(raw).map_err(|e| {
        ArgoError::Schema(format!(
            "malformed truth-boundary policy at {}: {e}",
            path.display()
        ))
    })?;
    if parsed.id.is_empty() {
        return Err(ArgoError::Schema(format!(
            "truth-boundary policy at {} has empty id",
            path.display()
        )));
    }
    if parsed.tenant_id.is_empty() {
        return Err(ArgoError::Schema(format!(
            "truth-boundary policy '{}' has empty tenant_id",
            parsed.id
        )));
    }
    let tenant_uuid = Uuid::parse_str(&parsed.tenant_id).map_err(|e| {
        ArgoError::Schema(format!(
            "truth-boundary policy '{}' tenant_id '{}' is not a valid UUID: {e}",
            parsed.id, parsed.tenant_id
        ))
    })?;
    let mut classes = Vec::with_capacity(parsed.claim_classes.len());
    for cls in parsed.claim_classes {
        classes.push(cls.into_claim_class(&parsed.id)?);
    }
    Ok(TruthBoundaryPolicy {
        id: parsed.id,
        tenant_id: TenantId(tenant_uuid),
        claim_classes: classes,
    })
}

fn parse_refusal_template_str(raw: &str, path: &Path) -> ArgoResult<RefusalTemplate> {
    let parsed: RefusalTemplate = toml::from_str(raw).map_err(|e| {
        ArgoError::Schema(format!(
            "malformed refusal template at {}: {e}",
            path.display()
        ))
    })?;
    if parsed.id.is_empty() {
        return Err(ArgoError::Schema(format!(
            "refusal template at {} has empty id",
            path.display()
        )));
    }
    if parsed.variants.is_empty() {
        return Err(ArgoError::Schema(format!(
            "refusal template '{}' has empty variants — at least one is required",
            parsed.id
        )));
    }
    for (i, v) in parsed.variants.iter().enumerate() {
        if v.trim().is_empty() {
            return Err(ArgoError::Schema(format!(
                "refusal template '{}' has empty variant at index {i}",
                parsed.id
            )));
        }
    }
    Ok(parsed)
}

#[derive(Debug, Deserialize)]
struct TomlPolicy {
    id: String,
    tenant_id: String,
    #[serde(default)]
    claim_classes: Vec<TomlClaimClass>,
}

#[derive(Debug, Deserialize)]
struct TomlClaimClass {
    name: String,
    disposition: String,
    #[serde(default)]
    refusal_template_id: Option<String>,
    channels: String,
    #[serde(default)]
    escalate_to: Option<TomlEscalationTarget>,
}

#[derive(Debug, Deserialize)]
struct TomlEscalationTarget {
    kind: String,
    address: String,
}

impl TomlClaimClass {
    fn into_claim_class(self, policy_id: &str) -> ArgoResult<ClaimClass> {
        if self.name.is_empty() {
            return Err(ArgoError::Schema(format!(
                "policy '{policy_id}': claim_class has empty name"
            )));
        }
        let disposition = match self.disposition.as_str() {
            "Allow" => ClaimDisposition::Allow,
            "Forbid" => {
                let tpl = self.refusal_template_id.unwrap_or_default();
                if tpl.is_empty() {
                    return Err(ArgoError::Schema(format!(
                        "policy '{policy_id}': claim_class '{}' disposition=Forbid requires refusal_template_id",
                        self.name
                    )));
                }
                ClaimDisposition::Forbid {
                    refusal_template_id: tpl,
                }
            }
            "Escalate" => ClaimDisposition::Escalate,
            other => {
                return Err(ArgoError::Schema(format!(
                    "policy '{policy_id}': claim_class '{}' has unknown disposition '{other}' (expected Allow | Forbid | Escalate)",
                    self.name
                )));
            }
        };
        let channels = parse_channels(&self.channels, policy_id, &self.name)?;
        let escalate_to = self
            .escalate_to
            .map(|t| t.into_target(policy_id, &self.name))
            .transpose()?;
        Ok(ClaimClass {
            name: self.name,
            disposition,
            channels,
            escalate_to,
        })
    }
}

impl TomlEscalationTarget {
    fn into_target(self, policy_id: &str, class_name: &str) -> ArgoResult<EscalationTarget> {
        let kind = match self.kind.as_str() {
            "Approver" => EscalationKind::Approver,
            "Human" => EscalationKind::Human,
            "Webhook" => EscalationKind::Webhook,
            other => {
                return Err(ArgoError::Schema(format!(
                    "policy '{policy_id}': claim_class '{class_name}' escalate_to.kind '{other}' (expected Approver | Human | Webhook)"
                )));
            }
        };
        if self.address.trim().is_empty() {
            return Err(ArgoError::Schema(format!(
                "policy '{policy_id}': claim_class '{class_name}' escalate_to.address is empty"
            )));
        }
        Ok(EscalationTarget {
            kind,
            address: self.address,
        })
    }
}

fn parse_channels(raw: &str, policy_id: &str, class_name: &str) -> ArgoResult<ChannelScope> {
    let trimmed = raw.trim();
    if trimmed == "AllChannels" {
        return Ok(ChannelScope::AllChannels);
    }
    let (op, rest) = if let Some(r) = trimmed.strip_prefix("Only:") {
        ("Only", r)
    } else if let Some(r) = trimmed.strip_prefix("Except:") {
        ("Except", r)
    } else {
        return Err(ArgoError::Schema(format!(
            "policy '{policy_id}': claim_class '{class_name}' channels '{raw}' must be 'AllChannels', 'Only:<list>', or 'Except:<list>'"
        )));
    };
    let mut list = Vec::new();
    for part in rest.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        list.push(parse_channel_kind(p, policy_id, class_name)?);
    }
    if list.is_empty() {
        return Err(ArgoError::Schema(format!(
            "policy '{policy_id}': claim_class '{class_name}' channels '{op}:' has empty list"
        )));
    }
    Ok(match op {
        "Only" => ChannelScope::Only(list),
        _ => ChannelScope::Except(list),
    })
}

fn parse_channel_kind(raw: &str, policy_id: &str, class_name: &str) -> ArgoResult<ChannelKind> {
    match raw {
        "Rehearsal" => Ok(ChannelKind::Rehearsal),
        "WebEmbed" => Ok(ChannelKind::WebEmbed),
        "LiveMeeting" => Ok(ChannelKind::LiveMeeting),
        "Chat" => Ok(ChannelKind::Chat),
        other => Err(ArgoError::Schema(format!(
            "policy '{policy_id}': claim_class '{class_name}' unknown channel '{other}' (expected Rehearsal | WebEmbed | LiveMeeting | Chat)"
        ))),
    }
}
