//! `argo provider` — list / validate / show / test manifests.
use crate::cli::ProviderCmd;
use argo_provider::{
    AnthropicTransport, BedrockTransport, OpenAiCompatTransport, OpenAiResponsesTransport,
    ProviderFamily, ProviderManifest, ProviderRequest, Sampling, Transport,
};
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId};
use chrono::Utc;
use std::path::PathBuf;

pub async fn run(cmd: ProviderCmd) -> ArgoResult<()> {
    match cmd {
        ProviderCmd::List { json } => list_manifests(json),
        ProviderCmd::Show { id, json } => show(&id, json),
        ProviderCmd::Validate { manifest } => validate(&PathBuf::from(manifest)),
        ProviderCmd::ValidateAll => validate_all(),
        ProviderCmd::Test { provider } => test(&provider).await,
    }
}

async fn test(provider_id: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let path = cwd
        .join("manifests")
        .join("providers")
        .join(format!("{provider_id}.toml"));
    let body = std::fs::read_to_string(&path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let manifest = ProviderManifest::from_toml_str(&body)?;
    let model = manifest
        .models
        .first()
        .map(|m| m.id.clone())
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "manifest {provider_id} has no models — add at least one entry to `[[models]]` or call with --model"
            ))
        })?;
    println!("provider test: {provider_id} → {model}");
    let transport: Box<dyn Transport> = match manifest.provider.family {
        ProviderFamily::Bedrock => {
            let cfg = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
            Box::new(BedrockTransport::from_manifest(manifest, &cfg)?)
        }
        ProviderFamily::Anthropic => {
            let api_key = std::env::var(&manifest.auth.api_key_env).map_err(|_| {
                ArgoError::Provider(format!("{} not set", manifest.auth.api_key_env))
            })?;
            Box::new(AnthropicTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiCompatible => {
            let api_key = std::env::var(&manifest.auth.api_key_env).map_err(|_| {
                ArgoError::Provider(format!("{} not set", manifest.auth.api_key_env))
            })?;
            Box::new(OpenAiCompatTransport::from_manifest(manifest, &api_key)?)
        }
        ProviderFamily::OpenaiResponses => {
            let api_key = std::env::var(&manifest.auth.api_key_env).map_err(|_| {
                ArgoError::Provider(format!("{} not set", manifest.auth.api_key_env))
            })?;
            Box::new(OpenAiResponsesTransport::from_manifest(manifest, &api_key)?)
        }
    };

    let session_id = SessionId(uuid::Uuid::nil());
    let messages = vec![Message {
        id: MessageId(uuid::Uuid::now_v7()),
        session_id,
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: "ping".into(),
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        reasoning: None,
        created_at: Utc::now(),
        provider_meta: serde_json::Value::Null,
    }];
    let req = ProviderRequest {
        model: model.clone(),
        messages,
        tools: Vec::new(),
        system: None,
        sampling: Sampling {
            max_tokens: Some(16),
            ..Sampling::default()
        },
    };
    let started = std::time::Instant::now();
    let resp = transport.complete(&req).await?;
    let elapsed = started.elapsed();
    let text = resp.text.unwrap_or_default();
    let snippet: String = text.chars().take(160).collect();
    println!("  status:    ok");
    println!("  latency:   {} ms", elapsed.as_millis());
    if let (Some(ti), Some(to)) = (resp.tokens_in, resp.tokens_out) {
        println!("  tokens:    {ti} in / {to} out");
    }
    if !snippet.is_empty() {
        println!("  reply:     {snippet}");
    }
    Ok(())
}

fn validate_all() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let dir = cwd.join("manifests").join("providers");
    if !dir.exists() {
        return Err(ArgoError::Provider(format!(
            "no manifests at {} (run from a workspace with manifests/providers/)",
            dir.display()
        )));
    }
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_some_and(|x| x == "toml") {
            paths.push(p);
        }
    }
    paths.sort();
    if paths.is_empty() {
        println!("(no manifests under {})", dir.display());
        return Ok(());
    }
    let mut failures: Vec<(PathBuf, String)> = Vec::new();
    for p in &paths {
        let body = std::fs::read_to_string(p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        match ProviderManifest::from_toml_str(&body) {
            Ok(m) => println!(
                "  {:<24} ok ({:?})",
                p.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
                m.provider.family
            ),
            Err(e) => failures.push((p.clone(), e.to_string())),
        }
    }
    if failures.is_empty() {
        println!(
            "{} manifest(s) validated under {}",
            paths.len(),
            dir.display()
        );
        Ok(())
    } else {
        for (p, msg) in &failures {
            eprintln!("  {} INVALID — {msg}", p.display());
        }
        Err(ArgoError::Provider(format!(
            "{} of {} manifest(s) failed validation",
            failures.len(),
            paths.len()
        )))
    }
}

