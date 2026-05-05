# scripts/

Operator-runnable shell scripts that aren't part of the Rust build.

## `smoke-skycot.sh`

End-to-end SKYCOT install verification. Runs in mock-mode (no API
key required) against a fresh tempdir workspace, drives every
state transition the operator surface exposes, and asserts the
artifact bundle landed correctly.

**Usage:**

```bash
./scripts/smoke-skycot.sh             # use `argo` from PATH
./scripts/smoke-skycot.sh ~/.local/bin/argo
```

**What it asserts (12 checks):**

1. `argo` is on PATH (or at the given path).
2. `argo workspace init` succeeds.
3. The `gtm-engineer` vertical loads.
4. A worker job queues + runs (mock provider).
5. The 10 v1 artifact bundle files materialise.
6. The action ledger has the 2 expected rows (verify_price +
   post_listing) from the mock emission.
7. `post_listing` correctly lands `blocked` under mock evidence
   (the safety primitive — no approval-readiness on weak research).
8. `approve` correctly refuses the autonomous-safe `verify_price`.
9. `execute` writes a per-action bundle with the rendered
   `verify_price` instruction template.
10. `complete` advances state to `completed`.
11. `status` emits the full pending → executing → completed
    transition history.
12. `render` writes the 3 workspace ledger files with expected
    entries.

**Exit codes:**

- `0` — all 12 checks passed.
- `1` — a check failed (output names which one + offending
  command output).
- `2` — prerequisites missing (no `argo` binary, no verticals
  dir).

**When to run:**

- Right after `argo setup` to confirm SKYCOT is wired end-to-end.
- After upgrading the binary to confirm nothing regressed.
- In CI for any vertical / handler / executor change.

The script uses tempdirs for both `ARGO_HOME` and the workspace
cwd, so it leaves nothing behind on success.
