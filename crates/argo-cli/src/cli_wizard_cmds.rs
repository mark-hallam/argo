//! `argo wizard` — first-operator end-to-end setup wizard.
//!
//! `argo wizard onboard` is the single command that replaces the
//! five-step `argo setup` → hand-edit `argo.toml` → `argo new`
//! sequence. Mission brief: `mission-operator-onboarding-wizard`.
//! Lives in its own file so the parent `Command` enum stays under
//! the 500-LOC ceiling.
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum WizardCmd {
    /// Walk a fresh operator through detection → provider key →
    /// operator identity → first vertical → first worker. Runs
    /// interactively against stdin/stdout by default; non-interactive
    /// mode reads pre-populated answers from a TOML file (`--answers
    /// <path>`). Idempotent: a populated `[operator]` block in
    /// `argo.toml` prompts the operator to overwrite (default No).
    Onboard {
        /// Skip prompts; read every answer from `--answers <path>`.
        /// Errors out if any required answer is missing or invalid.
        #[arg(long)]
        non_interactive: bool,
        /// Override the default `<cwd>/argo.toml` path. Useful for
        /// tests and for operators with multiple workspaces on one
        /// machine.
        #[arg(long, value_name = "PATH")]
        config: Option<String>,
        /// TOML file with pre-populated answers for `--non-interactive`.
        /// Schema: flat keys mirroring the prompt order — see
        /// `docs/skycot/runbook/wizard-onboard.md`.
        #[arg(long, value_name = "PATH")]
        answers: Option<String>,
    },
}
