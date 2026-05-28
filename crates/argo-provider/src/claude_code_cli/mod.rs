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

#[cfg(test)]
mod tests;

pub use driver::ClaudeCodeCliTransport;
