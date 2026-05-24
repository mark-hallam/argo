# eval-argo-runtime-vs-agent-runner-clarify

**Owner**: claude.
**Depends on**: none (read-only audit; produces report + 1 surface pin).
**Authored against**: origin/main `0bf75c1a1` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-runtime-vs-agent-runner-clarify`.

🟡 **P2 SUBSTRATE-CLARITY.** The gap manifest flags `argo-runtime` as 🔴 NOT AUDITED with the note: *"Relationship to argo-agent-runner unclear; may be deprecated or a wrapper."* This brief produces a one-pager that resolves the ambiguity and pins the answer with a surface contract so future drift is detectable.

## Why this is P2

Per gap manifest line 106:
> | `argo-runtime` | All | 🔴 NOT AUDITED | Relationship to argo-agent-runner unclear; may be deprecated or a wrapper. | 🟡 P2 — clarify. |

`argo-runtime` is 6289 LOC across 14 files (action_throttle, anomaly, context_compress, context_truncate, dispatch_gate, prompt_cache_affinity, worker_resource_cage, etc.). `argo-agent-runner` is the canonical worker runtime. Neither doc nor MISSION.md explains the split. Two workers (one editing each crate) will pick contradictory homes for a new safety primitive unless this is resolved.

## Scope

**In:**
- Read pass on every `argo-runtime` source file + its callsites in the workspace (`rg 'argo_runtime::' crates/`).
- Read pass on every `argo-agent-runner` source file referenced from `argo-runtime` (and vice versa).
- Produce: `docs/skycot/evals/scheduled/2026-05-25/eval-argo-runtime-vs-agent-runner-clarify.md` answering:
  - What is `argo-runtime` actually responsible for? (one-paragraph mission statement)
  - What is `argo-agent-runner` actually responsible for? (one-paragraph mission statement)
  - Which crate should own NEW: (a) a safety primitive, (b) a per-turn budget cap, (c) a session-lifecycle hook? (One sentence each.)
  - Are there parallel surfaces (same-named structs/enums in both crates)? Run `rg 'pub (struct|enum|type) ' crates/argo-runtime/src/ crates/argo-agent-runner/src/` and diff.
  - Is `argo-runtime` deprecated? If yes, what's the migration plan?
- Add 1 pin test at `crates/argo-runtime/tests/surface_contract.rs` that locks the current pub surface (re-exports + pub structs + pub fns) so future drift is loud — same pattern as #2115, #2132, etc.
- Update gap-manifest row 106 with the answer + cite the new report.

**Out:**
- Refactoring or merging the two crates (this is a doc + pin only).
- Renaming anything.

## Acceptance

- Report file exists with the 4 questions answered.
- Surface pin test compiles + passes.
- Gap-manifest row updated.
- `cargo clippy -p argo-runtime --all-targets -- -D warnings` clean (test-only addition).

## Branch + PR

- Branch: `claude-eval-argo-runtime-vs-agent-runner-clarify`
- PR title: `eval — argo-runtime — clarify relationship to argo-agent-runner + pin surface`

## Notes for the implementer

- ~200-400 LOC expected (mostly the report; pin test ~50-100 LOC).
- Cite the gap manifest row verbatim in the PR body.
- If you find argo-runtime is a thin deprecated wrapper, also author a follow-up brief `mission-argo-runtime-deprecate-or-merge` and reference it in the report.
- If you find argo-runtime is the load-bearing one and argo-agent-runner is the wrapper, reverse it — but flag clearly because that's a load-bearing inversion.
