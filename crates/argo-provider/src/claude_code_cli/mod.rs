//! Claude Code CLI subprocess provider.
//!
//! Routes LLM completion calls through `claude --print --output-format json`
//! subprocess, billing against the operator's Claude Code subscription
//! instead of the Anthropic developer API key. Use for cost-controlled
//! dev + demo workflows; production flips routing back to `anthropic.toml`.
//!
//! Brief: `docs/build-plan/streams/provider-claude-code-cli-subprocess.md`.
mod cli_response;
mod driver;
mod prompt_renderer;

// Unix-only: every test in this module spawns the mock `claude` binary
// (`tests/fixtures/mock-claude.sh`, a shell script) as a subprocess. Windows
// cannot exec a `.sh` (`%1 is not a valid Win32 application`), so the whole
// driver suite is gated to Unix. CI (Linux) + macOS still compile + run it.
#[cfg(all(test, unix))]
mod tests;

pub use driver::ClaudeCodeCliTransport;
