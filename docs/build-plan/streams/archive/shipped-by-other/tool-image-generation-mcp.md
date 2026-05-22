# tool-image-generation-mcp — MCP server exposing `generate_image` with three backends

**Owner**: claude. **Depends on**: nothing. **Tier A — Agent tool surface.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Hermes parity: agents currently cannot synthesise images. Closes
MISSION_BACKLOG #163 / #113. Image generation belongs out-of-core
(third-party HTTP, key rotation, latency) so it ships as a new MCP
connector crate, mirroring the proven Zendesk-connector shape. Three
configurable backends keep the operator free of vendor lock-in.

## Approach (concrete)

1. Create `crates/argo-mcp-connector-image-gen/` mirroring the
   `argo-mcp-connector-zendesk/` layout (`Cargo.toml`, `src/lib.rs`,
   `src/main.rs`, `src/auth.rs`, `src/client.rs`, `src/schemas.rs`,
   `src/service.rs`).
2. Expose one tool: `generate_image(prompt: string, style?: string,
   size?: string, backend?: string)`. Backend selection follows the
   Zendesk env pattern: `IMAGE_GEN_BACKEND` env (`openai` | `stability`
   | `replicate`) wins over arg; default = `openai` when
   `OPENAI_API_KEY` is set, else first backend with creds, else
   schema-only error.
3. Implement three thin HTTP clients in `client.rs`:
   - OpenAI DALL-E 3: `POST https://api.openai.com/v1/images/generations`
   - Stability AI: `POST https://api.stability.ai/v2beta/stable-image/generate/core`
   - Replicate: `POST https://api.replicate.com/v1/predictions` + poll
   Each returns bytes; the service writes to
   `<workspace_root>/.argo/generated/<sha256>.png` and returns
   `{ url: "file://<path>", backend: "<name>", bytes: N }`.
4. Register the connector with `argo-mcp` so `argo mcp list` shows it.
   Add a sample manifest entry in one vertical (recruiter or product
   manager) under `[tools].allowed = [..., "image-gen.generate_image"]`.
5. Capability gate: dispatch path uses
   `Capability::ToolDispatch{tool_name_glob: "image-gen.*"}`. Document
   per-backend NetEgress requirements in the connector's rustdoc.
6. Tests: 3 unit tests (one per backend, with `wiremock`-style HTTP
   stub) covering happy path + auth-missing error. 1 integration test
   end-to-end against a mock OpenAI server proving file-on-disk +
   returned URL match.

## Files you may touch

- `crates/argo-mcp-connector-image-gen/Cargo.toml` (NEW, ≤ 40 LOC)
- `crates/argo-mcp-connector-image-gen/src/lib.rs` (NEW, ≤ 40 LOC)
- `crates/argo-mcp-connector-image-gen/src/main.rs` (NEW, ≤ 40 LOC)
- `crates/argo-mcp-connector-image-gen/src/auth.rs` (NEW, ≤ 100 LOC)
- `crates/argo-mcp-connector-image-gen/src/client.rs` (NEW, ≤ 400 LOC)
- `crates/argo-mcp-connector-image-gen/src/schemas.rs` (NEW, ≤ 150 LOC)
- `crates/argo-mcp-connector-image-gen/src/service.rs` (NEW, ≤ 150 LOC)
- `crates/argo-mcp-connector-image-gen/tests/integration.rs` (NEW, ≤ 200 LOC)
- `Cargo.toml` (workspace `members =` list — add the new crate)
- One vertical manifest under `verticals/<chosen>/vertical.toml`
  (single line in `[tools].allowed`)

## Files you must NOT touch

- Any other `crates/argo-mcp-connector-*` crate
- `crates/argo-tool/`, `crates/argo-capability/`, `crates/argo-types/`
- Any other vertical manifest

## Acceptance criteria

- [ ] Capability declaration documented (uses existing `ToolDispatch`)
- [ ] At least one vertical's manifest allows `image-gen.generate_image`
- [ ] 3+ unit tests (one per backend)
- [ ] 1 integration test against mock HTTP backend
- [ ] Rustdoc on `lib.rs` with at least one example
- [ ] `cargo clippy -p argo-mcp-connector-image-gen --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-mcp-connector-image-gen` green
- [ ] Workspace clippy final-sanity clean
- [ ] Public API documented; backend env-key matrix in lib.rs rustdoc

## Branch + PR

- Branch: `tool-image-generation-mcp`
- PR title: `tool-image-generation-mcp — MCP server exposing generate_image with three backends`
