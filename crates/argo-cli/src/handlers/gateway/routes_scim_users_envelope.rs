//! Envelope helpers + query/body parsers for `routes_scim_users`.
//!
//! Split out from `routes_scim_users.rs` to keep the dispatcher under
//! the 500-LOC budget. Three concerns live here:
//!
//! - RFC 7644 response envelope builders (`user_to_envelope`,
//!   `list_envelope`, `scim_error`, `scim_error_with_type`).
//! - List-query parsing (`parse_list_query` + the
//!   `userName eq "..."` filter sub-grammar).
//! - PATCH body interpretation (`build_patch` + the per-op helpers
//!   that turn SCIM PatchOp envelopes or plain-attribute bodies into
//!   a [`ScimUserPatch`]).
//!
//! Pure functions only — no I/O, no shared state. Tests drive these
//! directly via the dispatcher's integration tests.
use argo_store::scim_user::{ScimUserPatch, ScimUserRow};
use chrono::Utc;
use serde_json::{json, Value};

pub(super) const USER_SCHEMA: &str = "urn:ietf:params:scim:schemas:core:2.0:User";
pub(super) const LIST_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:ListResponse";
pub(super) const PATCH_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:PatchOp";
pub(super) const ERROR_SCHEMA: &str = "urn:ietf:params:scim:api:messages:2.0:Error";

const DEFAULT_COUNT: i64 = 100;
const MAX_COUNT: i64 = 1000;

/// Parsed `?filter=…&startIndex=…&count=…` triple.
pub(super) struct ListQuery {
    pub filter_user_name: Option<String>,
    pub start_index: i64,
    pub count: i64,
}

/// Parse the request path's query string. Only `userName eq "..."`
/// is recognised as a filter; unknown keys are ignored.
pub(super) fn parse_list_query(path: &str) -> Result<ListQuery, String> {
    let Some((_, qs)) = path.split_once('?') else {
        return Ok(ListQuery {
            filter_user_name: None,
            start_index: 1,
            count: DEFAULT_COUNT,
        });
    };
    let mut out = ListQuery {
        filter_user_name: None,
        start_index: 1,
        count: DEFAULT_COUNT,
    };
    for kv in qs.split('&') {
        let Some((key, raw)) = kv.split_once('=') else {
            continue;
        };
        let value = url_decode(raw);
        match key {
            "filter" => out.filter_user_name = Some(parse_user_name_filter(&value)?),
            "startIndex" => {
                out.start_index = value
                    .parse::<i64>()
                    .map_err(|_| format!("invalid startIndex '{value}'"))?;
                if out.start_index < 1 {
                    return Err("startIndex must be >= 1".into());
                }
            }
            "count" => {
                out.count = value
                    .parse::<i64>()
                    .map_err(|_| format!("invalid count '{value}'"))?;
                if !(0..=MAX_COUNT).contains(&out.count) {
                    return Err(format!("count must be in [0, {MAX_COUNT}]"));
                }
            }
            _ => {}
        }
    }
    Ok(out)
}

/// Accept `userName eq "value"` (case-insensitive attr + op).
fn parse_user_name_filter(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("username ") {
        return Err(format!(
            "only `userName eq` filters are supported, got: '{trimmed}'"
        ));
    }
    let after_attr = trimmed
        .get("username ".len()..)
        .unwrap_or("")
        .trim_start();
    let lower_after = after_attr.to_ascii_lowercase();
    let rest = lower_after
        .strip_prefix("eq ")
        .ok_or_else(|| format!("only `eq` operator supported, got: '{trimmed}'"))?;
    let needle_raw = after_attr
        .get(after_attr.len() - rest.len()..)
        .unwrap_or("")
        .trim();
    needle_raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .map(str::to_string)
        .ok_or_else(|| format!("filter value must be a quoted string, got: '{needle_raw}'"))
}

/// Extract a SCIM `emails` array, accepting both the canonical
/// `[{ "value": "..." }]` shape and a plain `["..."]` array.
pub(super) fn extract_emails(value: Option<&Value>) -> Vec<String> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| {
            v.as_str()
                .map(str::to_string)
                .or_else(|| v.get("value").and_then(Value::as_str).map(str::to_string))
        })
        .collect()
}

/// Build a [`ScimUserPatch`] from either a PatchOp envelope or a
/// plain-attribute body. The PatchOp form is what Azure AD and
/// JumpCloud send; the plain form is what Okta uses in some profile-
/// update scenarios. Supporting both keeps interop wide.
pub(super) fn build_patch(body: &Value) -> Result<ScimUserPatch, String> {
    let mut patch = ScimUserPatch {
        updated_at: Utc::now(),
        ..ScimUserPatch::default()
    };
    let is_envelope = body
        .get("schemas")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().any(|v| v.as_str() == Some(PATCH_SCHEMA)))
        .unwrap_or(false);
    if is_envelope {
        let ops = body
            .get("Operations")
            .and_then(Value::as_array)
            .ok_or_else(|| "`Operations` array missing".to_string())?;
        for op in ops {
            apply_operation(op, &mut patch)?;
        }
    } else {
        apply_plain_attrs(body, &mut patch);
    }
    Ok(patch)
}

