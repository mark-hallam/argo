//! `argo route` — inspect cost-router decisions for a session.
use crate::cli::RouteCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_routing::{FixedRouter, Router, RoutingContext};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, ProviderModel, RoutingPolicy, SessionId};
use std::time::{Duration, Instant};

pub async fn run(cmd: RouteCmd) -> ArgoResult<()> {
    match cmd {
        RouteCmd::Inspect { session_id } => inspect(&session_id).await,
        RouteCmd::Bench => bench().await,
    }
}

async fn bench() -> ArgoResult<()> {
    let iters: usize = 100_000;
    let router = FixedRouter::new();
    let session_id = SessionId::new();
    let default_model = ProviderModel {
        provider: "anthropic".into(),
        model: "claude-haiku".into(),
    };
    let mut samples: Vec<u128> = Vec::with_capacity(iters);
    let total_start = Instant::now();
    for turn in 0..iters {
        let ctx = RoutingContext {
            session_id,
            turn: u32::try_from(turn).unwrap_or(u32::MAX),
            messages: &[],
            user_message: "bench",
            tools_in_scope: &[],
            default_model: default_model.clone(),
            policy: RoutingPolicy::Fixed,
            deadline: Instant::now() + Duration::from_secs(60),
        };
        let t = Instant::now();
        let _ = router.route(&ctx).await?;
        samples.push(t.elapsed().as_nanos());
    }
    let total = total_start.elapsed();
    samples.sort_unstable();
    let p50 = samples[iters / 2];
    let p99 = samples[(iters * 99) / 100];
    let max = samples[iters - 1];
    let throughput = iters as f64 / total.as_secs_f64();
    println!("argo route bench — FixedRouter ({iters} iterations)");
    println!("  total:      {:.2?}", total);
    println!("  throughput: {throughput:.0} routes/s");
    println!("  p50:        {p50} ns");
    println!("  p99:        {p99} ns");
    println!("  max:        {max} ns");
    Ok(())
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
