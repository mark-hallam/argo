# PROTOCOL_BRIEF_TEMPLATE — seed brief

One brief per row in the ledger. By default, one brief = one PR.
Workers read the brief once and follow it exactly. The conductor reads
the brief when reviewing the worker's PR (acceptance checklist) and
when deciding how to unblock a BLOCKED row (may-touch expansion).
Manager-worker mode is an explicit opt-in exception: the parent brief
still has one final PR, while child task state lives in a manager
package under `docs/build-plan/work-packages/`.

Save each brief as `streams/<brief-id>.md`. The `<brief-id>` should
match the ledger row exactly (e.g. `A-executor-trait`, `C-gmail`,
`H-4b`).

---

## The template

Copy this verbatim, fill in the angle-bracket sections, delete this
preamble. The `**Authored against**:` line is a forward-only freshness
anchor (Stream `claude-brief-freshness-header`). Capture the current
`git rev-parse --short origin/main` SHA when authoring; workers feed
that SHA to `scripts/check-brief-freshness.sh` to detect drift before
they claim. Older briefs without the line stay valid; the freshness
script still runs, it just skips the SHA-distance check on those.

```markdown
# Stream <X> — <one-line summary>

**Owner**: <claude | codex | other agent family>.
**Depends on**: <brief-id-N merged | none>.
**Authored against**: origin/main `<short-sha>` on `<YYYY-MM-DD>`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh <brief-id>`.
<optional: critical-path / capstone / preflight tag>

## Scope

