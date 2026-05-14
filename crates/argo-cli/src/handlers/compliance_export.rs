//! Thin CLI shim for `argo compliance export`.

use crate::cli::ComplianceExportArgs as CliComplianceExportArgs;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_compliance::export::ComplianceExportArgs;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

pub async fn run(args: CliComplianceExportArgs) -> ArgoResult<()> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    argo_compliance::export::run(ComplianceExportArgs::from(args), &store).await
}

impl From<CliComplianceExportArgs> for ComplianceExportArgs {
    fn from(args: CliComplianceExportArgs) -> Self {
        Self {
            tenant: args.tenant,
            format: args.format,
            out: args.out,
        }
    }
}
