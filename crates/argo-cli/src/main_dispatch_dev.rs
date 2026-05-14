//! Per-domain dispatch sub-fns for the developer-experience slice
//! of `RuntimeCmd`: `Tui / Voice / Tools`. Each fn is the body of
//! one top-level `match Command::*` arm extracted out of `main.rs`
//! (`claude-argo-cli-main-rs-split`); semantics are byte-identical
//! to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn runtime_tui() -> ArgoResult<()> {
    handlers::tui::run().await
}

pub(crate) async fn runtime_tools(cmd: Option<cli::ToolsCmd>) -> ArgoResult<()> {
    handlers::tools::run(cmd).await
}

pub(crate) fn runtime_voice(mock_script: Option<String>, json: bool) -> ! {
    let code = handlers::voice::run(mock_script, json);
    std::process::exit(code);
}
