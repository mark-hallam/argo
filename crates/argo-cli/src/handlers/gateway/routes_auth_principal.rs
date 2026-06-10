//! GatewayPrincipal parsing for authenticated gateway requests.
//!
//! Bearer auth proves the caller knows the token; this module carries
//! optional caller identity into gateway audit payloads without changing
//! the audit schema.

use super::middleware_session_verify::{decode_session_bearer, decode_session_cookie, DecodeOutcome};
use serde_json::Value;
use uuid::Uuid;

const HEADER: &str = "X-Argo-Principal";
const MAX_PRINCIPAL_LEN: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GatewayPrincipal {
    Worker(Uuid),
    Human(String),
    System(String),
    Anonymous,
}

impl GatewayPrincipal {
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

pub(super) fn parse_principal_header(req: &str) -> Result<GatewayPrincipal, &'static str> {
    let Some(raw) = principal_header_value(req) else {
        return Ok(GatewayPrincipal::Anonymous);
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
            .map(GatewayPrincipal::Worker)
            .map_err(|_| "bad principal"),
        "human" => Ok(GatewayPrincipal::Human(value.to_string())),
        "system" => Ok(GatewayPrincipal::System(value.to_string())),
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

/// Resolve the audit principal for a **session-authenticated** work
/// action (e.g. the manual "Run now"). Mirrors the WS1 JWT-first
/// precedence of `super::resolve_tenant::resolve_tenant_for_request`:
/// a signed-in operator's action attributes to THEIR server-resolved
/// session identity, and a client-supplied `X-Argo-Principal` header can
/// NEVER override it. The header stays as the fallback ONLY when there is
/// no decodable session, so `None` (→ `"anonymous"` at the call site)
/// survives for genuinely anonymous calls.
///
/// This is the honesty fix for the manual-run audit row: previously the
/// row read "triggered by anonymous" even though the operator was signed
/// in with a `__Host-session` JWT — the resolver only ever consulted the
/// (absent) header. See `fix-manual-run-audit-row-kind-status-and-principal`.
pub(super) fn session_principal_id_for_audit(req: &str) -> Option<String> {
    session_human_principal(req).or_else(|| principal_id_for_audit(req))
}

/// The `human:<email|sub>` principal carried by a valid session JWT
/// (`__Host-session` cookie preferred, then the forwarded
/// `Authorization: Bearer` session JWT), or `None` when the request
/// carries no decodable session. Uses the same signature+`exp` decode the
/// tenant resolver trusts; a stale or client-forged cookie only ever names
/// its OWN signed identity, so no principal escalation is possible here.
fn session_human_principal(req: &str) -> Option<String> {
    let claims = match decode_session_cookie(req) {
        DecodeOutcome::Decoded(c) => c,
        _ => match decode_session_bearer(req) {
            DecodeOutcome::Decoded(c) => c,
            _ => return None,
        },
    };
    let email = claims.email.trim();
    let who = if email.is_empty() {
        claims.sub.trim()
    } else {
        email
    };
    (!who.is_empty()).then(|| GatewayPrincipal::Human(who.to_string()).as_payload_value())
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
    use super::{parse_principal_header, principal_payload, GatewayPrincipal};
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
            GatewayPrincipal::Worker(id)
        );
    }

    #[test]
    fn human_principal_parses() {
        assert_eq!(
            parse_principal_header(&req(Some("human:mark@example.com"))).unwrap(),
            GatewayPrincipal::Human("mark@example.com".to_string())
        );
    }

    #[test]
    fn system_principal_parses() {
        assert_eq!(
            parse_principal_header(&req(Some("system:scheduler-1"))).unwrap(),
            GatewayPrincipal::System("scheduler-1".to_string())
        );
    }

    #[test]
    fn missing_principal_is_anonymous() {
        assert_eq!(
            parse_principal_header(&req(None)).unwrap(),
            GatewayPrincipal::Anonymous
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
