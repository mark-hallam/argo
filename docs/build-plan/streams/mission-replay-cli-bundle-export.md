# mission-replay-cli-bundle-export — `argo replay export <session_id>` (post-v1 polish P1)

**Owner**: claude. **Depends on**: none (single-crate; reuses existing replay + audit-export logic).
**Authored against**: origin/main `e2fe4b70` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-replay-cli-bundle-export`.
**Source**: post-v1 polish — incident-triage workflow. Operator needs to export a session for offline analysis or to share with support.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — full-session reproducibility for incident response.
- **Repeatability** (#3) — exported bundle replays deterministically on a fresh install.

## Scope

P1. New CLI subcommand that emits a self-contained tarball:
```bash
argo replay export <session_id> --out <path>     # tar.gz
argo replay export <session_id> --out <path> --uncompressed   # plain tar
argo replay export <session_id> --sanitize       # redact secrets/PII
```

Bundle contents (tarball structure):
```
session-<id>.tar.gz
├── manifest.toml          # session metadata + tool version + content checksum
├── audit/
│   ├── chain.jsonl        # full audit rows (canonical JSON, one per line)
│   └── verify.txt         # `argo audit verify-chain --session` output
├── skills/
│   └── *.json             # any skill IRs invoked by this session
├── memory/
│   └── *.json             # memory rows touched (read OR written) by this session
├── bundle/
│   └── action-*.json      # per-action input/output bundles
└── README.md              # how to replay: instructions
```

Behavior:
- **Sanitize mode** (`--sanitize`) — redact: provider keys (`sk-*`, `eyJ*` JWTs), email addresses, phone numbers in payloads. Replaces values with deterministic placeholders (`sk-redacted-<hmac>`).
- **Without sanitize** — full fidelity, including secrets (caller's responsibility to handle the file).
- **Manifest checksum** — SHA-256 of all included file contents, recorded in manifest.toml. Mismatch on extract = tampered.

Replay-on-extract: a follow-up brief wires `argo replay import <bundle>` to reverse-load; not in scope here.

LOC: ~250.

## Files you may touch

- `crates/argo-cli/src/cli_replay_cmds.rs` (existing or NEW) — `Export { session_id, out, uncompressed, sanitize }` variant. ≤30 LOC.
- `crates/argo-cli/src/handlers/replay_export.rs` (NEW) — handler. ≤180 LOC.
- `crates/argo-cli/src/handlers/replay_export_tests.rs` (NEW). ≤120 LOC.
- `crates/argo-replay/src/bundle.rs` (NEW) — pure logic: assemble bundle from DB. ≤180 LOC.
- `crates/argo-replay/src/bundle_tests.rs` (NEW). ≤80 LOC.
- `crates/argo-replay/src/lib.rs` — re-export. ≤4 modified LOC.
- `crates/argo-replay/Cargo.toml` — narrowly: ensure `tar`, `flate2`, `sha2` deps (probably already present in workspace). ≤4 modified LOC.
- `docs/skycot/cli-reference.md` — add `replay export` row. ≤8 modified LOC.
- `docs/skycot/operator/incident-response.md` (existing or NEW) — add "Export session for support" section. ≤40 modified LOC.

## Files you must NOT touch

- The audit chain hash logic — read-only.
- `argo-store` schemas — no migrations.
- The existing `argo session replay` / `argo session diff` subcommands — orthogonal.
- The memory store internals — read-only access via existing query helpers.

## Acceptance criteria

- [ ] `argo replay export <session_id> --out /tmp/s.tar.gz` writes a tarball.
- [ ] `tar tzf /tmp/s.tar.gz` lists: `manifest.toml`, `audit/chain.jsonl`, `skills/`, `memory/`, `bundle/`, `README.md`.
- [ ] `manifest.toml` contains: `session_id`, `tenant_id`, `start_ts`, `end_ts`, `argo_version`, `audit_row_count`, `sha256` (covering all included files).
- [ ] `audit/chain.jsonl` re-verifies clean: extracting + running `argo audit verify-chain --from-file` (where that exists) returns clean.
- [ ] `--sanitize` redacts strings matching the patterns; replay manifest indicates `sanitized: true`.
- [ ] Unknown session_id → exit 1 with "session not found".
- [ ] Bundle is round-trippable: `tar xzf bundle.tar.gz && diff <(jq -S . audit/chain.jsonl) <(argo audit export --session <id>)` matches.
- [ ] Memory rows export uses canonical sort order (by `(tenant, namespace, key)`).
- [ ] `cargo test -p argo-replay -p argo-cli` passes.
- [ ] `cargo clippy -p argo-replay -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-replay-cli-bundle-export`
- Title: `feat(cli): argo replay export — session-bundle tarball (mission-replay-cli-bundle-export)`
- Worker verification: crate-scoped clippy + tests + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Streaming, not buffered.** Multi-thousand-row sessions need to stream rows to the tar writer, not collect-then-write. `tar::Builder` with a streaming reader.
- **Memory rows are per-session via audit walk.** No `session_id` column on memory rows directly; collect by walking the audit chain for `Memory*` kinds.
- **Sanitization is regex-based** but applied to JSON payload strings only — don't accidentally redact field names or audit kinds.
- **Manifest checksum order is deterministic.** Sort file names before hashing.
- **No symlinks in tar.** `tar::Builder` defaults to following symlinks; explicitly disable for security.
- **`--out` parent dir must exist.** Don't `mkdir -p` silently; error with clear message.
- **README.md content** — short, 20 lines max, points to operator-docs incident-response page for full replay instructions.
