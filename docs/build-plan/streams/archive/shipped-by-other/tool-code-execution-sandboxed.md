# tool-code-execution-sandboxed — In-core `code_execute` tool dispatching to argo-sandbox

**Owner**: claude. **Depends on**: nothing (`argo-sandbox` already
exists with `docker` + `local` backends). **Tier A — Agent tool
surface.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Hermes parity (mirrors `code_execution_tool.py`): agents need to run
ad-hoc code (data crunching, format conversion, quick scripts)
without polluting the host. Routing through `argo-sandbox` reuses the
existing isolation layer and keeps the host clean.

## Approach (concrete)

1. **Verify backend.** `crates/argo-sandbox/src/{lib,sandbox,docker,local}.rs`
   exists. Operator brief mentions E2B Firecracker; the in-tree
   sandbox today is docker + local. Use what's there; document this
   as a sensible deviation in the PR body and call out E2B as a
   future backend.
2. Create `crates/argo-tool/src/tools/code_execute.rs` with a
   `CodeExecute` tool exposing
   `code_execute(language: string, source: string, files?: object)`.
   v1 languages: `python`, `node`, `bash`. Unknown language → clean
   schema error.
3. Implementation: each language maps to an `argo-sandbox::Sandbox`
   invocation that:
   - writes `source` to a tmp file in the sandbox workspace
   - writes any `files` (filename → contents map) alongside
   - runs `python3 main.py` / `node main.js` / `bash main.sh`
   - captures stdout/stderr/exit, snapshots created files
4. Returns `{ stdout, stderr, exit, created_files: [{path, bytes}] }`.
   Truncate stdout/stderr at 64 KiB with a `truncated: true` marker.
5. Register in `crates/argo-tool/src/tools/mod.rs` `builtin_tools` /
   `builtin_tools_with_memory`.
6. Add to one vertical manifest (suggest `product-manager` or a new
   `data-analyst`-flavoured one if present) under `[tools].allowed`.
7. Capability gate: this tool needs both
   `Capability::ToolDispatch{tool_name_glob: "code_execute"}` AND
   `Capability::Spawn{spawn_kind: SpawnKind::Container}` — document
   that the dispatch path verifies BOTH before invocation. The check
   itself reuses the existing capability engine; no enum change.
8. Tests:
   - 3+ unit: python hello-world, node hello-world, bash echo. Use
     the `local` sandbox backend in tests for determinism (docker
     unavailable in CI).
   - 1 integration: python script that writes a created file,
     proving `created_files` round-trip.

## Files you may touch

- `crates/argo-tool/src/tools/code_execute.rs` (NEW, ≤ 300 LOC)
- `crates/argo-tool/src/tools/mod.rs` (mod + register in two arrays)
- `crates/argo-tool/src/lib.rs` (re-export `CodeExecute`)
- `crates/argo-tool/Cargo.toml` (add `argo-sandbox` dep)
- One vertical manifest (single-line `allowed` edit)
- `crates/argo-tool/tests/code_execute_integration.rs` (NEW, ≤ 250 LOC)

## Files you must NOT touch

- `crates/argo-sandbox/` source (use as-is; if a missing API blocks
  you, BLOCK with a one-line reason rather than expanding scope)
- Any other tool file
- `crates/argo-types/src/capability.rs`

## Acceptance criteria

- [ ] Capability gating documented (ToolDispatch + Spawn::Container)
- [ ] At least one vertical's manifest allows `code_execute`
- [ ] 3+ unit tests (one per language)
- [ ] 1 integration test
- [ ] Rustdoc with example
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` green
- [ ] Workspace clippy final-sanity clean
- [ ] Public API documented; deviation note ("E2B not yet wired")
  recorded in PR body

## Branch + PR

- Branch: `tool-code-execution-sandboxed`
- PR title: `tool-code-execution-sandboxed — in-core code_execute via argo-sandbox`
