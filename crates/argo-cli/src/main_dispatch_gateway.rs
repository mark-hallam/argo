//! Per-domain dispatch sub-fns for the runtime-edge slice of the
//! Command enum: `Gateway / Daemon / Connector / Sandbox /
//! Security` and the gateway-adjacent slice of `RuntimeCmd`
//! (`Health / Doctor / Latency`). Each fn is the body of one
//! top-level `match Command::*` arm extracted out of `main.rs`
//! (`claude-argo-cli-main-rs-split`); semantics are byte-identical
//! to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn gateway(cmd: cli::GatewayCmd) -> ArgoResult<()> {
    handlers::gateway::run(cmd).await
}

pub(crate) async fn daemon(cmd: cli::DaemonCmd) -> ArgoResult<()> {
    match cmd {
        cli::DaemonCmd::Install {
            json,
            dry_run,
            bin,
            launch_agents_dir,
        } => {
            handlers::daemon_install::run(
                json,
                dry_run,
                bin.as_deref(),
                launch_agents_dir.as_deref(),
            )
            .await
        }
        cli::DaemonCmd::Uninstall {
            json,
            dry_run,
            launch_agents_dir,
        } => {
            handlers::daemon_uninstall::run(json, dry_run, launch_agents_dir.as_deref()).await
        }
    }
}

pub(crate) async fn connector(cmd: cli::ConnectorCmd) -> ArgoResult<()> {
    handlers::connector_authorize::run(cmd).await
}

pub(crate) async fn sandbox(cmd: cli::SandboxCmd) -> ArgoResult<()> {
    handlers::sandbox_e2b::run(cmd).await
}

pub(crate) async fn security(cmd: cli::SecurityCmd) -> ArgoResult<()> {
    match cmd {
        audit @ cli::SecurityCmd::Audit { .. } => handlers::security_audit::run(audit).await,
        cli::SecurityCmd::ScanSecrets(args) => handlers::security_scan_secrets::run(args),
        cli::SecurityCmd::RotateKeys { cmd } => {
            handlers::security_rotate_keys::dispatch(cmd).await
        }
    }
}

pub(crate) async fn runtime_health(json: bool, format: String) -> ArgoResult<()> {
    handlers::health::run(json, &format).await
}

pub(crate) async fn runtime_doctor(
    cmd: Option<cli::DoctorCmd>,
    json: bool,
    format: String,
) -> ArgoResult<()> {
    handlers::doctor::dispatch(cmd, json, format).await
}

pub(crate) async fn runtime_latency(format: String) -> ArgoResult<()> {
    handlers::latency::run(&format).await
}
