//! `argo handoff-rules test` handler. Loads the tenant's active
//! rules into a [`RoutingEngine`] and simulates evaluation against
//! a hypothetical handoff context — no DB writes.

use super::handoff_rules_list::{open_routing_store, parse_tenant};
use argo_orchestrator::{EscalationStep, RoutingContext};
use argo_types::{ArgoError, ArgoResult};
use chrono::{Datelike, Timelike, Utc};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct TestOut {
    matched: bool,
    target_worker_id: Option<String>,
    target_human_addr: Option<String>,
}

pub async fn run(
    tenant: &str,
    expected_outcome: &str,
    hour: Option<u32>,
    json: bool,
) -> ArgoResult<()> {
    let store = open_routing_store().await?;
    store.ensure_schema().await?;
    let tenant_id = parse_tenant(tenant)?;
    let engine = store.engine_for_tenant(tenant_id).await?;
    let mut now = Utc::now();
    if let Some(h) = hour {
        if h >= 24 {
            return Err(ArgoError::Provider(format!(
                "hour {h} out of range; expected 0..24"
            )));
        }
        let day = now.date_naive();
        now = day
            .and_hms_opt(h, 0, 0)
            .ok_or_else(|| ArgoError::Provider(format!("compose hour {h} for {day}")))?
            .and_utc();
        // Preserve weekday consistency by re-anchoring via num_days_from_monday.
        let _ = now.weekday().num_days_from_monday();
        let _ = now.minute();
    }
    let ctx = RoutingContext::new(now, expected_outcome);
    let resolved = engine.route(&ctx);
    let out = match resolved {
        None => TestOut {
            matched: false,
            target_worker_id: None,
            target_human_addr: None,
        },
        Some(EscalationStep::Worker(w)) => TestOut {
            matched: true,
            target_worker_id: Some(w.to_string()),
            target_human_addr: None,
        },
        Some(EscalationStep::Human(addr)) => TestOut {
            matched: true,
            target_worker_id: None,
            target_human_addr: Some(addr),
        },
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&out)
                .map_err(|e| ArgoError::Provider(format!("serialize: {e}")))?
        );
    } else if out.matched {
        if let Some(w) = &out.target_worker_id {
            println!("matched: route to worker {w}");
        } else if let Some(addr) = &out.target_human_addr {
            println!("matched: route to human {addr}");
        }
    } else {
        println!("no rule matched (would fall back to escalation chain)");
    }
    Ok(())
}
