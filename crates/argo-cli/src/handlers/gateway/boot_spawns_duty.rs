//! Duty-scheduler construction for the gateway boot path. Mechanical
//! extraction from `boot_spawns.rs` (no behaviour change) to keep that
//! file under the per-file LOC budget. Owns the `ARGO_DUTY_AUTOFIRE` gate
//! selection (default-OFF `ClosedGate` vs the real `AutoApprovalDutyGate`),
//! the manifest-backed risk resolver, and the gateway's `DutyAgentRunner`.
//!
//! LOC headroom: keep new duty-scheduler helpers in this sibling, not back
//! in `boot_spawns.rs`.

use argo_orchestrator::{
    AutoApprovalDutyGate, ClosedGate, DutyAgentRunner, DutyGate, DutyRunContext, DutyScheduler,
    DutyStore, OrchestratorStore,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// True when `ARGO_DUTY_AUTOFIRE` is set to a truthy value
/// (`1`/`true`/`yes`/`on`, case-insensitive). **Default OFF**
/// (UF Phase-B · B2, DEC-036 / DEC-040): unset / any other value
/// keeps wiring [`ClosedGate`], so today's deny-all behaviour is
/// preserved. The live flip is operator-only, after the adversarial
/// budget/risk review.
pub(super) fn duty_autofire_enabled() -> bool {
    parse_autofire_flag(std::env::var("ARGO_DUTY_AUTOFIRE").ok().as_deref())
}

/// Pure truthiness parser for the `ARGO_DUTY_AUTOFIRE` flag — split
/// out so the default-OFF contract is unit-testable without mutating
/// the process environment.
fn parse_autofire_flag(raw: Option<&str>) -> bool {
    raw.map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Build the DEC-041 risk resolver the `AutoApprovalDutyGate` consults to
/// learn a duty's real `(risk_level, requires_approval)` from its worker's
/// vertical manifest (`argo-orchestrator` can't read manifests; the gateway
/// can). Returns `None` — the gate then fail-closes to held — when the
/// vertical or the `entry_action` isn't found. Loads the registry per call
/// so a freshly-hired teammate's vertical is visible without a restart;
/// duties fire at most once/min/duty, so the per-call load is acceptable.
fn make_duty_risk_resolver() -> argo_orchestrator::DutyRiskResolver {
    Arc::new(|vertical_id: &str, entry_action: &str| {
        let dir = crate::handlers::vertical::resolve_verticals_dir();
        let (registry, _errors) = argo_vertical::load_dir(&dir).ok()?;
        // Explicit loop (not `find_map` as the block tail) so the borrowing
        // iterator temporary drops before `registry` does (E0597) — and
        // avoids the `let_and_return` clippy lint the borrow-fix would trip.
        for (_, loaded) in registry.list() {
            if loaded.manifest.vertical.id != vertical_id {
                continue;
            }
            if let Some(decl) = loaded
                .manifest
                .actions
                .as_ref()
                .and_then(|a| a.actions.get(entry_action))
            {
                let risk = match decl.risk {
                    argo_vertical::ActionRiskLevel::Low => "low",
                    argo_vertical::ActionRiskLevel::Medium => "medium",
                    argo_vertical::ActionRiskLevel::High => "high",
                };
                return Some((risk.to_string(), decl.requires_approval));
            }
        }
        None
    })
}

/// Enabled workspace-default auto-approval policy the duty gate falls back
/// to when a worker carries no explicit policy (DEC-041). Enables the
/// act-by-default safe tier within a CONSERVATIVE spend envelope. Only
/// consulted when `ARGO_DUTY_AUTOFIRE` is on, so it stays dormant by
/// default; the caps are a deliberately tight starting point, meant to be
/// operator-tuned before the live flip.
fn safe_tier_workspace_default() -> argo_identity::AutoApprovalPolicy {
    argo_identity::AutoApprovalPolicy {
        enabled: true,
        // $0.50/action, $5/day — above the ~$0.05 per-fire estimate so a
        // safe-tier duty CAN fire, tight enough to bound a runaway loop.
        max_usd_micros_per_action: Some(500_000),
        daily_budget_usd_micros: Some(5_000_000),
        ..Default::default()
    }
}

pub(super) fn build_gateway_duty_scheduler(store: Arc<SessionStore>) -> Arc<DutyScheduler> {
    let duties = Arc::new(DutyStore::new(Arc::clone(&store)));
    let orchestrator = Arc::new(OrchestratorStore::new(Arc::clone(&store)));
    let sink = Arc::new(argo_audit::AuditSink::new_with_tenant(
        Arc::clone(&store),
        argo_audit::SYSTEM_AUDIT_TENANT_ID.to_string(),
    ));
    // UF Phase-B · B2: behind `ARGO_DUTY_AUTOFIRE` (default OFF),
    // swap the V1 fail-safe `ClosedGate` (deny-all) for the real
    // auto-approval gate that reuses the merged H-1 evaluator over
    // each worker's resolved policy + per-worker spend. With the flag
    // unset the gate stays closed — every fire is denied exactly as
    // before, so this lands dormant and reversible.
    let gate: Arc<dyn DutyGate> = if duty_autofire_enabled() {
        // DEC-041: inject the manifest-backed risk resolver so the gate
        // tiers on each duty's REAL risk (orchestrator can't read
        // manifests; we can). Without it the gate fail-closes (holds).
        Arc::new(
            AutoApprovalDutyGate::new(Arc::clone(&store))
                .with_audit_sink(Arc::clone(&sink))
                .with_risk_resolver(make_duty_risk_resolver())
                .with_workspace_default(safe_tier_workspace_default()),
        )
    } else {
        Arc::new(ClosedGate)
    };
    let runner = Arc::new(GatewayDutyAgentRunner::new(Arc::clone(&store)));
    build_gateway_duty_scheduler_with(duties, orchestrator, gate, runner, sink)
}

fn build_gateway_duty_scheduler_with(
    duties: Arc<DutyStore>,
    orchestrator: Arc<OrchestratorStore>,
    gate: Arc<dyn DutyGate>,
    runner: Arc<dyn DutyAgentRunner>,
    sink: Arc<argo_audit::AuditSink>,
) -> Arc<DutyScheduler> {
    Arc::new(
        DutyScheduler::new(duties, orchestrator, gate)
            .with_agent_runner(runner)
            .with_audit_sink(sink),
    )
}

struct GatewayDutyAgentRunner {
    store: Arc<SessionStore>,
    cwd: PathBuf,
}

impl GatewayDutyAgentRunner {
    fn new(store: Arc<SessionStore>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self { store, cwd }
    }
}

#[async_trait]
impl DutyAgentRunner for GatewayDutyAgentRunner {
    async fn run_turn(&self, ctx: DutyRunContext) -> ArgoResult<()> {
        let session_uuid = uuid::Uuid::parse_str(ctx.session_id.as_str()).map_err(|e| {
            ArgoError::Schema(format!(
                "duty scheduler session id '{}' is not a UUID: {e}",
                ctx.session_id.as_str()
            ))
        })?;
        let session_id = argo_types::SessionId(session_uuid);
        let prompt = format!(
            "Run scheduled duty `{}` for worker `{}`. Duty id: {}.",
            ctx.entry_action, ctx.worker_id, ctx.duty_id
        );
        argo_agent_runner::run_turn(
            &self.store,
            &self.cwd,
            session_id,
            Some(&prompt),
            false,
            argo_agent_runner::MAX_TOOL_ROUNDS,
        )
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod autofire_flag_tests {
    use super::parse_autofire_flag;

    #[test]
    fn defaults_off_when_unset_or_unrecognised() {
        // Default-OFF contract (DEC-036 / DEC-040): unset keeps the
        // gate closed; so does any non-truthy value.
        assert!(!parse_autofire_flag(None));
        assert!(!parse_autofire_flag(Some("")));
        assert!(!parse_autofire_flag(Some("0")));
        assert!(!parse_autofire_flag(Some("false")));
        assert!(!parse_autofire_flag(Some("off")));
        assert!(!parse_autofire_flag(Some("maybe")));
    }

    #[test]
    fn enables_on_truthy_values_case_insensitively() {
        for v in ["1", "true", "TRUE", " yes ", "On"] {
            assert!(parse_autofire_flag(Some(v)), "{v:?} should enable autofire");
        }
    }
}
