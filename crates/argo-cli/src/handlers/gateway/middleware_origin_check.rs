//! Origin-allowlist gate for state-changing requests
//! (Stream `claude-rebase-pr-1835-cookies`).
//!
//! Browsers send `Origin` on every cross-origin XHR + on every
//! same-origin POST/PATCH/DELETE; rejecting requests with a foreign
//! `Origin` blocks the classic CSRF + cross-site-XHR attack surfaces
//! before the request body is parsed. Non-browser callers (CLI,
//! curl, internal hosts) send no `Origin`, so the absence of the
//! header is treated as accept — the operator-bearer gate already
//! defends those callers.
//!
//! Allowlist is env-driven via `ARGO_TRUSTED_ORIGINS` (comma-
//! separated). Dev default includes `localhost` + `127.0.0.1` on a
//! handful of common SPA dev ports.

/// Default origins accepted when `ARGO_TRUSTED_ORIGINS` is unset.
/// Production deploys set the env explicitly to
/// `https://app.skycot.com,https://api.skycot.com`; this default
/// keeps the local dev loop working without configuration.
const DEFAULT_TRUSTED_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:3001",
    "http://localhost:8787",
    "http://127.0.0.1:3000",
    "http://127.0.0.1:3001",
    "http://127.0.0.1:8787",
];

const REJECT_BODY: &str =
    r#"{"error":"origin_rejected","reason":"Origin header not in ARGO_TRUSTED_ORIGINS allowlist"}"#;

/// Returns `Some(403)` when the request carries an `Origin` header
/// that is not in the configured allowlist and the method is one of
/// the brief's mutating verbs (POST / PATCH / DELETE). Returns
/// `None` for safe methods, requests without an `Origin` header,
/// and allow-listed origins.
pub(super) fn check_origin(req: &str) -> Option<(&'static str, String)> {
    if !is_mutating_request(req) {
        return None;
    }
    let origin = origin_header(req)?;
    let trusted = trusted_origins();
    if trusted.iter().any(|allowed| allowed == &origin) {
        return None;
    }
    Some(("403 Forbidden", REJECT_BODY.to_string()))
}

fn is_mutating_request(req: &str) -> bool {
    req.starts_with("POST ") || req.starts_with("PATCH ") || req.starts_with("DELETE ")
}

fn origin_header(req: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("Origin")
            .then(|| value.trim().to_string())
    })
}

fn trusted_origins() -> Vec<String> {
    match std::env::var("ARGO_TRUSTED_ORIGINS") {
        Ok(raw) if !raw.is_empty() => raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => DEFAULT_TRUSTED_ORIGINS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // All tests in this module mutate `ARGO_TRUSTED_ORIGINS`; serialize
    // them through a process-wide mutex so cargo's parallel runner
    // doesn't observe an env-var write from a sibling test.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(value: Option<&str>) -> Self {
            let prev = std::env::var("ARGO_TRUSTED_ORIGINS").ok();
            match value {
                Some(v) => std::env::set_var("ARGO_TRUSTED_ORIGINS", v),
                None => std::env::remove_var("ARGO_TRUSTED_ORIGINS"),
            }
            Self { prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var("ARGO_TRUSTED_ORIGINS", v),
                None => std::env::remove_var("ARGO_TRUSTED_ORIGINS"),
            }
        }
    }

    fn req_with(method: &str, headers: &[(&str, &str)]) -> String {
        let mut s = format!("{method} /api/v1/dispatch HTTP/1.1\r\n");
        for (k, v) in headers {
            s.push_str(&format!("{k}: {v}\r\n"));
        }
        s.push_str("\r\n");
        s
    }

    #[test]
    fn safe_methods_pass() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(None);
        assert!(check_origin(&req_with("GET", &[("Origin", "https://evil.example")])).is_none());
    }

    #[test]
    fn no_origin_header_passes() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(None);
        assert!(check_origin(&req_with("POST", &[])).is_none());
    }

    #[test]
    fn allow_listed_origin_passes() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(None);
        let r = req_with("POST", &[("Origin", "http://localhost:3000")]);
        assert!(check_origin(&r).is_none());
    }

    #[test]
    fn untrusted_origin_rejected() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(None);
        let r = req_with("POST", &[("Origin", "https://evil.example")]);
        let resp = check_origin(&r).expect("rejected");
        assert_eq!(resp.0, "403 Forbidden");
        assert!(resp.1.contains("origin_rejected"));
    }

    #[test]
    fn env_driven_allowlist_overrides_default() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _g = EnvGuard::set(Some("https://app.skycot.com"));
        let r_allowed = req_with("POST", &[("Origin", "https://app.skycot.com")]);
        let r_blocked = req_with("POST", &[("Origin", "http://localhost:3000")]);
        assert!(check_origin(&r_allowed).is_none(), "configured origin should pass");
        assert!(check_origin(&r_blocked).is_some(), "default origin blocked when env set");
    }
}
