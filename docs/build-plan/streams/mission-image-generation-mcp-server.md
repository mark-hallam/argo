# Mission load-bearing wire — image-generation as an MCP server (precursor for vision-needing verticals)

**Owner**: claude. **Depends on**: existing MCP plugin pattern (verify in first 5min via `find crates -name '*.rs' -path '*mcp*'`); existing `argo-mcp` crate (verify path).
**Authored against**: origin/main `da6415b0` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-image-generation-mcp-server`.

> SOURCE: MISSION_BACKLOG.md Section 11 #163 — "Build image generation as MCP server." Verticals like product-manager, marketing-intern, and any AI-receptionist needing avatar/banner generation currently have NO image-creation tool. Mission promise: a thin MCP server `argo-mcp-image-gen` that exposes `image.generate` / `image.edit` via existing provider integrations (Anthropic doesn't generate images yet — wire DALL-E 3 + Stable Diffusion via openai-compatible API).

## Scope

1. **NEW crate** `crates/argo-mcp-image-gen/`:
   - `lib.rs` exposing `ImageGenMcpServer` implementing the standard MCP server trait (verify trait location in first 5min — `argo-mcp::server::McpServer` or similar).
   - Two tools: `image.generate(prompt, size, model) -> ImageResult` and `image.edit(image_bytes, prompt) -> ImageResult`.
   - `ImageResult { url: String, b64_data: Option<String>, model: String, prompt_hash: String }`.
   - Backends: `OpenAiCompatibleBackend` (DALL-E 3 + SD via openai-compat APIs) + `StubBackend` for tests.

2. **Config**: `[mcp.image_gen]` in `argo.toml`:
   ```toml
   [mcp.image_gen]
   backend = "openai"  # or "stable_diffusion" or "stub"
   api_base = "https://api.openai.com/v1"
   model_default = "dall-e-3"
   size_default = "1024x1024"
   ```

3. **Binary**: `crates/argo-mcp-image-gen/src/main.rs` — small entry point that boots the server over stdio (MCP standard transport).

4. **NEW CLI**: `argo mcp image-gen [--config <path>]` — runs the server in the foreground (operator can pipe into Claude Desktop or argo's MCP client).

5. **Capability declaration**: `image:generate` capability for any vertical that wants to use this tool. Verticals declare in their manifest.

6. **Cost guard**: `image.generate` writes a cost-estimate audit row BEFORE the actual API call (estimated_usd_cents per pricing of the model). If `--require-confirm` flag is set on the MCP server boot AND the cost > $0.20, returns `ApprovalRequired` error letting the operator decide.

This is NOT in scope: image storage/CDN integration; image-to-video; per-tenant cost caps (cross-cutting); building our own SD inference server; on-device CoreML/MLX backends.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-mcp-image-gen/Cargo.toml` (NEW)
- `crates/argo-mcp-image-gen/src/lib.rs` (NEW, ≤ 200 LOC)
- `crates/argo-mcp-image-gen/src/server.rs` (NEW, ≤ 250 LOC)
- `crates/argo-mcp-image-gen/src/backend.rs` (NEW, ≤ 200 LOC) — trait + impls
- `crates/argo-mcp-image-gen/src/main.rs` (NEW, ≤ 80 LOC)
- `crates/argo-mcp-image-gen/src/server_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests w/ stub backend
- `Cargo.toml` (workspace) — narrowly scoped: ADD `crates/argo-mcp-image-gen` to workspace `members`
- `crates/argo-types/src/capability.rs` — narrowly scoped: ADD `ImageGenerate` capability variant (verify enum file is right location)
- `crates/argo-cli/src/handlers/mcp_image_gen.rs` (NEW, ≤ 100 LOC) — `argo mcp image-gen` CLI shim

## Files you must NOT touch

- `crates/argo-mcp/src/server.rs` (or wherever the MCP server trait lives) — reuse only
- Any non-mcp/non-cli/non-types crate
- `crates/argo-store/migrations/**` — no schema change
- Existing verticals/ — verticals declare capability separately

## Acceptance criteria

- [ ] New `argo-mcp-image-gen` crate compiles
- [ ] `image.generate` + `image.edit` tools registered with MCP server trait
- [ ] `OpenAiCompatibleBackend` works against a mocked HTTP endpoint (no real API call in tests)
- [ ] `StubBackend` returns deterministic placeholder for tests
- [ ] `argo.toml [mcp.image_gen]` parses; missing → server refuses to start with clear error
- [ ] `argo mcp image-gen` boots the stdio server
- [ ] Cost-estimate audit row written before each API call
- [ ] `--require-confirm` gates calls > $0.20
- [ ] `ImageGenerate` capability variant exists
- [ ] 6 unit tests
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-mcp-image-gen -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-image-generation-mcp-server`
- PR title: `mcp: image generation MCP server (OpenAI-compatible + stub backends) (mission-image-generation-mcp-server)`

## Notes for the implementer

- Use the existing `argo-mcp-connector-shared::retry::retry_with_policy` (MERGED #1301) for transient API failures. Don't add a new retry primitive.
- For the cost-estimate: use a hardcoded table for v1 (DALL-E 3 = 4¢ standard / 8¢ HD per image, etc.). Document the table in module rustdoc.
- The capability declaration is load-bearing — verticals MUST list `image:generate` in their manifest before they can use this tool.
- Don't try to wire image-storage in this brief — that's a separate concern.
- The MCP server trait may live in `argo-mcp` or `argo-mcp-connector-shared`; verify which one in first 5min and document the choice.
