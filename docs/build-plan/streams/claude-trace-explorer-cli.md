# claude-trace-explorer-cli — `argo trace explore <session>` walks OTLP trace tree

**Owner**: claude. **Depends on**: `mission-otlp-exporter` (#938) MERGED.

## Scope

`mission-otlp-exporter` exports spans via OTLP/HTTP. Operators currently have
to open Grafana/Tempo to inspect a session's trace tree. This brief adds an
in-CLI walker: `argo trace explore <session>` reconstructs the trace tree
from local span records and renders it as markdown (default) or JSON.

Local span records: the exporter (per #938) writes spans to OTLP **and**
keeps a local NDJSON copy at `.skycot/traces/<session>.ndjson` for
in-CLI inspection without round-tripping a backend. (If #938 does not yet
emit a local copy, this brief adds the local sink as a narrow extension —
search first; do not duplicate.)

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/trace_explore.rs` (~280 LOC):
   - `pub fn run(session: &str, format: TraceFormat, path: &Path) -> ArgoResult<()>`
   - Reads `.skycot/traces/<session>.ndjson`, deserializes each line into
     a `SpanRecord`, builds a tree by `parent_span_id`
   - Markdown render: indented bullet list, one line per span,
     `name (kind, dur=12.3ms, status=ok)`
   - JSON render: nested `{ span, children: [...] }`
2. **Modify** `crates/argo-cli/src/cli.rs`:
   - Add `Trace` enum + `Explore { session, format, path }` variant
3. **Modify** `crates/argo-cli/src/main.rs`:
   - Dispatch arm calling `trace_explore::run`
4. **Modify** `crates/argo-cli/src/handlers/mod.rs`: 1 pub mod line.

## Files you may touch

- `crates/argo-cli/src/handlers/trace_explore.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/trace_explore_tests.rs` (NEW, ≤ 250 LOC)
  - 6 unit tests:
    - empty file → empty tree (not an error)
    - single root span renders
    - parent + child renders nested
    - multi-root (orphans) renders flat
    - circular parent (defensive) does not infinite-loop
    - JSON format round-trips through `serde_json`
- `crates/argo-cli/src/cli.rs` — narrow: 1 subcommand
- `crates/argo-cli/src/main.rs` — narrow: 1 dispatch arm
- `crates/argo-cli/src/handlers/mod.rs` — narrow: 1 pub mod line
- `crates/argo-cli/tests/trace_explore_smoke.rs` (NEW) — 1 integration test
  with a hand-written `.ndjson` fixture

## Files you must NOT touch

- argo-types — no new public types; deserialize a private `SpanRecord` struct
- argo-store schema
- The OTLP exporter wiring itself (#938) — read-only consumer of its sink
- argo-tui

## Acceptance criteria

- [ ] `argo trace explore <session>` prints a markdown tree to stdout
- [ ] `--format json` emits valid JSON parseable by `serde_json`
- [ ] `--path <dir>` overrides the default `.skycot/traces/` location
- [ ] Missing session file → clear error (not panic)
- [ ] Empty session file → empty output, exit 0
- [ ] Circular `parent_span_id` chains do not hang
- [ ] 6 unit tests + 1 smoke test
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-trace-explorer-cli`
- PR title: `trace: argo trace explore session walker (claude-trace-explorer-cli)`

## Notes for the implementer

- Verify the local NDJSON sink exists in #938 before assuming the path. If
  absent, confirm the gap in the PR description and propose adding it as a
  narrow change inside this brief (≤ 80 LOC in the exporter module).
- Keep the `SpanRecord` struct private to `trace_explore.rs` — do not promote
  it to argo-types. If a second consumer appears later, refactor then.
- Markdown output: 2-space indent per depth, max depth 32 (defensive cap).
