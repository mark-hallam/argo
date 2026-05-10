//! Principal parsing for authenticated gateway requests.
//!
//! Bearer auth proves the caller knows the token; this module carries
//! optional caller identity into gateway audit payloads without changing
//! the audit schema.

use serde_json::Value;
use uuid::Uuid;

const HEADER: &str = "X-Argo-Principal";
const MAX_PRINCIPAL_LEN: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Principal {
    Worker(Uuid),
    Human(String),
    System(String),
    Anonymous,
}

impl Principal {
    fn as_payload_value(&self) -> String {
        match self {
            Self::Worker(id) => format!("worker:{id}"),
            Self::Human(name) => format!("human:{name}"),
            Self::System(name) => format!("system:{name}"),
            Self::Anonymous => "anonymous".to_string(),
        }
    }

    /// Wire form for the `audit_record.principal_id` column
    /// (`mvp-audit-principal-stamp`). `None` for anonymous calls so
    /// the column lands NULL — distinct from `"anonymous"` which
    /// would imply a resolved-but-unauthenticated principal.
    fn audit_id(&self) -> Option<String> {
        match self {
            Self::Anonymous => None,
            other => Some(other.as_payload_value()),
        }
    }
}

pub(super) fn parse_principal_header(req: &str) -> Result<Principal, &'static str> {
    let Some(raw) = principal_header_value(req) else {
        return Ok(Principal::Anonymous);
    };
    if raw.is_empty() || raw.len() > MAX_PRINCIPAL_LEN {
        return Err("bad principal");
    }
    let Some((kind, value)) = raw.split_once(':') else {
        return Err("bad principal");
    };
    if value.is_empty() || !is_safe_principal_value(value) {
        return Err("bad principal");
    }
    match kind {
        "worker" => Uuid::parse_str(value)
            .map(Principal::Worker)
            .map_err(|_| "bad principal"),
        "human" => Ok(Principal::Human(value.to_string())),
        "system" => Ok(Principal::System(value.to_string())),
        _ => Err("bad principal"),
    }
}

pub(super) fn principal_payload(req: &str, mut payload: Value) -> Result<Value, &'static str> {
    let principal = parse_principal_header(req)?.as_payload_value();
    let Some(obj) = payload.as_object_mut() else {
        return Err("bad principal");
    };
    obj.insert("principal".to_string(), Value::String(principal));
    Ok(payload)
}

/// Resolve the `audit_record.principal_id` column value for `req`
/// (`mvp-audit-principal-stamp`). Returns `None` when the request
/// has no `X-Argo-Principal` header (or the header is malformed —
/// the gateway dispatcher already short-circuits malformed values
/// to 400 before calling handlers, so this returns `None` only on
/// the anonymous path in practice).
pub(super) fn principal_id_for_audit(req: &str) -> Option<String> {
    parse_principal_header(req).ok().and_then(|p| p.audit_id())
}

fn principal_header_value(req: &str) -> Option<&str> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case(HEADER).then(|| value.trim())
    })
}

fn is_safe_principal_value(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_graphic() && b != b':' && b != b'\\')
}

#[cfg(test)]
mod tests {
    use super::{Principal, parse_principal_header, principal_payload};
    use serde_json::json;
    use uuid::Uuid;

    fn req(value: Option<&str>) -> String {
        let mut req = String::from("POST /sessions/demo/approve HTTP/1.1\r\n");
        if let Some(value) = value {
            req.push_str(&format!("X-Argo-Principal: {value}\r\n"));
        }
        req.push_str("\r\n{}");
        req
    }

    #[test]
    fn worker_principal_parses() {
        let id = Uuid::parse_str("018f55b7-89ab-7cde-8123-456789abcdef").unwrap();
        assert_eq!(
            parse_principal_header(&req(Some(&format!("worker:{id}")))).unwrap(),
            Principal::Worker(id)
        );
    }

    #[test]
    fn human_principal_parses() {
        assert_eq!(
            parse_principal_header(&req(Some("human:mark@example.com"))).unwrap(),
            Principal::Human("mark@example.com".to_string())
        );
    }

    #[test]
    fn system_principal_parses() {
        assert_eq!(
            parse_principal_header(&req(Some("system:scheduler-1"))).unwrap(),
            Principal::System("scheduler-1".to_string())
        );
    }

    #[test]
    fn missing_principal_is_anonymous() {
        assert_eq!(
            parse_principal_header(&req(None)).unwrap(),
            Principal::Anonymous
        );
    }

    #[test]
    fn malformed_principals_are_rejected() {
        for value in [
            "",
            "worker:not-a-uuid",
            "human:",
            "system:",
            "operator:mark",
            "human:mark smith",
            "human:mark\\smith",
        ] {
            assert!(
                parse_principal_header(&req(Some(value))).is_err(),
                "{value}"
            );
        }
    }

    #[test]
    fn oversized_principal_is_rejected() {
        let long = format!("human:{}", "a".repeat(300));
        assert!(parse_principal_header(&req(Some(&long))).is_err());
    }

    #[test]
    fn principal_payload_inserts_string_value() {
        let payload = principal_payload(&req(Some("human:mark")), json!({"tool":"x"})).unwrap();
        assert_eq!(payload["principal"], "human:mark");
        assert_eq!(payload["tool"], "x");
    }
}
