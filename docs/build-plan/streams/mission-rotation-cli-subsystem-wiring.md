# Mission load-bearing wire — Wire key-rotation coordinator to subsystem CLIs

**Owner**: claude. **Depends on**: `mission-automated-key-rotation` MERGED (#1462); `mission-encryption-at-rest-sqlcipher` MERGED (#1461); `mission-encryption-at-rest-credentials-toml` MERGED (#1460).
**Authored against**: origin/main `77ee9a59` on `2026-05-13`.

> SOURCE: PR-deferral from #1462. The `argo security rotate-keys` coordinator records rotation intent + prints the subsystem command for the operator to run manually. Now that the subsystem CLIs exist (`argo db rekey`, `argo credentials encrypt-all`), wire the coordinator to invoke them directly.

## Scope

Switch `argo security rotate-keys commit <key>` from print-and-suggest to direct subprocess invocation:

1. **For each key category in the registry**, replace the static `subsystem_command` string with a typed dispatch:
   - `db-encryption` → spawn `argo db rekey --new-key-source <src>` as subprocess
   - `credentials-master` → spawn `argo credentials encrypt-all --new-master-key <src>` as subprocess
   - `gateway-bearer` → still print-and-suggest (env-var rotation requires restart; can't be self-rotated)
   - `channel-hmac` → still print-and-suggest (same reason)
   - `connector-oauth` → spawn `argo connector reauth <name>` per connector
2. **Capture subprocess stdout/stderr** into the audit-row payload; propagate non-zero exit as `rotation.failed` audit event.
3. **Atomic semantics**: if a subsystem call partially succeeds and then fails, the rotation_id stays recorded so re-running with `--retry` resumes. Don't try to roll back — the operator handles partial state.
4. **--dry-run flag** that prints what would be invoked without actually running, for operator confidence.

NOT in scope: parallel rotation (sequential one-at-a-time in v1); rolling back partial rotations; cross-host coordination.

## Files you may touch

- `crates/argo-cli/src/handlers/security_rotate_keys.rs` (≤ 490 LOC after) — replace string return with dispatch enum + Command::new spawn
- `crates/argo-cli/src/handlers/security_rotate_keys_registry.rs` (≤ 280 LOC after) — extend registry entry shape with execution kind enum (`Subprocess(cmd, args)` vs `OperatorPrompt(msg)`)
- `crates/argo-cli/src/handlers/security_rotate_keys_tests.rs` (≤ 480 LOC after — coordinate with brief's existing 7+3 tests) — add 5 new tests: subprocess-success / subprocess-fail-captured / dry-run-prints-without-spawn / retry-resumes / operator-prompt-fallback
- `docs/skycot/RUNBOOK.md` — update "Key rotation" section noting the auto-dispatch + the still-manual env-var categories

## Files you must NOT touch

- `argo db rekey` and `argo credentials encrypt-all` themselves — this brief is a caller, not a rewrite
- The audit-row schema; the rotation_id idempotency check

## Acceptance criteria

- [ ] `db-encryption` and `credentials-master` rotations spawn subprocess + capture output
- [ ] `gateway-bearer` and `channel-hmac` still print-and-suggest with a clear "requires restart" message
- [ ] Connector OAuth: spawn `argo connector reauth <name>` per known connector
- [ ] Audit row captures stdout/stderr + exit code
- [ ] `--dry-run` flag prints command without executing
- [ ] `--retry` resumes from partial state (rotation_id deduplication)
- [ ] 5 new unit tests
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] All touched files < 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-rotation-cli-subsystem-wiring`
- PR title: `security: rotate-keys spawns subsystem CLIs directly (mission-rotation-cli-subsystem-wiring)`

## Notes

- Subprocess spawning from a security-sensitive CLI is dangerous. Use `std::process::Command` with absolute path resolution (find `argo` binary by `std::env::current_exe()` parent dir) to avoid PATH injection.
- Capture stdout/stderr to in-memory buffers (no spawning a shell). Output goes into audit-row payload after redaction (the subsystem CLIs already redact their own secrets in their output).
- `--retry` semantics: same `rotation_id` lookup as today's coordinator. If a row with the same id exists, check its outcome; if `failed`, allow retry; if `succeeded`, no-op.
