# eval-vertical-regression-cadence

Date: 2026-05-24
Brief: `eval-vertical-regression-cadence`
Crate: `argo-vertical-regression`

## Verdict

`argo-vertical-regression` is a real static regression harness, but it is
manual-only today. It is in the workspace, has a runnable integration test, and
is documented in the operator runbook. I found no CI, pre-commit, Genesis, or
production caller that runs it automatically for vertical edits.

This means the substrate is useful and testable, but not a live automated drift
gate.

## Public Surface

Top-level public items exported from `src/lib.rs`:

| Item | Purpose |
|---|---|
| `VerticalCase` | Parsed manifest plus fixtures for one role-bearing vertical. |
| `Fixture` | One parsed JSONL eval fixture. |
| `Violation` | Drift finding: undeclared action type or approval mismatch. |
| `discover` | Walks `verticals/<id>/`, parses manifests, loads fixtures. |
| `check` | Cross-checks one `VerticalCase` against its manifest action vocabulary and approval policy. |
| `partition` | Separates new drift from the private `KNOWN_DRIFT` baseline and detects stale baseline rows. |
| `workspace_root` | Resolves the workspace root for integration tests. |

The new `cadence_contract.rs` test pins this inventory by reading `src/lib.rs`
and comparing the top-level `pub` declarations to the expected set.

## Current Consumers

- `crates/argo-vertical-regression/tests/regression_all.rs` is the only
  workspace-level executable consumer. It calls `discover`, `check`,
  `partition`, and `workspace_root`.
- `crates/argo-vertical-regression/src/tests.rs` covers local unit behavior.
- `docs/skycot/runbook/per-vertical-regression.md` documents the manual command:
  `cargo test -p argo-vertical-regression --test regression_all`.
- `docs/skycot/RUNBOOK.md` links to the per-topic runbook.
- The crate is listed in workspace `Cargo.toml`.

No production crate, Genesis path, gateway path, CI workflow, or repository
script currently invokes `argo-vertical-regression`.

## Contracts Pinned

New test file:
`crates/argo-vertical-regression/tests/cadence_contract.rs`

Pinned contracts:

1. Public inventory from `src/lib.rs` stays explicit.
2. The manual runbook names the exact executable target and preserves the
   static/no-network contract.
3. A fixture with an undeclared `expected.action_type` is reported as a new,
   unbaselined violation.
4. A fixture whose `expected.requires_approval` disagrees with
   `[escalation.human_approval_required]` is reported as an approval mismatch.

The existing `tests/regression_all.rs` remains the full workspace walk over the
real `verticals/` tree.

## Finding

P3 wiring gap: the harness is not automatically run when vertical content
changes. If the intended cadence is "every vertical edit must be checked",
queue follow-up brief `ops-wire-vertical-regression-into-vertical-change-gate`
to run `cargo test -p argo-vertical-regression --test regression_all` from an
existing dev-safe/pre-push gate when staged paths touch `verticals/**`.

If the intended cadence is manual operator use only, no production follow-up is
needed; the runbook and contract tests now make that status explicit.
