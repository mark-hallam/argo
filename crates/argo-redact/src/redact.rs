//! `redact_text` (string in → string out) and `redact_approval_row`
//! (JSON Value → redacted JSON Value), plus the audit-payload helper
//! the gateway uses to emit `approval.row.redacted` on every fenced
//! read.

use serde_json::{Map, Value};

use crate::patterns::{find_matches, Category};
use crate::policy::{PolicyMode, RedactionPolicy};

/// The four string fields on the `ApprovalRow` JSON shape that the
/// fence redacts. Order is stable so tests don't depend on hash
/// iteration.
const REDACTABLE_FIELDS: [&str; 4] = ["detail", "consequence_if_yes", "consequence_if_no", "reason"];

/// Wire `event_type` value the gateway uses when emitting the
/// per-row redaction audit signal. Carried as a tracing field on
/// target `audit::approval.row.redacted` until the AuditKind variant
/// lands in argo-audit (PR notes — argo-audit is out of may-touch).
pub const AUDIT_EVENT_TYPE: &str = "approval.row.redacted";

/// Outcome of [`redact_approval_row`]: the (possibly mutated) row,
/// the set of categories actually masked (de-duplicated, sorted), and
/// the grant id surfaced when an active grant was used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRedaction {
    /// The JSON row, with redactable fields rewritten where the
    /// policy demanded masking.
    pub row: Value,
    /// Categories that fired AND were masked. Empty when nothing
    /// matched OR when the policy was permissive enough to skip
    /// every match. Sorted by [`Category::label`] for determinism.
    pub categories_masked: Vec<Category>,
    /// Surrogate id of the grant the viewer used, if any. Plumbed
    /// through from the caller — `redact_approval_row` doesn't read
    /// the grant table itself.
    pub grant_id: Option<String>,
}

impl ApprovalRedaction {
    /// True when at least one category was masked. Drives whether
    /// the gateway emits the audit row at all.
    #[must_use]
    pub fn was_masked(&self) -> bool {
        !self.categories_masked.is_empty()
    }
}

/// Redact `text` per `policy`. Returns the rewritten text and the
/// list of categories actually masked.
///
/// Per-category overlap handling: matches are sorted by span-start and
/// any later match overlapping a prior committed mask is dropped.
/// This protects the common "phone-shape inside a card sequence"
/// case where the longer card match should win.
#[must_use]
pub fn redact_text(text: &str, policy: &RedactionPolicy) -> (String, Vec<Category>) {
    let mut hits = find_matches(text);
    if hits.is_empty() {
        return (text.to_string(), Vec::new());
    }
    // Stable order: by span start, then prefer longer matches (so a
    // card hit wins over an embedded phone-shape).
    hits.sort_by(|a, b| {
        a.span
            .start
            .cmp(&b.span.start)
            .then_with(|| b.span.end.cmp(&a.span.end))
    });
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut masked: Vec<Category> = Vec::new();
    for m in hits {
        if m.span.start < cursor {
            continue;
        }
        if policy.mode(m.category) != PolicyMode::Mask {
            continue;
        }
        if m.span.start > text.len() || m.span.end > text.len() {
            continue;
        }
        out.push_str(&text[cursor..m.span.start]);
        out.push_str(m.category.sentinel());
        cursor = m.span.end;
        if !masked.contains(&m.category) {
            masked.push(m.category);
        }
    }
    out.push_str(&text[cursor..]);
    masked.sort_by_key(|c| c.label());
    (out, masked)
}

/// Apply [`redact_text`] to every redactable string field on an
/// `ApprovalRow` JSON object. Non-object inputs and missing fields
/// are no-ops; non-string field values are skipped (the caller has
/// already serialized the row from `ActionRecord`, so the shape is
/// trusted, but defensive in case of schema drift).
///
/// `grant_id` is the surrogate id the gateway resolved for the
/// viewer (e.g. the first grant that contributed to a `Show` mode);
/// it threads through into [`audit_payload`] unchanged.
#[must_use]
pub fn redact_approval_row(
    row: Value,
    policy: &RedactionPolicy,
    grant_id: Option<String>,
) -> ApprovalRedaction {
    let mut categories_masked: Vec<Category> = Vec::new();
    let row = if let Value::Object(mut map) = row {
        for field in REDACTABLE_FIELDS {
            if let Some(text) = map.get(field).and_then(Value::as_str) {
                let (rewritten, hits) = redact_text(text, policy);
                if !hits.is_empty() {
                    for c in hits {
                        if !categories_masked.contains(&c) {
                            categories_masked.push(c);
                        }
                    }
                    map.insert(field.to_string(), Value::String(rewritten));
                }
            }
        }
        Value::Object(map)
    } else {
        row
    };
    categories_masked.sort_by_key(|c| c.label());
    ApprovalRedaction {
        row,
        categories_masked,
        grant_id,
    }
}

/// Build the audit payload JSON for `approval.row.redacted`. Caller
/// emits it via `tracing::info!(target: "audit::approval.row.redacted", …)`
/// or, once a dedicated `AuditKind` variant exists, via `AuditSink`.
///
/// Deliberately omits the redacted strings themselves — the audit
/// payload only carries the categories that were masked, not the
/// underlying values.
#[must_use]
pub fn audit_payload(
    actor_id: &str,
    approval_id: &str,
    redaction: &ApprovalRedaction,
) -> Value {
    let cats: Vec<&str> = redaction
        .categories_masked
        .iter()
        .map(|c| c.label())
        .collect();
    let mut obj = Map::new();
    obj.insert("event_type".to_string(), Value::String(AUDIT_EVENT_TYPE.to_string()));
    obj.insert("actor_id".to_string(), Value::String(actor_id.to_string()));
    obj.insert(
        "approval_id".to_string(),
        Value::String(approval_id.to_string()),
    );
    obj.insert(
        "categories_masked".to_string(),
        Value::Array(cats.into_iter().map(|c| Value::String(c.to_string())).collect()),
    );
    if let Some(gid) = redaction.grant_id.as_ref() {
        obj.insert("grant_id".to_string(), Value::String(gid.clone()));
    } else {
        obj.insert("grant_id".to_string(), Value::Null);
    }
    Value::Object(obj)
}
