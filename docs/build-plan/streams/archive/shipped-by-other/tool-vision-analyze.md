# tool-vision-analyze — In-core `vision_analyze` tool routing through provider multimodal

**Owner**: claude. **Depends on**: nothing. **Tier A — Agent tool surface.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Hermes parity: workers cannot read screenshots, photos, or
PDF-as-image without a custom tool. `vision_analyze` lets any worker
ask a question grounded in an image by routing through the provider's
native multimodal surface (Anthropic image blocks, OpenAI vision
content, Bedrock Claude vision). Closes the "agent eyes" gap.

## Approach (concrete)

1. Create `crates/argo-tool/src/tools/vision.rs` with a `Vision` tool
   exposing `vision_analyze(image: string, question: string)`. `image`
   accepts a `file://` path, an absolute filesystem path, an
   `https://` URL, or a `data:` URL. Path inputs go through the
   existing `path_guard` to enforce workspace-root containment.
2. The tool resolves the image to bytes + mime, then dispatches via
   the existing provider trait. Provider routing reuses the agent's
   currently-bound provider (read from `ToolContext`); no new provider
   plumbing. Each provider knows how to embed an image part:
   - Anthropic: `messages.create` with an `image` content block
     (`source.type = base64` for local, `source.type = url` for https).
   - OpenAI: `chat.completions` `image_url` content part.
   - Bedrock Claude: same body shape as Anthropic via the Bedrock
     adapter.
3. Returns `{ answer: string, provider: string, tokens_used: u32 }`.
4. Register in `crates/argo-tool/src/tools/mod.rs` `builtin_tools` /
   `builtin_tools_with_memory` arrays so it appears alongside
   `web_search` etc.
5. Add `vision_analyze` to one vertical manifest (suggest
   `customer-support`) under `[tools].allowed` so a worker can invoke
   it on attached screenshots.
6. Tests:
   - 3+ unit: image-from-path happy path (mock provider), https-URL
     happy path, malformed image → clean error.
   - 1 integration: end-to-end with a stub provider proving the
     content block is shaped per provider.

## Files you may touch

- `crates/argo-tool/src/tools/vision.rs` (NEW, ≤ 250 LOC)
- `crates/argo-tool/src/tools/mod.rs` (mod registration + add to two
  builtin_tools arrays only)
- `crates/argo-tool/src/lib.rs` (re-export `Vision`)
- `crates/argo-tool/Cargo.toml` (image-decoding dep if needed, e.g.
  `image` or rely on existing `mime_guess`)
- One vertical manifest (`verticals/customer-support/vertical.toml`
  single-line `allowed` edit)
- Test files inside `crates/argo-tool/tests/` (NEW integration test
  file ≤ 200 LOC)

## Files you must NOT touch

- Provider crates (`crates/argo-provider*`) — reuse the existing trait
- Any other tool file in `crates/argo-tool/src/tools/`
- `crates/argo-types/src/capability.rs` — gate via existing
  `Capability::ToolDispatch{tool_name_glob: "vision_analyze"}`

## Acceptance criteria

- [ ] Capability declaration via existing `ToolDispatch` documented in rustdoc
- [ ] At least one vertical's manifest allows `vision_analyze`
- [ ] 3+ unit tests
- [ ] 1 integration test
- [ ] Rustdoc with example
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` green
- [ ] Workspace clippy final-sanity clean
- [ ] Public API documented (`Vision`, schema)

## Branch + PR

- Branch: `tool-vision-analyze`
- PR title: `tool-vision-analyze — in-core vision_analyze tool over provider multimodal`
