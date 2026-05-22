# Genesis prompt engineer — iterate worker system prompts from role descriptions

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Genesis turns a plain-English role description into a concrete worker
system prompt. Today the crate has `prompt_writer.rs` (single-shot
generation). This brief adds an **iterative prompt engineer** that
accepts operator feedback ("be more conservative", "skip cold-call
phrasing", "add a memory-recall hint") and revises the prompt while
preserving the parts the operator approved.

Mirrors mission claim 3: "Operator describes a role in plain English;
SKYCOT generates the vertical (system prompt, action vocabulary, memory
schema, eval seeds). No hand-authoring TOML."

## Approach (concrete)

1. **NEW** `crates/argo-genesis/src/prompt_engineer.rs` (~320 LOC):
   - `struct PromptEngineerSession { role_description: String, current_prompt: String, history: Vec<PromptRevision> }`
   - `struct PromptRevision { rev: u32, prompt: String, feedback: Option<String>, locked_sections: Vec<String> }`
   - `pub fn start(role_description: &str) -> ArgoResult<PromptEngineerSession>`
     — generates v0 via existing `prompt_writer::write_prompt`
   - `pub fn revise(session: &mut PromptEngineerSession, feedback: &str) -> ArgoResult<&PromptRevision>`
     — emits a new revision; locks any section the operator marked `[KEEP]`
   - `pub fn diff(prev: &PromptRevision, next: &PromptRevision) -> String`
     — unified-diff-style text for CLI display
2. **Extend** `crates/argo-genesis/src/lib.rs` to `pub mod prompt_engineer;`
3. Plug into `pipeline.rs` so `genesis run --interactive` calls the
   engineer instead of single-shot `prompt_writer`. Keep the
   non-interactive path on `prompt_writer` (no behaviour change).

## Files you may touch

- `crates/argo-genesis/src/prompt_engineer.rs` (NEW, ≤ 350 LOC)
- `crates/argo-genesis/src/prompt_engineer_tests.rs` (NEW, ≤ 250 LOC) — 5 tests:
  - start() emits a non-empty prompt for a 1-line role
  - revise() with feedback "be more terse" shortens the prompt
  - locked sections survive a revision
  - 4 revisions in a row do not panic + produce monotonic rev numbers
  - empty role_description → ArgoError::Validation
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: route interactive flag)

## Files you must NOT touch

- `prompt_writer.rs` — keep single-shot path unchanged for callers
- `manifest_writer.rs` — out of scope
- Any vertical TOML

## Acceptance criteria

- [ ] `start()` produces v0 prompt ≥ 200 words for a 50-word role description
- [ ] `revise()` round-trip: feedback "shorter" → next rev shorter than prev
- [ ] Locked-section markers `[KEEP]…[/KEEP]` survive revision verbatim
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-prompt-engineer`
- PR title: `genesis: iterative prompt engineer with feedback loop (claude-genesis-prompt-engineer)`

## Notes for the implementer

- Look at `prompt_writer.rs` and `refine.rs` end-to-end before starting —
  the revise() flow should feel like a natural extension of refine.
- Use `argo-llm` (workspace dep) for the model call; do not introduce a
  new LLM client.
- Reuse `archetype.rs` if the role description matches a known archetype;
  otherwise fall back to free-form generation.
