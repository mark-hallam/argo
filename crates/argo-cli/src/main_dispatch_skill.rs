//! Per-domain dispatch sub-fns for the `Skill / Memory / Trust /
//! Judge / Plugin / Model / Mcp / Forge` Command variants. Each fn
//! is the body of one top-level `match Command::*` arm extracted out
//! of `main.rs` (`claude-argo-cli-main-rs-split`); semantics are
//! byte-identical to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn skill(cmd: cli::SkillCmd) -> ArgoResult<()> {
    handlers::skill::run(cmd).await
}

pub(crate) async fn memory(cmd: cli::MemoryCmd) -> ArgoResult<()> {
    handlers::memory::run(cmd).await
}

pub(crate) async fn trust(cmd: cli::cli_trust_cmds::TrustCmd) -> ArgoResult<()> {
    handlers::trust_graph::run(cmd).await
}

pub(crate) async fn judge(cmd: cli::JudgeCmd) -> ArgoResult<()> {
    handlers::judge::run(cmd).await
}

pub(crate) fn plugin(cmd: cli::PluginCmd) -> ArgoResult<()> {
    handlers::plugin::run(cmd)
}

pub(crate) fn model(cmd: cli::ModelCmd) -> ArgoResult<()> {
    handlers::model::run(cmd)
}

pub(crate) async fn mcp(cmd: cli::McpCmd) -> ArgoResult<()> {
    handlers::mcp::run(cmd).await
}

pub(crate) async fn forge(cmd: cli::ForgeCmd) -> ArgoResult<()> {
    handlers::forge::run(cmd).await
}
