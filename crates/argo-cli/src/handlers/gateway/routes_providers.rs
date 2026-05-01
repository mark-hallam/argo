//! `GET /providers` + `GET /providers/<id>` handlers. Lifted
//! out of `routes_meta.rs` so it stays under the 500-LOC budget
//! as the catalog grows.
use argo_provider::ProviderManifest;
use std::path::PathBuf;

/// `GET /providers` — list every parseable manifest under
/// `<cwd>/manifests/providers/*.toml`. Invalid manifests are
/// reported with `{path, error}` rows so a typo is visible
/// without crashing the whole list.
pub(super) async fn providers_response() -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let dir = cwd.join("manifests").join("providers");
    let mut rows: Vec<serde_json::Value> = Vec::new();
    if !dir.exists() {
        return (
            "200 OK",
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
        );
    }
    let mut paths: Vec<PathBuf> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect(),
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"read_dir: {e}\"}}"),
            );
        }
    };
    paths.sort();
    for p in paths {
        match std::fs::read_to_string(&p).and_then(|body| {
            ProviderManifest::from_toml_str(&body).map_err(std::io::Error::other)
        }) {
            Ok(m) => rows.push(manifest_to_json(&m)),
            Err(e) => rows.push(serde_json::json!({
                "path": p.display().to_string(),
                "error": e.to_string(),
            })),
        }
    }
    (
        "200 OK",
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /providers/<id>/models` — just the `[[models]]` array
/// from one provider's manifest. 200 + JSON array of
/// `{id, context_window, price_per_1k_in, price_per_1k_out,
/// tier}`. 404 when manifest missing; 400 on path-traversal.
pub(super) async fn provider_models_response(raw_id: &str) -> (&'static str, String) {
    let (status, body) = provider_show_response(raw_id).await;
    if !status.starts_with("200") {
        return (status, body);
    }
    let parsed: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"reparse manifest JSON: {e}\"}}"),
            );
        }
    };
    let models = parsed
        .get("models")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    (
        "200 OK",
        serde_json::to_string(&models).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /providers/<id>` — fetch one provider manifest by
/// basename. Returns 200 + manifest JSON; 404 when no
/// `manifests/providers/<id>.toml` exists; 400 on path-
/// traversal `id` segments.
pub(super) async fn provider_show_response(raw_id: &str) -> (&'static str, String) {
    if raw_id.contains('/') || raw_id.contains('\\') || raw_id.contains("..") {
        return (
            "400 Bad Request",
            "{\"error\":\"provider id must be a single segment\"}".to_string(),
        );
    }
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let path = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{raw_id}.toml"));
    if !path.exists() {
        return (
            "404 Not Found",
            format!("{{\"error\":\"no provider manifest '{raw_id}'\"}}"),
        );
    }
    let body = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return (
                "500 Internal Server Error",
                format!("{{\"error\":\"read {}: {e}\"}}", path.display()),
            );
        }
    };
    let m = match ProviderManifest::from_toml_str(&body) {
        Ok(m) => m,
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"parse {}: {e}\"}}", path.display()),
            );
        }
    };
    (
        "200 OK",
        serde_json::to_string(&manifest_to_json(&m)).unwrap_or_else(|_| "{}".to_string()),
    )
}

fn manifest_to_json(m: &ProviderManifest) -> serde_json::Value {
    let models: Vec<serde_json::Value> = m
        .models
        .iter()
        .map(|spec| {
            serde_json::json!({
                "id": spec.id,
                "context_window": spec.context_window,
                "price_per_1k_in": spec.price_per_1k_in,
                "price_per_1k_out": spec.price_per_1k_out,
                "tier": spec.tier,
            })
        })
        .collect();
    serde_json::json!({
        "id": m.provider.id,
        "family": format!("{:?}", m.provider.family),
        "base_url": m.provider.base_url,
        "models": models,
    })
}