fn show(provider_id: &str, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let manifests_dir = cwd.join("manifests").join("providers");
    if !manifests_dir.exists() {
        return Err(ArgoError::Provider(format!(
            "no manifests at {} (run from a workspace with manifests/providers/)",
            manifests_dir.display()
        )));
    }
    for entry in std::fs::read_dir(&manifests_dir)
        .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", manifests_dir.display())))?
    {
        let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
        let p = e.path();
        if p.extension().is_none_or(|x| x != "toml") {
            continue;
        }
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        let m = match ProviderManifest::from_toml_str(&body) {
            Ok(m) => m,
            Err(_) => continue, // skip invalid manifests in show-by-id flow
        };
        if m.provider.id == provider_id {
            if json {
                let serialized = serde_json::to_string(&m)
                    .map_err(|e| ArgoError::Provider(format!("serialize manifest: {e}")))?;
                println!("{serialized}");
                return Ok(());
            }
            return render(&m);
        }
    }
    Err(ArgoError::Provider(format!(
        "no provider with id '{provider_id}' under {}",
        manifests_dir.display()
    )))
}

fn render(m: &ProviderManifest) -> ArgoResult<()> {
    println!("# {} ({:?})", m.provider.id, m.provider.family);
    println!("  base_url:   {}", m.provider.base_url);
    println!(
        "  auth:       header={} format={} env={}",
        m.auth.header_name, m.auth.header_format, m.auth.api_key_env
    );
    println!("  streaming:  {:?}", m.transport.streaming);
    println!(
        "  timeouts:   connect={}ms read={}ms max_retries={}",
        m.transport.connect_timeout_ms, m.transport.read_timeout_ms, m.transport.max_retries
    );
    println!("  tool_calls: {:?}", m.tool_calls.format);
    if !m.models.is_empty() {
        println!("  models:");
        for spec in &m.models {
            let tier = spec.tier.as_deref().unwrap_or("-");
            println!(
                "    {:<32} ctx={:<8} tier={tier}",
                spec.id,
                spec.context_window
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".into())
            );
        }
    }
    if let Some(d) = m.discovery.as_ref() {
        println!(
            "  discovery:  endpoint={} prefer={:?} refresh={}h",
            d.endpoint, d.prefer, d.refresh_interval_hours
        );
    }
    if let Some(r) = m.reasoning.as_ref() {
        println!(
            "  reasoning:  field={} preserve_across_turns={}",
            r.content_field, r.preserve_across_turns
        );
    }
    Ok(())
}

fn list_manifests(json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    // Spec layout: workspace `manifests/providers/*.toml` for tenant
    // overrides; the bundled v1 set lives in the source repo at
    // `manifests/providers/`. Look in both, the workspace dir taking
    // precedence.
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in [cwd.join("manifests").join("providers")] {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| ArgoError::Provider(format!("read_dir {}: {e}", dir.display())))?
        {
            let e = entry.map_err(|e| ArgoError::Provider(format!("dir entry: {e}")))?;
            let p = e.path();
            if p.extension().is_some_and(|x| x == "toml") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    if json {
        let arr: Vec<serde_json::Value> = paths
            .iter()
            .filter_map(|p| {
                let body = std::fs::read_to_string(p).ok()?;
                let m = ProviderManifest::from_toml_str(&body).ok()?;
                Some(serde_json::json!({
                    "id": m.provider.id,
                    "family": format!("{:?}", m.provider.family),
                    "base_url": m.provider.base_url,
                }))
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
        );
        return Ok(());
    }
    if paths.is_empty() {
        println!(
            "(no provider manifests under {}/manifests/providers/)",
            cwd.display()
        );
        return Ok(());
    }
    for p in paths {
        let body = std::fs::read_to_string(&p)
            .map_err(|e| ArgoError::Provider(format!("read {}: {e}", p.display())))?;
        match ProviderManifest::from_toml_str(&body) {
            Ok(m) => println!(
                "  {:<24} {:?}  base={}",
                m.provider.id, m.provider.family, m.provider.base_url
            ),
            Err(e) => println!("  {:<24} INVALID  ({e})", p.display()),
        }
    }
    Ok(())
}

fn validate(path: &std::path::Path) -> ArgoResult<()> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let m = ProviderManifest::from_toml_str(&body)?;
    println!("manifest ok: {} ({:?})", m.provider.id, m.provider.family);
    println!("  base_url:  {}", m.provider.base_url);
    println!("  streaming: {:?}", m.transport.streaming);
    Ok(())
}
