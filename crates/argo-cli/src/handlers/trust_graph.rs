//! `argo trust graph` / `argo trust composed` handlers (Stream
//! `mission-cross-tenant-trust-graph`).
//!
//! Materializes a `TrustGraph` from the local store's
//! `trust_observation` rows, then either prints the `--from` tenant's
//! outgoing edges or walks the composed BFS to `--to`.

use crate::cli::cli_trust_cmds::TrustCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::trust_graph::{TrustEdge, TrustGraph, TrustObservation};
use argo_store::trust_observation::list_all_observations_since;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

pub async fn run(cmd: TrustCmd) -> ArgoResult<()> {
    match cmd {
        TrustCmd::Graph { from, days_back, json } => run_graph(&from, days_back, json).await,
        TrustCmd::Composed { from, to, max_hops, days_back, json } => {
            run_composed(&from, &to, max_hops, days_back, json).await
        }
    }
}

async fn run_graph(from: &str, days_back: u32, json: bool) -> ArgoResult<()> {
    let from_tenant = parse_tenant(from)?;
    let graph = load_graph(days_back).await?;
    let neighbors: Vec<&TrustEdge> = graph.neighbors(&from_tenant);
    if json {
        let rows: Vec<NeighborJson> = neighbors.iter().map(|e| NeighborJson::from(*e)).collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&rows)
                .map_err(|e| ArgoError::Provider(format!("trust graph serialize: {e}")))?
        );
        return Ok(());
    }
    if neighbors.is_empty() {
        println!("(no outgoing trust edges for {from_tenant})");
        return Ok(());
    }
    println!("{:<36}  {:>6}  {:>6}  last_updated_at", "to_tenant", "weight", "obs");
    for e in neighbors {
        println!(
            "{:<36}  {:>6.3}  {:>6}  {}",
            e.to_tenant,
            e.weight,
            e.observation_count,
            e.last_updated_at.to_rfc3339()
        );
    }
    Ok(())
}

async fn run_composed(
    from: &str,
    to: &str,
    max_hops: usize,
    days_back: u32,
    json: bool,
) -> ArgoResult<()> {
    let from_tenant = parse_tenant(from)?;
    let to_tenant = parse_tenant(to)?;
    let graph = load_graph(days_back).await?;
    let result = graph.composed_trust_with_path(&from_tenant, &to_tenant, max_hops);
    if json {
        let row = ComposedJson {
            from: from_tenant.to_string(),
            to: to_tenant.to_string(),
            max_hops,
            score: result.as_ref().map(|(s, _)| *s),
            path: result
                .as_ref()
                .map(|(_, p)| p.iter().map(ToString::to_string).collect()),
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&row)
                .map_err(|e| ArgoError::Provider(format!("trust composed serialize: {e}")))?
        );
        return Ok(());
    }
    match result {
        Some((score, path)) => {
            let path_str = path
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" → ");
            println!("composed_trust({from_tenant} → {to_tenant}) = {score:.3} via {path_str}");
        }
        None => println!("(no path from {from_tenant} to {to_tenant} within {max_hops} hops)"),
    }
    Ok(())
}

async fn load_graph(days_back: u32) -> ArgoResult<TrustGraph> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    let since = Utc::now() - Duration::days(i64::from(days_back));
    let rows = list_all_observations_since(&store, since).await?;
    let observations: Vec<TrustObservation> = rows
        .into_iter()
        .map(|r| TrustObservation {
            source_tenant: r.source_tenant,
            consumer_tenant: r.consumer_tenant,
            skill_id: r.skill_id,
            quality_score: r.quality_score,
            observed_at: r.observed_at,
        })
        .collect();
    Ok(TrustGraph::from_observations(&observations))
}

fn parse_tenant(s: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(s)
        .map(TenantId)
        .map_err(|e| ArgoError::Provider(format!("tenant id '{s}': {e}")))
}

#[derive(Debug, Serialize)]
struct NeighborJson {
    to_tenant: String,
    weight: f32,
    observation_count: u32,
    last_updated_at_rfc3339: String,
}

impl From<&TrustEdge> for NeighborJson {
    fn from(edge: &TrustEdge) -> Self {
        Self {
            to_tenant: edge.to_tenant.to_string(),
            weight: edge.weight,
            observation_count: edge.observation_count,
            last_updated_at_rfc3339: edge.last_updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ComposedJson {
    from: String,
    to: String,
    max_hops: usize,
    score: Option<f32>,
    path: Option<Vec<String>>,
}
