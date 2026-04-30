//! `argo route` — inspect cost-router decisions for a session.
use crate::cli::RouteCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};

pub async fn run(cmd: RouteCmd) -> ArgoResult<()> {
    match cmd {
        RouteCmd::Inspect { session_id } => inspect(&session_id).await,
        RouteCmd::Bench => {
            // Real bench needs a live router + provider — that's the
            // daemon's job. The lib-tier `argo-routing` carries the
            // FixedRouter / TieredRouter / PolicyPluginRouter that
            // a future PR will exercise here.
            println!("argo route bench: requires the daemon's router binding (separate PR).");
            Ok(())
        }
    }
}

async fn inspect(raw: &str) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = SessionStore::open(&db).await?;
    store.migrate().await?;
    let id = uuid::Uuid::parse_str(raw)
        .map_err(|e| ArgoError::Provider(format!("invalid session id '{raw}': {e}")))?;
    let decisions = store.list_routing_decisions(SessionId(id)).await?;
    if decisions.is_empty() {
        println!("(no routing decisions for session {raw})");
        return Ok(());
    }
    println!(
        "{turn:<5} {tier:<10} {policy:<14} {ms:<6}  provider:model     reason",
        turn = "turn",
        tier = "tier",
        policy = "policy",
        ms = "ms"
    );
    for d in decisions {
        println!(
            "{turn:<5} {tier:<10} {policy:<14} {ms:<6}  {provider}:{model}    {reason}",
            turn = d.turn,
            tier = d.tier,
            policy = d.policy,
            ms = d.classifier_latency_ms,
            provider = d.chosen_provider_model.provider,
            model = d.chosen_provider_model.model,
            reason = d.reason
        );
    }
    Ok(())
}
