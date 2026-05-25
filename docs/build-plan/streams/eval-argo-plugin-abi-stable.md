# eval-argo-plugin-abi-stable — audit argo-plugin JSON-RPC ABI surface + stability contract

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-plugin-abi-stable`.

## Why this is P1

Mission gap manifest row `argo-plugin` is 🔴 NOT AUDITED, P1. Polyglot core invariant #1 says plugins are subprocesses spoken to via JSON-RPC (JSON Remote Procedure Call — a way for one program to call functions in another using JSON over a pipe or socket). The ABI (Application Binary Interface — the contract between the host and the plugin process) must be stable so out-of-tree plugins survive runtime upgrades.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-plugin/src/lib.rs`.
2. **JSON-RPC method catalogue** — every request method the host sends; every notification; document the parameter schemas.
3. **Stability contract** — is there a versioned ABI envelope? How does the host negotiate version with the plugin?
4. **Breaking-change detection** — if a `pub struct` in the ABI gains a non-optional field, what fails? Is there a CI lint that flags this?
5. **Pin tests** at `crates/argo-plugin/tests/abi_stability_pin_test.rs` (~200-300 LOC) — surface contract tests pinning the JSON-RPC envelope.
6. **Report** at `docs/skycot/evals/2026-05-25-argo-plugin-abi-stable.md`.

## Files you may touch

- `crates/argo-plugin/tests/abi_stability_pin_test.rs` — new.
- `docs/skycot/evals/2026-05-25-argo-plugin-abi-stable.md` — new.

## Files you must NOT touch

- `crates/argo-plugin/src/**`.

## Acceptance criteria

- [ ] Surface table + method catalogue.
- [ ] Versioning contract documented with citations.
- [ ] Pin tests PASS.
- [ ] ≤5 follow-up briefs (likely including a CI lint for ABI changes).

## Branch + PR

- Branch: `claude-eval-argo-plugin-abi-stable` or `codex-…`.
- PR title: `eval(plugin): audit JSON-RPC ABI stability surface (eval-argo-plugin-abi-stable)`.
