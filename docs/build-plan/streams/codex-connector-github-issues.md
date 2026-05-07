# V1.2 — GitHub Issues MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire (#806) merged.

## Why

Native GitHub-Issues surface — separate concern from forks/PRs
— so agents can triage and label issues with explicit approval
gates. Mirrors zendesk/discord pattern.

## Scope

GitHub Issues connector. Auth via PAT (`GITHUB_TOKEN`) against
`https://api.github.com`.

- Methods: `list_issues` (repo-scoped, state filter),
  `get_issue`, `create_issue`, `update_issue`, `add_comment`,
  `close_issue`, `label_issue` (add/remove labels)
- Read methods autonomous-safe; create/update/close/comment/
  label all approval-gated

## Approach

REST client wrapping `reqwest` with `Authorization: Bearer
<token>` and `Accept: application/vnd.github+json` headers.
This crate is **scoped to the issues surface only** — does NOT
overlap with any future PR/forks connector. Each method =
one method on `client.rs`; `service.rs` applies the approval
gate.

## Files you may touch

NEW crate at `crates/argo-mcp-connector-github-issues/`
mirroring `argo-mcp-connector-zendesk`:
- `crates/argo-mcp-connector-github-issues/Cargo.toml` (NEW)
- `crates/argo-mcp-connector-github-issues/src/lib.rs` (NEW)
- `crates/argo-mcp-connector-github-issues/src/main.rs` (NEW)
- `crates/argo-mcp-connector-github-issues/src/auth.rs` (NEW)
- `crates/argo-mcp-connector-github-issues/src/client.rs` (NEW)
- `crates/argo-mcp-connector-github-issues/src/schemas.rs` (NEW)
- `crates/argo-mcp-connector-github-issues/src/service.rs` (NEW)
- `Cargo.toml` (workspace) — narrowly: add the new crate to
  the `members` array

NO CLI subcommand — connectors are invoked via the MCP
manifest's `[mcp_servers]` section.

## Files you must NOT touch

- Any other connector crate (including any future
  `argo-mcp-connector-github-pr` — keep boundaries clean)
- `crates/argo-cli/` and `crates/argo-gateway/`
- Workspace `Cargo.lock` is regenerated on first build —
  matches #951/#953 boundary pattern (don't hand-edit)

## Acceptance criteria

- [ ] All 7 methods callable end-to-end
- [ ] Bearer auth via `GITHUB_TOKEN`
- [ ] `User-Agent` header set (GitHub requires it)
- [ ] Approval-gate marks correct (read=false, write=true)
- [ ] 4–8 unit tests + 1 wire test against a recorded fixture
- [ ] Each `src/*.rs` ≤ 500 LOC (HARD RULE 6)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR

- Branch: `codex-connector-github-issues`
- PR title: `mcp: native GitHub Issues connector (codex-connector-github-issues)`

## Notes

- ~700 LOC total across 6 src files plus Cargo.toml.
- `close_issue` = PATCH `/repos/{o}/{r}/issues/{n}` with
  `{"state":"closed"}` — keep as a distinct method.
- `label_issue` should accept `add` and `remove` arrays;
  use POST `/labels` for add and DELETE `/labels/{name}`
  for remove.
