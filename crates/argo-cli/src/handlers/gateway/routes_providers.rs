//! `GET /providers` + `GET /providers/<id>` handlers. Lifted
//! out of `routes_meta.rs` so it stays under the 500-LOC budget
//! as the catalog grows.
use argo_provider::ProviderManifest;
use std::path::PathBuf;

/// `GET /providers[?family=<f>&format=markdown]` — list every
/// parseable manifest. Invalid manifests are reported with
/// `{path, error}` rows. Optional `family` (case-insensitive)
/// narrows to Anthropic / OpenAICompatible / OpenAIResponses /
/// Bedrock. Markdown returns a 3-column table (id / family /
/// base_url) — paste-able into READMEs.
pub(super) async fn providers_response(
    family: Option<&str>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Ok(cwd) = std::env::current_dir() else {
        return (
            "500 Internal Server Error",
            "{\"error\":\"cwd unavailable\"}".to_string(),
        );
    };
    let dir = cwd.join("manifests").join("providers");
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let mut manifests: Vec<ProviderManifest> = Vec::new();
    if !dir.exists() {
        if matches!(format, Some("markdown")) {
            return ("200 OK", "# Providers (0)\n".to_string());
        }
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
    let needle = family.map(str::to_lowercase);
    for p in paths {
        match std::fs::read_to_string(&p).and_then(|body| {
            ProviderManifest::from_toml_str(&body).map_err(std::io::Error::other)
        }) {
            Ok(m) => {
                if let Some(want) = needle.as_deref() {
                    let actual = format!("{:?}", m.provider.family).to_lowercase();
                    if !actual.contains(want) {
                        continue;
                    }
                }
                rows.push(manifest_to_json(&m));
                manifests.push(m);
            }
            Err(e) => rows.push(serde_json::json!({
                "path": p.display().to_string(),
                "error": e.to_string(),
            })),
        }
    }
    if matches!(format, Some("markdown")) {
        let n = manifests.len();
        let mut out = format!(
            "# Providers ({n} manifest{})\n\n",
            if n == 1 { "" } else { "s" }
        );
        if !manifests.is_empty() {
            out.push_str("| id | family | base_url | models |\n");
            out.push_str("| --- | --- | --- | ---: |\n");
            for m in &manifests {
                out.push_str(&format!(
                    "| `{}` | `{:?}` | `{}` | {} |\n",
                    m.provider.id,
                    m.provider.family,
                    m.provider.base_url,
                    m.models.len(),
                ));
            }
        }
        return ("200 OK", out);
    }
    (
        "200 OK",
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
    )
}

/// `GET /providers/<id>/models[?format=markdown]` — just the
/// `[[models]]` array from one provider's manifest. Default
/// JSON shape is an array of `{id, context_window,
/// price_per_1k_in, price_per_1k_out, tier}`. Markdown returns
/// a 5-column table. 404 when manifest missing; 400 on
/// path-traversal.
pub(super) async fn provider_models_response(
    raw_id: &str,
    format: Option<&str>,
) -> (&'static str, String) {
    if !matches!(format, Some("markdown")) {
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
        return (
            "200 OK",
            serde_json::to_string(&models).unwrap_or_else(|_| "[]".to_string()),
        );
    }
    // Markdown branch: load the manifest and render the models
    // as a 5-column table. Same path-traversal + read errors
    // as `provider_show_response`.
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
    let n = m.models.len();
    let mut out = format!(
        "# Models for `{raw_id}` ({n} model{})\n\n",
        if n == 1 { "" } else { "s" }
    );
    if !m.models.is_empty() {
        out.push_str("| id | ctx | tier | input/1k | output/1k |\n");
        out.push_str("| --- | ---: | --- | ---: | ---: |\n");
        for spec in &m.models {
            let ctx = spec
                .context_window
                .map(|n| n.to_string())
                .unwrap_or_else(|| "-".into());
            let tier = spec.tier.as_deref().unwrap_or("-");
            let in_p = spec
                .price_per_1k_in
                .map(|p| format!("${p:.4}"))
                .unwrap_or_else(|| "-".into());
            let out_p = spec
                .price_per_1k_out
                .map(|p| format!("${p:.4}"))
                .unwrap_or_else(|| "-".into());
            out.push_str(&format!(
                "| `{}` | {ctx} | `{tier}` | {in_p} | {out_p} |\n",
                spec.id
            ));
        }
    }
    ("200 OK", out)
}

/// `GET /providers/<id>` — fetch one provider manifest by
/// basename. Returns 200 + manifest JSON; 404 when no
/// `manifests/providers/<id>.toml` exists; 400 on path-
/// traversal `id` segments.
pub(super) async fn provider_show_response(raw_id: &str) -> (&'static str, String) {
    provider_show_response_with_format(raw_id, None).await
}

/// Same as `provider_show_response`, but supports
/// `?format=markdown` rendering. Default returns the
/// historical JSON shape unchanged.
pub(super) async fn provider_show_response_with_format(
    raw_id: &str,
    format: Option<&str>,
) -> (&'static str, String) {
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
    if matches!(format, Some("markdown")) {
        let mut out = format!("# Provider `{}` ({:?})\n\n", m.provider.id, m.provider.family);
        out.push_str(&format!("- **base_url**: `{}`\n", m.provider.base_url));
        out.push_str(&format!(
            "- **auth**: env=`{}` (header=`{}`, format=`{}`)\n",
            m.auth.api_key_env, m.auth.header_name, m.auth.header_format
        ));
        if !m.models.is_empty() {
            out.push_str("\n## Models\n\n");
            out.push_str("| id | ctx | tier | input/1k | output/1k |\n");
            out.push_str("| --- | ---: | --- | ---: | ---: |\n");
            for spec in &m.models {
                let ctx = spec
                    .context_window
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into());
                let tier = spec.tier.as_deref().unwrap_or("-");
                let in_p = spec
                    .price_per_1k_in
                    .map(|p| format!("${p:.4}"))
                    .unwrap_or_else(|| "-".into());
                let out_p = spec
                    .price_per_1k_out
                    .map(|p| format!("${p:.4}"))
                    .unwrap_or_else(|| "-".into());
                out.push_str(&format!(
                    "| `{}` | {ctx} | `{tier}` | {in_p} | {out_p} |\n",
                    spec.id
                ));
            }
        }
        return ("200 OK", out);
    }
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
