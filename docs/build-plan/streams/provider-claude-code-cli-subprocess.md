# provider-claude-code-cli-subprocess

**Priority**: P0 — dev/demo cost-control unlock (route LLM calls through Claude Code subscription, not Anthropic API)
**Source**: 2026-05-29T22:30Z operator-directed cost-control extension
**Authored against**: origin/main `1e075af00`
**Authored by**: claude conductor (interactive session, operator overnight authority)

## Why this brief exists

The platform currently routes ALL LLM calls through `manifests/providers/anthropic.toml` → `https://api.anthropic.com/v1/messages` → bills against the operator's Anthropic developer API key (`sk-ant-api03-...`).

For the **dev + demo + build-phase window**, API billing burns through quota too fast — every Pitch question, every Genesis generation, every worker turn during testing costs real money. The operator's Claude Code subscription has its own quota model (essentially unlimited within fair-use windows) and is already paid-for.

Operator-locked direction (2026-05-29): in dev/demo, route LLM calls through the **Claude Code CLI** as a subprocess (`claude --print --output-format json "<prompt>"`). The CLI uses the operator's Claude Code subscription. Production deploys flip back to `anthropic.toml` and the real API key.

## Architecture

Existing pattern: `crates/argo-provider/src/{anthropic,bedrock,openai_compat,...}` — each provider has a Rust driver that implements the `Provider` trait. The platform's "polyglot core" + "provider manifests not adapters" invariant (per CLAUDE.md) means new providers ship as a TOML manifest + a thin driver.

New pattern for this brief: a SUBPROCESS-based provider. Same `Provider` trait, but the driver spawns a subprocess instead of making an HTTP call.

```rust
// crates/argo-provider/src/claude_code_cli/driver.rs
impl Provider for ClaudeCodeCliProvider {
    async fn complete(&self, req: CompleteRequest) -> CompleteResponse {
        let prompt = render_prompt(&req.system, &req.messages);
        let output = Command::new(&self.claude_binary_path)
            .arg("--print")
            .arg("--output-format").arg("json")
            .arg(&prompt)
            .output()
            .await?;
        let json: ClaudeCliJsonResponse = serde_json::from_slice(&output.stdout)?;
        CompleteResponse {
            text: json.assistant_text,
            model: json.model,
            tokens_used: json.usage,
            ...
        }
    }
}
```

## Scope

### 1. New manifest `manifests/providers/claude-code-cli.toml`

```toml
# Claude Code CLI subprocess provider — dev/demo only.
#
# Routes LLM completion calls through `claude --print --output-format json`
# subprocess, billing against the operator's Claude Code subscription
# instead of the Anthropic developer API key. Use for cost-controlled
# dev + demo + build-phase workflows; production flips routing back to
# anthropic.toml.

[provider]
id = "claude-code-cli"
family = "anthropic"  # same response shape, different transport
display_name = "Claude Code CLI (subprocess)"
transport_kind = "subprocess"  # NEW — differentiates from HTTP providers

[subprocess]
binary_path_env = "ARGO_CLAUDE_CODE_BINARY"  # defaults to `which claude`
binary_path_fallback = "claude"
default_args = ["--print", "--output-format", "json"]
timeout_ms = 30000
streaming = false  # claude --print is one-shot; streaming added in v1.1

[models]
# Claude Code subscription routes to the latest available Claude.
# We declare these as advisory — the CLI picks the model.
default = "claude-sonnet-4-6"
available = ["claude-sonnet-4-6", "claude-opus-4-7", "claude-haiku-4-5"]
```

### 2. New driver crate module `crates/argo-provider/src/claude_code_cli/`

Files:
- `mod.rs` — module surface
- `driver.rs` — implements the `Provider` trait; spawns `claude` subprocess; parses stdout JSON
- `prompt_renderer.rs` — translates the platform's `(system, messages)` shape into the single-prompt string the CLI accepts (`<system>\n\n<user>: <text>\n<assistant>:`)
- `cli_response.rs` — typed struct for the CLI's JSON output (fields: `assistant_text`, `model`, `usage.input_tokens`, `usage.output_tokens`, `error`)
- `tests.rs` — mocks the `claude` binary with a small bash script that emits known JSON; pins parsing + error handling

Target: ~250 LOC Rust.

### 3. Routing config

