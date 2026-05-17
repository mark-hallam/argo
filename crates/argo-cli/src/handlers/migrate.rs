//! Dispatch for `argo migrate <subcommand>`
//! (`mission-sqlcipher-3-migrate-cli`).

use crate::cli::cli_subcommands_migrate::MigrateCmd;
use argo_types::ArgoResult;

pub async fn run(cmd: MigrateCmd) -> ArgoResult<()> {
    match cmd {
        MigrateCmd::Encrypt(args) => crate::handlers::migrate_encrypt::run(args).await,
    }
}