<1–2 paragraphs describing what this PR delivers and why. State the
contract this PR is locking, if any (e.g. "this is a CONTRACT PR;
downstream briefs implement against the trait this PR defines —
get the shape right"). State explicitly what is NOT in scope but
might appear to be.>

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

Default to `Manager-worker allowed: no`. If set to `yes`, the manager
must create `docs/build-plan/work-packages/<brief-id>.md` from
`PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md`, child tasks must stay inside
this brief's may-touch list, child status must not be added to
`BUILD_STATUS.md`, auto-spawn requests must fit the spawn budget, and
the conductor remains the only final merge authority.

## Files you may touch

- `<crate/path/to/file.rs>` — <what changes>
- `<crate/path/to/file_tests.rs>` — <what changes>
- `<NEW: crate/path/to/new_file.rs>` — (NEW)
- `<Cargo.toml>` — narrowly scoped: <add ONE dependency line for X.
  No other edits.>
- `<crate/lib.rs>` — narrowly scoped: <add ONE module line. No
  other edits.>

## Files you must NOT touch

- <crate or directory you must stay out of, with a one-line reason
  per entry — usually "owned by Stream Y" or "lock from brief Z">
- Anything not on the may-touch list above

If your task genuinely requires a file not listed, **stop and mark
this brief BLOCKED in BUILD_STATUS.md** with a one-line reason. Do
not silently expand scope.

## Acceptance criteria

- [ ] <checkbox describing one verifiable outcome — type, function,
      test, or behavioral assertion. Each box must be independently
      verifiable from a fresh checkout.>
- [ ] <next box>
- [ ] <gate: project's build command passes>   e.g. `cargo build -p <crate>`
- [ ] <gate: lint command passes>               e.g. `cargo clippy -- -D warnings`
- [ ] <gate: tests pass>                        e.g. `cargo test -p <crate>`
- [ ] Every modified file under <LOC budget>    e.g. < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `<owner>-<brief-id>` — e.g. `claude-A-executor-trait`
- PR title: `<area>: <one-line summary> (<brief-id>)`

## Notes for the implementer

- <implementation hints, gotchas, library choices, naming
  conventions specific to this brief>
- <any locked decisions: "use library X, not Y", "match the
  existing pattern in file Z", "do not introduce a new abstraction
  here">
- <any expected edge cases the worker should test>
```

---

## Sizing rules

- **One brief = one reviewable PR.** If you can't review the PR in
  10 minutes, the brief was too big. Split.
- **Target 200–500 LOC per brief.** Briefs above 500 LOC tend to
  hit conflicts mid-implementation. Briefs below 100 LOC are fine
  but indicate the dependency graph could be coarser.
- **Acceptance criteria: 5–12 boxes.** Fewer than 5 means the
  brief is under-specified; more than 12 means split.
- **The may-touch list should fit on one screen.** If it doesn't,
  the brief is touching too much. Split.

If a brief turns out fat in flight, the worker marks BLOCKED with
reason `too large, request split`. The conductor then sub-slices
the brief into N sub-briefs, each its own row in the ledger
(e.g. `H-1`, `H-2`, `H-3`, `H-4a`, `H-4b`, `H-4c`).

---

## Locking the contract

When a brief defines a shared abstraction (a trait, a schema, an
event type) that downstream briefs depend on, the brief should say
so explicitly:

> This is a CONTRACT PR. Other streams will implement against the
> trait + tests this PR defines. Get the shape right.

Downstream briefs that need to extend the contract (add a new
section to a schema, add a new enum variant) get a "narrowly
scoped" allowance in their may-touch list:

> `<file in contract crate>` — narrowly scoped: add a NEW
> `<NewSection>` (struct + re-export) for the `[new_section]`
> table. Do NOT modify any existing declaration in those files.

This pattern keeps the contract crate stable while allowing
forward-compatible additions.

---

## What a good brief looks like (anonymised example)

A small example brief, sized like Stream C's connector briefs from
the SKYCOT run. Project name replaced with `<vertical>`,
`<connector>` as substitutions.

```markdown
# Stream C — <connector> MCP connector

**Owner**: codex. **Depends on**: C-mcp-wire merged.

## Scope

Implement an MCP server (stdio JSON-RPC) that exposes <connector>
operations as tools the agent can call mid-turn. The connector lives
in its own crate `<vertical>-<connector>`, registers with the agent
runner via the existing `[mcp_servers]` manifest section, and uses
the connection pool wired in C-mcp-wire.

This brief follows the shape locked by C-gmail (the first connector
to land). Reuse the OAuth refresh + token cache pattern from
`<vertical>-gmail/src/oauth.rs`; do not invent a new one.

## Files you may touch

- `crates/<vertical>-<connector>/**` (NEW crate)
- `Cargo.toml` — narrowly scoped: add ONE workspace member entry
  `crates/<vertical>-<connector>`. No other workspace edits.
- `Cargo.lock` (auto-updated by cargo when adding the crate)
- `manifests/connectors/<connector>.toml` (config: api token,
  base url, default scopes)

## Files you must NOT touch

- `crates/<vertical>-mcp/**` (the runtime — locked from C-mcp-wire)
- Any other connector crate
- `crates/<vertical>-vertical/**` (manifest schema — locked from A)
- The `Executor` trait

## Acceptance criteria

- [ ] Crate `<vertical>-<connector>` builds standalone
- [ ] Exposes 5 tools: `search`, `read`, `create`, `update`,
      `<one connector-specific tool>`
- [ ] OAuth refresh path: token cache at
      `<cwd>/.argo/connectors/<connector>/`; refresh token rotation
      on 401
- [ ] Integration test gated on `<CONNECTOR>_TEST_TOKEN` env var
- [ ] Manifest registration in `manifests/connectors/<connector>.toml`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo test -p <vertical>-<connector>` passes
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `codex-C-<connector>`
- PR title: `mcp(connector): <connector> — search / read / create / update / <X> (C-<connector>)`

## Notes for the implementer

- The OAuth pattern from C-gmail is the template. Match it exactly,
  including the env-var names and cache directory layout.
- Don't introduce a new HTTP client — reuse `reqwest` with the
  workspace-pinned version.
- If the <connector> SDK has poor docs, prefer raw HTTP calls.
  Don't pull in a new SDK crate without flagging in BUILD_STATUS.
- The agent dispatch path (C-mcp-wire) auto-discovers tools via
  `list_tools()`; just implement the JSON-RPC handler. No
  registration boilerplate beyond the manifest entry.
```

This is a real-shaped brief: ~50 lines, ~9 acceptance boxes, tight
may-touch list, explicit reuse of an upstream pattern, an explicit
non-goal ("don't introduce a new HTTP client"). A worker can land
this in one PR in one tab.
