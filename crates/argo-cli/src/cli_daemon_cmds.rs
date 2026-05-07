//! `argo daemon …` — launchd-agent install/uninstall (V1.4 macOS
//! deployment, Stream `claude-installer-macos`). The agent runs
//! `argo gateway start` at login + restarts it on crash via
//! `KeepAlive`. Plist lives at
//! `~/Library/LaunchAgents/com.skycot.argo.plist` so it loads
//! per-user (no `sudo` needed).
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DaemonCmd {
    /// Render the launchd plist + write it to
    /// `<launch-agents-dir>/com.skycot.argo.plist` + run
    /// `launchctl load` on it. `--dry-run` prints the rendered
    /// plist without writing.
    Install {
        /// Emit `{plist_path, bin, loaded}` as JSON instead of the
        /// human-readable summary.
        #[arg(long)]
        json: bool,
        /// Print the rendered plist + target path without writing
        /// to disk or invoking `launchctl`.
        #[arg(long)]
        dry_run: bool,
        /// Override the path to the `argo` binary written into the
        /// plist's ProgramArguments. Defaults to the running
        /// binary's path (`std::env::current_exe()`).
        #[arg(long)]
        bin: Option<String>,
        /// Override the LaunchAgents directory. Defaults to
        /// `$HOME/Library/LaunchAgents`. Tests use this to point
        /// at a TempDir.
        #[arg(long, value_name = "PATH")]
        launch_agents_dir: Option<String>,
    },
    /// `launchctl unload` the agent + remove the plist file.
    /// Idempotent — succeeds even if the agent isn't installed.
    Uninstall {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_name = "PATH")]
        launch_agents_dir: Option<String>,
    },
}
