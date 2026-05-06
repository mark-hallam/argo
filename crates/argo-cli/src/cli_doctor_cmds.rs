//! `argo doctor` subcommands.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DoctorCmd {
    /// Continuously emit the doctor JSON envelope until Ctrl-C.
    Watch {
        /// Poll interval. Supports bare seconds plus `ms`, `s`, and `m` suffixes.
        #[arg(long, default_value = "30s")]
        interval: String,
        /// Clear the terminal before each cycle for a live dashboard redraw.
        #[arg(long)]
        clear: bool,
    },
}
