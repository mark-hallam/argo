//! `argo migrate <subcommand>` — operator-facing data-migration
//! commands. Currently hosts `argo migrate encrypt`
//! (`mission-sqlcipher-3-migrate-cli`), which upgrades an existing
//! unencrypted SQLite database to a SQLCipher-encrypted one with
//! mandatory backup, row-count verification, and an atomic swap.
//!
//! `--key-source keyring` is documented but currently delegates to
//! the `SKYCOT_DB_KEY_BASE64` env-var path for sourcing the raw key
//! bytes — the per-tenant `argo_config::cred::db_key` API from
//! `mission-sqlcipher-2-keyring-wire` lands in parallel and PR 4
//! (`mission-sqlcipher-4-default-flip`) wires it in.

use clap::{Args, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum MigrateCmd {
    /// Convert an unencrypted SQLite database to SQLCipher-encrypted
    /// in place. Creates a timestamped backup under
    /// `~/.argo/backups/`, verifies row counts across every user
    /// table, then atomically swaps the encrypted file in. Idempotent:
    /// running against an already-encrypted DB is a clean no-op.
    #[command(after_help = "Examples:\n  \
        argo migrate encrypt --db ~/.argo/session.db\n  \
        argo migrate encrypt --db ./session.db --key-source env\n  \
        argo migrate encrypt --db ./session.db --dry-run --json")]
    Encrypt(MigrateEncryptArgs),
}

#[derive(Args, Debug, Clone)]
pub struct MigrateEncryptArgs {
    /// Path to the unencrypted SQLite database file to convert.
    #[arg(long)]
    pub db: PathBuf,
    /// Where the raw 32-byte encryption key comes from. `env` reads
    /// `SKYCOT_DB_KEY_BASE64` (base64-encoded 32 bytes). `keyring`
    /// will route through the per-tenant
    /// `argo_config::cred::db_key` API once PR 2 / PR 4 land; today
    /// it falls back to the env-var path with a one-line notice.
    #[arg(long, value_enum, default_value_t = MigrateKeySource::Env)]
    pub key_source: MigrateKeySource,
    /// Print the intended actions without writing anything.
    #[arg(long)]
    pub dry_run: bool,
    /// Emit a `{ok, migrate_encrypt: {...}}` JSON envelope instead of
    /// a status line. Pairs cleanly with the rest of the CLI's
    /// list/show/status JSON contract.
    #[arg(long)]
    pub json: bool,
    /// Skip the interactive "this will rewrite your database"
    /// confirmation. Required for non-interactive contexts (CI,
    /// runbooks, cron). `--dry-run` does not need this.
    #[arg(long)]
    pub yes: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum MigrateKeySource {
    /// Read base64-encoded 32-byte key from `SKYCOT_DB_KEY_BASE64`.
    Env,
    /// OS keyring (Keychain / Secret Service). Currently delegates
    /// to the env-var path; rewired by PR 4
    /// (`mission-sqlcipher-4-default-flip`).
    Keyring,
}
