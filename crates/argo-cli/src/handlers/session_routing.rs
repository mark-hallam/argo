//! `argo session set-routing-policy` handler (Stream
//! `mission-claim-3-runtime-wiring-v1x`). Thin wrapper around
//! `SessionStore::set_routing_policy` that translates CLI args into a
//! `RoutingPolicy::Tiered` (or `None` when `--none` is passed).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, ProviderModel, RoutingPolicy, SessionId, TieredRouting};

/// Per-tier `(provider, model)` pair. Each is required when
/// `--tiered` is passed; the handler validates them up front.
pub struct TierArgs<'a> {
    pub classifier_provider: Option<&'a str>,
    pub classifier_model: Option<&'a str>,
    pub simple_provider: Option<&'a str>,
    pub simple_model: Option<&'a str>,
    pub medium_provider: Option<&'a str>,
    pub medium_model: Option<&'a str>,
    pub complex_provider: Option<&'a str>,
    pub complex_model: Option<&'a str>,
    pub classify_threshold_ms: u64,
    pub fallback_to_default_on_error: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    raw_id: &str,
    tiered: bool,
    none: bool,
    args: TierArgs<'_>,
    json: bool,
) -> ArgoResult<()> {
    let id = uuid::Uuid::parse_str(raw_id)
        .map(SessionId)
        .map_err(|e| ArgoError::Schema(format!("invalid session id '{raw_id}': {e}")))?;
    let policy: Option<RoutingPolicy> = if none {
        None
    } else if tiered {
        Some(build_tiered_policy(args)?)
    } else {
        return Err(ArgoError::Schema(
            "must pass either --tiered (with classifier + tier model args) or --none".into(),
        ));
    };
    let store = open_store().await?;
    store.set_routing_policy(id, policy.as_ref()).await?;
    if json {
        let payload = serde_json::json!({
            "session": raw_id,
            "policy": policy,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        match &policy {
            Some(RoutingPolicy::Tiered(_)) => {
                println!("set tiered routing policy on session {raw_id}");
            }
            Some(_) => println!("set routing policy on session {raw_id}"),
            None => println!("cleared routing policy on session {raw_id}"),
        }
    }
    Ok(())
}

fn build_tiered_policy(args: TierArgs<'_>) -> ArgoResult<RoutingPolicy> {
    Ok(RoutingPolicy::Tiered(TieredRouting {
        classifier_model: require_pair(
            "classifier",
            args.classifier_provider,
            args.classifier_model,
        )?,
        simple: require_pair("simple", args.simple_provider, args.simple_model)?,
        medium: require_pair("medium", args.medium_provider, args.medium_model)?,
        complex: require_pair("complex", args.complex_provider, args.complex_model)?,
        classify_threshold_ms: args.classify_threshold_ms,
        fallback_to_default_on_error: args.fallback_to_default_on_error,
    }))
}

fn require_pair(
    tier: &str,
    provider: Option<&str>,
    model: Option<&str>,
) -> ArgoResult<ProviderModel> {
    let provider = provider.ok_or_else(|| {
        ArgoError::Schema(format!("--{tier}-provider is required with --tiered"))
    })?;
    let model =
        model.ok_or_else(|| ArgoError::Schema(format!("--{tier}-model is required with --tiered")))?;
    Ok(ProviderModel {
        provider: provider.to_string(),
        model: model.to_string(),
    })
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}
