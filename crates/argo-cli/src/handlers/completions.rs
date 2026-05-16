//! `argo completions <shell>` — emit a shell-completion script to
//! stdout for the named shell (mission-cli-shell-completions).
//!
//! Operators redirect the output to the shell's loader path; we keep
//! the handler strictly stdout-only so `argo completions bash > file`
//! produces a clean script with no warning lines interleaved.

use argo_types::ArgoResult;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::Cli;

/// Bin name passed to `clap_complete::generate`. The generated
/// completion scripts hard-code this string as the command to
/// complete, so it must match the `[[bin]] name` in Cargo.toml.
const BIN_NAME: &str = "argo";

pub fn run(shell: Shell) -> ArgoResult<()> {
    write_to(shell, &mut io::stdout());
    Ok(())
}

/// Test seam: render to any `Write` so unit tests can capture the
/// generated script without spawning the binary.
pub(crate) fn write_to<W: io::Write>(shell: Shell, out: &mut W) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, BIN_NAME, out);
}
