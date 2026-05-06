# V1.4 — Activity feed CLI

**Owner**: codex. **Depends on**: claude-realtime-audit-stream
(#831) merged.

## Scope

Operators want a real-time view of what every worker is
doing right now. PR #831 ships `GET /audit/stream` with a
broadcast channel. This brief adds the CLI surface:

- `argo activity tail [--tenant <id>] [--worker <id>]
  [--since <duration>]` — tails the audit stream as
  human-readable lines
- `argo activity since <duration> [--limit <n>]` — list
  recent activity in a window
- `--json` mode pipes NDJSON for downstream tools

Each line:
```
HH:MM:SS [worker-short-id] [action-kind] [decision] message
```

Use the existing `AuditSink::subscribe()` channel;
disconnect handling re-uses the gateway's WS heartbeat
pattern (or polls every 1s if WS isn't ready).

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/activity_tail.rs` (NEW)
- `crates/argo-cli/src/handlers/activity_since.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 2 new
  pub mods
- `crates/argo-cli/src/cli_activity_cmds.rs` (NEW)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly:
  register `Activity` subcommand
- `crates/argo-cli/tests/activity_tail_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo activity tail` follows audit stream live
- [ ] `--tenant` + `--worker` filters apply
- [ ] `--since` accepts "30s", "5m", "1h", "1d" durations
- [ ] `--json` outputs NDJSON envelope
- [ ] Reconnect on broken stream (single retry then exit
      with clear message)
- [ ] 4 unit tests + 1 CLI smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-activity-feed-cli`
- PR title: `cli: argo activity tail/since — real-time worker activity feed (codex-activity-feed-cli)`
