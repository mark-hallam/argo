//! Release-management CLI surface — `argo self-update` + `argo completions`.
//! Flattened back into the top-level `Command` enum via `#[command(flatten)]`
//! so the `argo --help` subcommand list is identical pre/post split.

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ReleaseCmd {
    /// Verified binary self-upgrade (Stream `codex-self-update-cli`,
    /// V1.4). Queries GitHub Releases for a newer build, downloads
    /// the platform asset + detached Ed25519 signature, verifies
    /// against the embedded release pubkey, and atomically replaces
    /// the running binary. Best-effort gateway daemon restart.
    #[command(after_help = "Examples:\n  \
        argo self-update --check\n  \
        argo self-update\n  \
        argo self-update --channel beta --force --json")]
    SelfUpdate {
        /// Report the latest available version without installing.
        #[arg(long)]
        check: bool,
        /// Release channel: `stable` (default) or `beta` (allows
        /// pre-release tags).
        #[arg(long, default_value = "stable")]
        channel: String,
        /// Skip the "already on latest" short-circuit and reinstall.
        /// Never bypasses signature verification.
        #[arg(long)]
        force: bool,
        /// Emit a JSON envelope instead of human-readable status.
        #[arg(long)]
        json: bool,
    },
    /// Print a shell completion script for `argo` to stdout
    /// (Stream `mission-cli-shell-completions`). Pipe to the
    /// shell's completion-loader path:
    ///
    /// ```text
    /// argo completions bash > /usr/local/etc/bash_completion.d/argo
    /// argo completions zsh  > "${fpath[1]}/_argo"
    /// argo completions fish > ~/.config/fish/completions/argo.fish
    /// ```
    Completions {
        /// Target shell — `bash`, `zsh`, `fish`, `powershell`, or
        /// `elvish`. Validated by `clap_complete::Shell`.
        shell: clap_complete::Shell,
    },
}
