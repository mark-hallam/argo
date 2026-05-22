# tool-todo-and-clarify — Two small in-core tools: `todo` and `clarify`

**Owner**: claude. **Depends on**: nothing. **Tier A — Agent tool
surface.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Hermes parity: long-running multi-step tasks benefit dramatically
from an explicit task list the model maintains itself, plus a
clean way to ask the operator for clarification mid-run. `todo`
keeps a structured per-session list; `clarify` reuses the existing
Ask/Approval surface to pause for an answer. ~200 LOC combined.

## Approach (concrete)

1. **`todo`** — `crates/argo-tool/src/tools/todo.rs`. Schema:
   `todo(action: "add"|"list"|"complete"|"remove", item?: string,
   id?: u64)`. State: per-session JSON file at
   `<workspace_root>/.argo/sessions/<session_id>/todo.json`
   (auto-created). Each item: `{ id: u64, text: string, done: bool,
   created_at: iso8601 }`. `add` returns the new id; `list` returns
   the array; `complete` flips `done`; `remove` deletes by id.
   Pure file IO; no DB schema migration required (keeps brief small).
2. **`clarify`** — `crates/argo-tool/src/tools/clarify.rs`. Schema:
   `clarify(question: string, options?: string[])`. Implementation
   wraps the existing Ask/Approval pathway used by other gated
   actions (locate by grepping for the existing approval enqueue
   call site, e.g. in `argo-runtime` or wherever
   `human_approval_required` is honored). Tool returns the
   operator's chosen answer string (or chosen option index). If
   the approval surface isn't directly callable from a tool today,
   BLOCK with a one-line reason — do not invent a new surface.
3. Register both in `crates/argo-tool/src/tools/mod.rs`
   `builtin_tools` / `builtin_tools_with_memory`.
4. Add `todo` (always-allowed, low-risk) to **two** vertical
   manifests; add `clarify` to one (suggest `recruiter`, since
   clarification mid-screening is high-value).
5. Capability gating: both via existing
   `Capability::ToolDispatch{tool_name_glob: "todo"}` and
   `"clarify"`. No enum change.
6. Tests:
   - 3+ unit on `todo`: add → list, complete flips state, remove
     deletes, list-when-empty.
   - 3+ unit on `clarify`: schema validation, options round-trip,
     no-options free-form path. (Use a stub approval surface in
     tests so no operator interaction is required.)
   - 1 integration: end-to-end `todo` add+list+complete sequence
     across two ToolContext invocations proving file persists.

## Files you may touch

- `crates/argo-tool/src/tools/todo.rs` (NEW, ≤ 200 LOC)
- `crates/argo-tool/src/tools/clarify.rs` (NEW, ≤ 150 LOC)
- `crates/argo-tool/src/tools/mod.rs` (mod + register in two arrays)
- `crates/argo-tool/src/lib.rs` (re-export both)
- Up to two vertical manifests (single-line `allowed` edits each)
- `crates/argo-tool/tests/todo_clarify_integration.rs` (NEW, ≤ 200 LOC)

## Files you must NOT touch

- DB / migration files in `crates/argo-store/`
- Any other tool file
- `crates/argo-types/src/capability.rs`
- The Ask/Approval implementation itself (call it; don't refactor it)

## Acceptance criteria

- [ ] Capability gating documented for both tools
- [ ] At least one vertical's manifest allows each new tool
- [ ] 3+ unit tests per tool (6+ total)
- [ ] 1 integration test
- [ ] Rustdoc with example for each tool
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-tool` green
- [ ] Workspace clippy final-sanity clean
- [ ] Public API documented

## Branch + PR

- Branch: `tool-todo-and-clarify`
- PR title: `tool-todo-and-clarify — todo list + clarify tools for multi-step work`
