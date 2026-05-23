//! `redact_text` (string in → string out) and `redact_approval_row`
//! (JSON Value → redacted JSON Value), plus the audit-payload helper
//! the gateway uses to emit `approval.row.redacted` on every fenced
//! read.

use std::ops::Range;

use serde_json::{Map, Value};

use crate::canonicalize::{canonicalize_for_scan, decode_base64_token, looks_like_base64};
use crate::patterns::{find_matches, Category, PatternMatch};
use crate::policy::{PolicyMode, RedactionPolicy};

/// The four string fields on the `ApprovalRow` JSON shape that the
/// fence redacts. Order is stable so tests don't depend on hash
/// iteration.
const REDACTABLE_FIELDS: [&str; 4] = [
    "detail",
    "consequence_if_yes",
    "consequence_if_no",
    "reason",
];

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
/// Stage-1 promotion: the input is run through a canonicalisation
/// pre-pass (NFKC + URL-decode + HTML-entity decode + zero-width /
/// soft-hyphen strip) before the regex layer, and base64-shaped tokens
/// are decoded and re-scanned. Match spans are projected back to
/// original byte positions so the masked output preserves
/// un-canonicalised text everywhere a sentinel doesn't replace a hit.
///
/// Per-category overlap handling: matches are sorted by span-start and
/// any later match overlapping a prior committed mask is dropped.
/// This protects the common "phone-shape inside a card sequence"
/// case where the longer card match should win.
#[must_use]
pub fn redact_text(text: &str, policy: &RedactionPolicy) -> (String, Vec<Category>) {
    redact_text_inner(text, policy, 0)
}

/// `depth` bounds recursive base64 decode-and-rescan so a pathological
/// nested-base64 payload can't spin forever.
fn redact_text_inner(text: &str, policy: &RedactionPolicy, depth: u8) -> (String, Vec<Category>) {
    let canonical = canonicalize_for_scan(text);
    let mut hits = find_matches(&canonical.text);
    if depth == 0 {
        hits.extend(base64_rescan_hits(&canonical.text, policy));
    }
    let projected: Vec<PatternMatch> = hits
        .into_iter()
        .map(|m| {
            let orig = canonical.original_span(m.span.clone());
            PatternMatch {
                category: m.category,
                span: orig,
            }
        })
        .collect();
    apply_masks(text, projected, policy)
}

fn apply_masks(
    text: &str,
    mut hits: Vec<PatternMatch>,
    policy: &RedactionPolicy,
) -> (String, Vec<Category>) {
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
        if m.span.start >= m.span.end {
            continue;
        }
        if policy.mode(m.category) != PolicyMode::Mask {
            continue;
        }
        if m.span.start > text.len() || m.span.end > text.len() {
            continue;
        }
        if !is_char_boundary_pair(text, m.span.start, m.span.end) {
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

fn is_char_boundary_pair(text: &str, start: usize, end: usize) -> bool {
    text.is_char_boundary(start) && text.is_char_boundary(end)
}

/// Scan `canonical_text` for base64-shaped tokens; for each token that
/// decodes to UTF-8 AND whose decoded text contains PII per `policy`,
/// emit a [`PatternMatch`] covering the token's canonical span. The
/// emitted category is whichever the decoded text matched first; the
/// caller projects the canonical span back to the original input.
fn base64_rescan_hits(canonical_text: &str, policy: &RedactionPolicy) -> Vec<PatternMatch> {
    let mut out = Vec::new();
    for span in find_base64_token_spans(canonical_text) {
        let token = &canonical_text[span.clone()];
        let Some(decoded) = decode_base64_token(token) else {
            continue;
        };
        let (_, cats) = redact_text_inner(&decoded, policy, 1);
        if let Some(cat) = cats.into_iter().next() {
            out.push(PatternMatch {
                category: cat,
                span,
            });
        }
    }
    out
}

/// Return byte spans in `text` for every maximal run that
/// [`looks_like_base64`] accepts. Whitespace and surrounding ASCII
/// punctuation bound the runs; we deliberately only scan tokens, not
/// embedded substrings, so the regex engine doesn't re-decode random
/// alphanumeric prose.
fn find_base64_token_spans(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if is_base64_alphabet(bytes[i]) {
            let start = i;
            while i < bytes.len() && is_base64_alphabet(bytes[i]) {
                i += 1;
            }
            let token = &text[start..i];
            if looks_like_base64(token) {
                out.push(start..i);
            }
        } else {
            i += 1;
        }
    }
    out
}

fn is_base64_alphabet(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=')
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
pub fn audit_payload(actor_id: &str, approval_id: &str, redaction: &ApprovalRedaction) -> Value {
    let cats: Vec<&str> = redaction
        .categories_masked
        .iter()
        .map(|c| c.label())
        .collect();
    let mut obj = Map::new();
    obj.insert(
        "event_type".to_string(),
        Value::String(AUDIT_EVENT_TYPE.to_string()),
    );
    obj.insert("actor_id".to_string(), Value::String(actor_id.to_string()));
    obj.insert(
        "approval_id".to_string(),
        Value::String(approval_id.to_string()),
    );
    obj.insert(
        "categories_masked".to_string(),
        Value::Array(
            cats.into_iter()
                .map(|c| Value::String(c.to_string()))
                .collect(),
        ),
    );
    if let Some(gid) = redaction.grant_id.as_ref() {
        obj.insert("grant_id".to_string(), Value::String(gid.clone()));
    } else {
        obj.insert("grant_id".to_string(), Value::Null);
    }
    Value::Object(obj)
}