/// True when at least one mutable field would change. The dispatcher
/// short-circuits no-op PATCHes by re-reading the existing row.
pub(super) fn patch_changes_anything(p: &ScimUserPatch) -> bool {
    p.active.is_some()
        || p.display_name.is_some()
        || p.emails.is_some()
        || p.external_id.is_some()
}

fn apply_operation(op: &Value, patch: &mut ScimUserPatch) -> Result<(), String> {
    let action = op
        .get("op")
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| "Operation missing `op`".to_string())?;
    if action == "remove" {
        if let Some(path) = op.get("path").and_then(Value::as_str) {
            apply_remove(path, patch);
        }
        return Ok(());
    }
    if action != "add" && action != "replace" {
        return Err(format!("unsupported op '{action}'"));
    }
    let value = op.get("value");
    if let Some(path) = op.get("path").and_then(Value::as_str) {
        apply_path_value(path, value, patch);
    } else if let Some(obj) = value.and_then(Value::as_object) {
        for (k, v) in obj {
            apply_path_value(k, Some(v), patch);
        }
    }
    Ok(())
}

fn apply_remove(path: &str, patch: &mut ScimUserPatch) {
    match path {
        "active" => patch.active = Some(false),
        "displayName" => patch.display_name = Some(String::new()),
        "emails" => patch.emails = Some(Vec::new()),
        "externalId" => patch.external_id = Some(String::new()),
        _ => {}
    }
}

fn apply_path_value(path: &str, value: Option<&Value>, patch: &mut ScimUserPatch) {
    match path {
        "active" => {
            if let Some(b) = value.and_then(Value::as_bool) {
                patch.active = Some(b);
            }
        }
        "displayName" => {
            if let Some(s) = value.and_then(Value::as_str) {
                patch.display_name = Some(s.to_string());
            }
        }
        "externalId" => {
            if let Some(s) = value.and_then(Value::as_str) {
                patch.external_id = Some(s.to_string());
            }
        }
        "emails" => {
            patch.emails = Some(extract_emails(value));
        }
        _ => {}
    }
}

fn apply_plain_attrs(body: &Value, patch: &mut ScimUserPatch) {
    if let Some(b) = body.get("active").and_then(Value::as_bool) {
        patch.active = Some(b);
    }
    if let Some(s) = body.get("displayName").and_then(Value::as_str) {
        patch.display_name = Some(s.to_string());
    }
    if let Some(s) = body.get("externalId").and_then(Value::as_str) {
        patch.external_id = Some(s.to_string());
    }
    if let Some(v) = body.get("emails") {
        patch.emails = Some(extract_emails(Some(v)));
    }
}

/// Render a SCIM User row as a RFC 7644 envelope JSON value.
pub(super) fn user_to_envelope(row: &ScimUserRow) -> Value {
    let emails: Vec<Value> = row
        .emails
        .iter()
        .map(|addr| json!({ "value": addr, "primary": false }))
        .collect();
    json!({
        "schemas": [USER_SCHEMA],
        "id": row.id.to_string(),
        "externalId": row.external_id,
        "userName": row.user_name,
        "displayName": row.display_name,
        "emails": emails,
        "active": row.active,
        "meta": {
            "resourceType": "User",
            "created": row.created_at.to_rfc3339(),
            "lastModified": row.updated_at.to_rfc3339(),
            "location": format!("/scim/v2/Users/{}", row.id),
        }
    })
}

/// Render a SCIM ListResponse envelope.
pub(super) fn list_envelope(
    items: Vec<Value>,
    total: i64,
    start_index: i64,
    returned: i64,
) -> Value {
    json!({
        "schemas": [LIST_SCHEMA],
        "totalResults": total,
        "startIndex": start_index,
        "itemsPerPage": returned,
        "Resources": items,
    })
}

/// Helper: 200 OK with a JSON body.
pub(super) fn ok_json(value: Value) -> (&'static str, String) {
    ("200 OK", value.to_string())
}

/// Build an RFC 7644 §3.12 error envelope without a `scimType` field.
pub(super) fn scim_error(status: u16, detail: &str) -> (&'static str, String) {
    scim_error_with_type(status, "", detail)
}

/// Build an RFC 7644 §3.12 error envelope with a `scimType` (e.g.
/// `uniqueness`, `mutability`).
pub(super) fn scim_error_with_type(
    status: u16,
    scim_type: &str,
    detail: &str,
) -> (&'static str, String) {
    let status_line = match status {
        400 => "400 Bad Request",
        401 => "401 Unauthorized",
        403 => "403 Forbidden",
        404 => "404 Not Found",
        405 => "405 Method Not Allowed",
        409 => "409 Conflict",
        500 => "500 Internal Server Error",
        _ => "500 Internal Server Error",
    };
    let mut body = json!({
        "schemas": [ERROR_SCHEMA],
        "status": status.to_string(),
        "detail": detail,
    });
    if !scim_type.is_empty() {
        body["scimType"] = Value::String(scim_type.to_string());
    }
    (status_line, body.to_string())
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'+' {
            out.push(' ');
            i += 1;
        } else if b == b'%' && i + 2 < bytes.len() {
            match (hex_digit(bytes[i + 1]), hex_digit(bytes[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(char::from((h << 4) | l));
                    i += 3;
                }
                _ => {
                    out.push(b as char);
                    i += 1;
                }
            }
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
