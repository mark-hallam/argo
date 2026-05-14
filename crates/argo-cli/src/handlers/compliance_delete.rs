//! Thin CLI shim for `argo compliance delete`.

use crate::cli::ComplianceDeleteArgs as CliComplianceDeleteArgs;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_compliance::delete::ComplianceDeleteArgs;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

#[allow(unused_imports)]
pub use argo_compliance::delete::EXIT_CONFIRM_MISMATCH;

pub async fn run(args: CliComplianceDeleteArgs) -> ArgoResult<i32> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    argo_compliance::delete::run(ComplianceDeleteArgs::from(args), &store).await
}

impl From<CliComplianceDeleteArgs> for ComplianceDeleteArgs {
    fn from(args: CliComplianceDeleteArgs) -> Self {
        Self {
            tenant: args.tenant,
            confirm: args.confirm,
            dry_run: args.dry_run,
            json: args.json,
        }
    }
}
