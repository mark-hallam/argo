# claude-zero-caller-substrate-lint

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `6eb4c1c72` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-zero-caller-substrate-lint`.

🟡 **P2 CI HYGIENE — drift prevention.** Source: evaluator round-3 drift finding (`docs/skycot/evals/scheduled/LATEST.md`):

> "`argo-workplace-knowledge` crate ships with zero `use` consumers. Substrate + audit-kinds merged; the gateway (would-be consumer) is BLOCKED on its own pre-flight refusing to claim. The CLAUDE.md 'wire-or-defer' rule isn't enforced at merge time."

CLAUDE.md drift-prevention §3 states: *"Wire or defer. Every new public surface in a brief names its consumer. If the consumer doesn't exist yet, the brief either includes the wire-up or is paired with a follow-up brief in the same batch."* But it's an authoring convention; not gated at merge.

**The cost when the rule isn't enforced:** PR #2034 (`mission-workplace-knowledge-substrate`) merged a new crate with 0 callers. The downstream consumer brief (`mission-workplace-knowledge-gateway`) is BLOCKED on its own pre-flight check, so the crate sat for 4+ days as substrate-without-runtime. Discoverable via `cargo tree`, not visible to authors.

## Scope

Add a CI lint that fails the pre-merge gate when a PR introduces a new crate (or a new top-level `pub mod`) with **zero consumer `use` lines anywhere in `crates/*/src/`** outside the new crate itself. The check runs on the post-merge state, not the diff — it's a steady-state invariant.

### How the lint works

1. Enumerate every workspace crate via `cargo metadata --format-version 1 --no-deps`.
2. For each crate, count the `use <crate_name>::` references across `crates/*/src/**/*.rs` excluding the crate's own `src/` (use crate `name` from Cargo.toml).
3. Any crate with **zero external `use` references** is flagged.
4. **Allowlist**: crates that are intentionally standalone (root binaries like `argo-cli`, scaffolding crates, integration-test-only crates). Use a comment header at the top of `Cargo.toml`: `# zero-caller-allowed: <reason>`. The lint skips crates with this marker.
5. Exit 1 on any unallowed zero-caller crate; print the crate name + suggested fix (either add a `use`, or add the allowlist marker with a reason).

The lint also catches the inverse case (a `pub mod foo` block inside an existing crate that has no `use crate::foo::` callers — i.e. dead module). Optional in v1; document as a follow-up if it adds too much scope.

### Where to wire it

Add to:
- `scripts/conductor-pre-merge-gate.sh` — runs every conductor admin-merge
- `scripts/workspace-clippy-fast-fail.sh` — runs in every worker's pre-push gate

Both already exist; this brief just adds a `[N/M] zero-caller-substrate check` step to each.

### Allowlisting the legitimate cases

Identify existing zero-caller crates and either wire them up OR add the allowlist marker:
- `argo-cli` is a binary, not consumed by other crates — allowlist.
- Any test-only or scaffolding crate — allowlist with reason.
- `argo-workplace-knowledge` currently fails the lint — leave UNALLOWED so the lint catches the regression. Mission-claim-4 gateway brief (already TODO at Phase A top) will wire it; until then, the lint stays red as a deliberate forcing-function.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: scripts/check-zero-caller-substrate.sh` — the lint script. ~80-120 LOC bash.
- `NEW: scripts/check-zero-caller-substrate-tests.sh` (or inline) — at minimum: smoke test against a fixture with one zero-caller crate + one allowlisted + one normal.
- `scripts/conductor-pre-merge-gate.sh` — narrowly scoped: add a step that invokes the new lint script.
- `scripts/workspace-clippy-fast-fail.sh` — narrowly scoped: add the same invocation.
- Add `# zero-caller-allowed: <reason>` markers to legitimate Cargo.toml files for binaries / scaffolding (do NOT add to argo-workplace-knowledge — leave it failing as the forcing function).
- `docs/build-plan/PROTOCOL.md` — narrowly scoped: add a paragraph under "Drift prevention" pointing at the new lint + how to allowlist.

## Files you must NOT touch

- The `argo-workplace-knowledge` crate (we want it to keep failing the lint until the gateway lands).
- Any other crate's source code (this is a CI lint, not a refactor).
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] `scripts/check-zero-caller-substrate.sh` exists, is executable, exits 1 when ≥1 unallowed crate has zero callers, exits 0 otherwise.
- [ ] Allowlist mechanism via `# zero-caller-allowed: <reason>` Cargo.toml comment works (smoke test).
- [ ] Test fixture covers: (a) one crate with callers (pass), (b) one zero-caller crate without marker (fail), (c) one zero-caller crate with marker (pass).
- [ ] `scripts/conductor-pre-merge-gate.sh` invokes the new lint as part of its multi-step run.
- [ ] `scripts/workspace-clippy-fast-fail.sh` invokes the new lint.
- [ ] Running the lint against current main FAILS with `argo-workplace-knowledge` flagged. This is intentional — the lint becomes the forcing function for the gateway brief.
- [ ] PROTOCOL.md drift-prevention section updated with the new lint reference + allowlist convention.
- [ ] Existing legitimate binaries/scaffolding crates (e.g. `argo-cli`) get `# zero-caller-allowed:` markers so the lint doesn't false-positive on them.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-zero-caller-substrate-lint`
- PR title: `ci(workspace): zero-caller substrate lint — enforce wire-or-defer rule at merge (claude-zero-caller-substrate-lint)`

## Notes for the implementer

- ~150-220 LOC expected. Lint script ~80, allowlist marker plumbing ~30, integration into the two gate scripts ~20, tests ~50, PROTOCOL.md ~30.
- **The lint is intentionally meant to fail on current main**. Don't try to "fix" the workplace-knowledge zero-caller state in this brief. That's the gateway brief's job (already TODO at Phase A top). Once that lands, this lint goes green.
- **For backward-compat**: existing zero-caller crates that are LEGITIMATELY standalone need allowlist markers. Audit before the PR opens — `cargo metadata --no-deps --format-version 1 | jq '.workspace_members'` then for each: count `use <name>::` callers. Add allowlist markers to all true standalone (binaries, dev-only) crates.
- **Don't be too aggressive on the inverse case** (dead pub mod). That's a different lint — punt to a follow-up brief if it adds too much code.
- The lint is a STEADY-STATE check, not a diff-shape check. That's intentional — diff-shape lints miss cases where a brief adds a crate AND its consumer in separate PRs that interleave badly.
- This brief is the evaluator round-3 drift finding's recommended fix. PR body should cite the evaluator report verbatim so the trail from "evaluator finds drift → conductor authors brief → worker fixes" stays visible.
