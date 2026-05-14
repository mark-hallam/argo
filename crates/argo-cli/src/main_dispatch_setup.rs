//! Per-domain dispatch sub-fns for the workspace-bootstrap /
//! installer / packaging slice of the Command enum:
//! `Wizard / Provider / Db / Workspace / Route / Cred / Backup /
//! Restore / SelfUpdate` and the bootstrap slice of `RuntimeCmd`
//! (`Setup / Onboard / Update`). Each fn is the body of one
//! top-level `match Command::*` arm extracted out of `main.rs`
//! (`claude-argo-cli-main-rs-split`); semantics are byte-identical
//! to the pre-split dispatch.

use crate::cli;
use crate::handlers;
use argo_types::ArgoResult;

pub(crate) async fn wizard(cmd: cli::cli_wizard_cmds::WizardCmd) -> ArgoResult<()> {
    match cmd {
        cli::cli_wizard_cmds::WizardCmd::Onboard {
            non_interactive,
            config,
            answers,
        } => handlers::wizard_onboard::run(non_interactive, config, answers).await,
    }
}

pub(crate) async fn provider(cmd: cli::ProviderCmd) -> ArgoResult<()> {
    handlers::provider::run(cmd).await
}

pub(crate) async fn db(cmd: cli::DbCmd) -> ArgoResult<()> {
    handlers::db::run(cmd).await
}

pub(crate) async fn workspace(cmd: cli::WorkspaceCmd) -> ArgoResult<()> {
    handlers::workspace::run(cmd).await
}

pub(crate) async fn route(cmd: cli::RouteCmd) -> ArgoResult<()> {
    handlers::route::run(cmd).await
}

pub(crate) fn cred(cmd: cli::CredCmd) -> ArgoResult<()> {
    match cmd {
        cli::CredCmd::Set { key, value } => handlers::cred_set::run(&key, value.as_deref()),
        cli::CredCmd::List { reveal } => handlers::cred_list::run(reveal),
        cli::CredCmd::Clear { key } => handlers::cred_clear::clear(&key),
        cli::CredCmd::ClearAll => handlers::cred_clear::clear_all(),
        cli::CredCmd::Export { shell } => handlers::cred_export::run(&shell),
        cli::CredCmd::Import { shell } => handlers::cred_import::run(&shell),
        cli::CredCmd::Backend { backend } => {
            handlers::cred_migrate_to_keyring::run_backend(&backend)
        }
        cli::CredCmd::MigrateToKeyring { dry_run, from_file } => {
            handlers::cred_migrate_to_keyring::run_migrate(dry_run, from_file)
        }
        cli::CredCmd::EncryptAll { path, json } => {
            handlers::credentials_encrypt_all::run(path, json)
        }
    }
}

pub(crate) async fn backup(cmd: cli::BackupCmd) -> ArgoResult<()> {
    match cmd {
        cli::BackupCmd::Create {
            output,
            include_credentials,
            json,
        } => handlers::backup_create::run(output.as_deref(), include_credentials, json).await,
        cli::BackupCmd::List { json } => handlers::backup_list::run(json).await,
        cli::BackupCmd::Verify { path, json } => handlers::backup_verify::run(&path, json).await,
    }
}

pub(crate) async fn restore(args: cli::RestoreCmd) -> ArgoResult<()> {
    handlers::backup_restore::run(&args.path, args.into.as_deref(), args.dry_run, args.json).await
}

pub(crate) async fn self_update(
    check: bool,
    channel: String,
    force: bool,
    json: bool,
) -> ArgoResult<()> {
    handlers::self_update::run(handlers::self_update::Args {
        check,
        channel,
        force,
        json,
    })
    .await
}

pub(crate) async fn runtime_setup(
    check: bool,
    require: Vec<String>,
    non_interactive: bool,
    json: bool,
) -> ArgoResult<()> {
    handlers::setup::run(check, require, non_interactive, json).await
}

pub(crate) async fn runtime_onboard(
    non_interactive: bool,
    json: bool,
    config: Option<String>,
) -> ArgoResult<()> {
    handlers::onboard::run(non_interactive, json, config).await
}

pub(crate) fn runtime_update() -> ! {
    eprintln!("argo: 'Update' is not yet wired in this build.");
    eprintln!("See ROADMAP.md for the integration-tier plan.");
    std::process::exit(2);
}
