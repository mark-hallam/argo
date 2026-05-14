//! Shared bits for `forge bench` + `forge optimize`:
//! - `BenchSubject` — `EvalSubject` impl that drives one task
//!   through `argo_agent_runner::run_turn` against a fresh
//!   recorded session.
//! - `open_store` — open + migrate the workspace's
//!   `SessionStore` (used by every forge subcommand).
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_eval::{EvalSubject, EvalTask};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

pub(super) async fn open_store() -> ArgoResult<SessionStore> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

/// `EvalSubject` impl that runs each task as a fresh recorded
/// session via `argo_agent_runner::run_turn`. Returns the final
/// assistant text; the runner's metric scores it.
#[allow(dead_code)]
pub(super) struct BenchSubject {
    pub(super) store: Arc<SessionStore>,
    pub(super) cwd: PathBuf,
    pub(super) tenant: argo_types::TenantId,
    pub(super) workspace: argo_types::WorkspaceId,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) skill_name: String,
}

#[async_trait]
impl EvalSubject for BenchSubject {
    async fn run(&self, task: &EvalTask) -> ArgoResult<String> {
        let title = format!("forge bench: {}", task.name);
        let sid = self
            .store
            .create_session(
                self.tenant,
                self.workspace,
                &self.model,
                &self.provider,
                Some(&title),
                None, // forge subject sessions aren't vertical-bound (#651)
            )
            .await?;
        let result = argo_agent_runner::run_turn(
            &self.store,
            &self.cwd,
            SessionId(sid.0),
            Some(&task.input),
            false,
            argo_agent_runner::MAX_TOOL_ROUNDS,
        )
        .await?;
        Ok(result.text)
    }
}
