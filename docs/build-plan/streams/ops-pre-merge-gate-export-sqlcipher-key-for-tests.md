# ops-pre-merge-gate-export-sqlcipher-key-for-tests

**Authored against**: origin/main `d80fa2998` on `2026-05-24`.
**Validate before claim**: bash scripts/check-brief-freshness.sh ops-pre-merge-gate-export-sqlcipher-key-for-tests

## Why this brief exists

Surfaced as round-42 P1 finding (evaluator handoff 2026-05-23T19:43:50Z) — sibling of the still-TODO `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (round-41 P0).

`scripts/conductor-pre-merge-gate.sh` is the pre-merge gate the conductor runs locally before admin-merging worker PRs (since GitHub Actions remain operator-disabled). When a worker PR includes integration tests that boot the gateway (e.g. `argo-cli` smoke tests, `argo-gateway` tenant-bind tests), those tests need a SQLCipher key to derive the per-tenant master keys; without `SKYCOT_DB_KEY_BASE64` set, gateway-bind hangs ~10s then deadline-exceeds at `gateway store: derive SQLCipher key for workspace-default tenant: keyring read tenant-… User canceled the operation`.

This is the SECOND structural gate-masking pattern surfaced in two consecutive evaluator rounds:

1. Round 41 (P0): `--skip-clippy` silently allowed RED `cargo test -p argo-skill --lib store_worker_overlay_tests::` to land. Brief `ops-pre-merge-gate-fix-test-failures-not-just-clippy` already TODO.
2. **Round 42 (P1, this brief)**: gateway-bind-hang on the test machine masks any test that exercises gateway boot. PR #2164's `demo_route_mints_principal_and_writes_audit_when_flag_set` was the canary — its column-name typo at line 79 (`payload` instead of `payload_json`) was never caught because the test couldn't even reach the audit-row query.

Round 42 directly reproduced: 10s deadline-exceeded without the key, vs <2s bind once `SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32)` was exported (per PR #2165's runbook §5.1).

Pre-merge gate ergonomics is at the top of the gate-hardening backlog. Both patterns are "test failed in a way the gate didn't surface" — round 41 by exit-code, round 42 by hang/deadline. Surfacing them is cheap; not surfacing them keeps landing broken tests.

## Acceptance criteria

1. Edit `scripts/conductor-pre-merge-gate.sh` (only — do not change other gate scripts in scope).
2. Before any `cargo test` step OR before the gate's existing test invocation surface, export:
   ```bash
   export SKYCOT_DB_KEY_BASE64="${SKYCOT_DB_KEY_BASE64:-$(openssl rand -base64 32)}"
   ```
   The `${VAR:-...}` form preserves any operator-supplied key (e.g. when reproducing a specific master-key scenario) while supplying a fresh per-invocation key when unset. The key is ephemeral — gate runs are stateless.
3. Add a 1-line gate stage banner like `[pre-merge-gate] [N/M] ensure SKYCOT_DB_KEY_BASE64` so the export is visible in tick logs.
4. If `openssl` is missing on the gate machine, fail loud with `exit 2` and a one-line install hint (`brew install openssl`); do NOT silently fall back to an empty key (that would re-introduce the gateway-bind hang the brief is fixing).
5. Add a single smoke test in the gate's own self-test path (if one exists) or in PR-body verification text: `bash scripts/conductor-pre-merge-gate.sh --skip-clippy` PASSES with `SKYCOT_DB_KEY_BASE64` unset on entry (proves the export wraps every downstream step).
6. PR body explicitly notes which `cargo test` surfaces in the gate now have a derived key, and confirms the key is NOT persisted (no `~/.bash_profile` / `~/.zshrc` edits).

## Parallel-surface declarations

- Bash-only change. No new pub Rust surface; no schema change; no doc-only sibling required (existing PR #2165 runbook §5.1 documents the env-var pattern).
- Sibling brief `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (still TODO) is the test-failure-parsing companion — these two together close the structural masking story. Do NOT bundle implementation; sibling has its own brief.

## May-touch

- `scripts/conductor-pre-merge-gate.sh` (only)
- Optional: a self-test under `scripts/tests/` if that pattern already exists (worker confirms at claim time)

## Boundary

- Do NOT add other env-var exports (e.g. `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) — those are out of scope; round 30's evaluator finding noted production gateway also requires those but the pre-merge gate doesn't currently run gateway-bind integration tests that need them.
- Do NOT persist the key across runs — per-invocation random is the contract.
- Do NOT touch `scripts/check-migration-slots.sh`, `scripts/parallel-pub-types-enforce.sh`, `scripts/loc-budget.sh`, `scripts/zero-caller-substrate-lint.sh`, or `scripts/workspace-clippy-fast-fail.sh` — they are not in scope.
- Do NOT touch worker pre-push gates (`scripts/worker-pre-push-gate.sh` if it exists) — workers running in their own environments handle their own keys; this brief is conductor-side only.

## Sizing

~5-20 LOC bash. Fast-mode candidate.

**🔴 P1 ANTI-THEATER** — closes round-42 evaluator finding. Without this fix any future gateway-bind smoke test added to a PR will silently hang on the conductor's pre-merge gate machine, allowing typo-class test bugs (cf. PR #2164's `payload` vs `payload_json` typo) to land unblocked. Combined with sibling brief `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (round-41), closes the structural gate-masking story.