Add to `argo.toml` (or `.argo/env.local`) the routing-default knob:

```toml
[providers]
# In dev/demo: route LLM calls through the Claude Code CLI subprocess.
# Switch to "anthropic" for production deploys to use the real API key.
default = "claude-code-cli"
```

When the gateway picks a provider for an LLM completion, it reads this default and dispatches to the named provider. No code change needed in consumers.

### 4. Streaming caveat

`claude --print` is one-shot (no streaming). The platform's `Provider` trait currently supports streaming (`Provider::complete_stream`). For v1 of this provider:

- Implement `complete()` (one-shot) — returns full response when the subprocess exits
- `complete_stream()` returns the whole response as a single chunk (fake-stream)
- Document the limitation in the manifest

For demo Pitch Agent, this is fine — responses are short (1-3 sentences) and waiting 1-2s for the subprocess is acceptable.

v1.1 could add real streaming by switching to `claude --output-format stream-json` and parsing line-delimited JSON. Out of scope here.

### 5. Tests

- Mock binary test: bash script in `tests/fixtures/mock-claude.sh` emits a known JSON; driver test asserts the parsing
- Error-path test: mock binary exits non-zero → driver returns proper `ProviderError`
- Timeout test: mock binary sleeps longer than timeout → driver aborts subprocess
- Integration test: end-to-end through the gateway with `default = "claude-code-cli"` (uses mock binary path via env override)

Target: 8-10 tests, ~200 LOC.

### 6. Documentation

Add `docs/skycot/operator/provider-routing.md` covering:
- Production: `anthropic.toml` + `ANTHROPIC_API_KEY` in env
- Dev/demo: `claude-code-cli.toml` + Claude Code CLI installed
- Switching between them (single config line in `argo.toml`)
- Cost model differences

## Acceptance

- [ ] New `manifests/providers/claude-code-cli.toml` ships
- [ ] New `crates/argo-provider/src/claude_code_cli/` module implements `Provider` trait
- [ ] Mock-binary tests pass (~8-10 cases)
- [ ] Integration test: gateway routes LLM call through the subprocess driver when `default = "claude-code-cli"` is set
- [ ] Real-binary smoke test: with actual `claude` CLI installed, dispatch one call end-to-end via the gateway, observe response shape matches the existing `anthropic` provider's response shape
- [ ] Documentation page added
- [ ] Workspace clippy clean
- [ ] No regression in existing provider tests (`anthropic`, `openai-compat`, etc.)

## Out of scope

- Streaming responses (v1.1 — `--output-format stream-json` parsing)
- Multi-instance subprocess pool (single subprocess per request for v1; v1.1 could pool)
- Cost-tracking integration with the platform's billing surface (the CLI's billing is opaque from outside — show "claude-code-cli" as the provider in audit and let the operator reconcile manually)
- Auto-fallback to `anthropic.toml` if the CLI subprocess fails (explicit operator config flip required)

## May-touch

- **Rust new**: `crates/argo-provider/src/claude_code_cli/{mod,driver,prompt_renderer,cli_response,tests}.rs`
- **Rust edit**: `crates/argo-provider/src/lib.rs` (declare new module + re-export driver)
- **Rust edit**: `crates/argo-provider/src/manifest/mod.rs` (add `transport_kind = "subprocess"` variant + dispatch)
- **Config**: `manifests/providers/claude-code-cli.toml` (NEW)
- **Tests fixtures**: `crates/argo-provider/tests/fixtures/mock-claude.sh` (NEW — chmod +x)
- **Docs**: `docs/skycot/operator/provider-routing.md` (NEW)

## Why this is the right v1 design

- **Same `Provider` trait** — zero downstream change in consumers (Pitch, Genesis, agent_runner, forge, evals all unchanged)
- **Manifest-driven** — flipping production back to API is one config line
- **Subprocess-based** — the `claude` CLI is already installed on the operator's machine (used by the swarm workers); no extra setup
- **Cost-controlled** — Claude Code subscription has fair-use quota; effectively unlimited for normal dev/demo volume
- **Reversible** — production deploys default back to `anthropic.toml`; this provider is dev/demo-flavored

## Operator overnight authority

Per 2026-05-29T22:30Z grant: `--skip-clippy` lane authorized; full worker-side verification (clippy + tests + workspace-clippy clean) required before merge.
