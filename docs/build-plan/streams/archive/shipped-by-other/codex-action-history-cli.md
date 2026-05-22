# V1.4 — Action history CLI

**Owner**: codex. **Depends on**: H-action-replay (#826),
H-audit-query-extensions (#833) merged.

## Scope

`argo action replay <id>` shows a single action's evidence
chain. `argo audit query` filters across actions but is
debug-flavored. What's missing: a chronological **operator
view** of a worker's recent actions for human triage.

Add:
- `argo action history <worker-id> [--since <duration>]
  [--limit N] [--include-skill] [--json]` — chronological
  list of actions newest-first, with timestamp / kind /
  decision / spend / skill (when --include-skill).
- `argo action timeline <worker-id> [--since <duration>]
  [--bucket-mins N]` — ASCII timeline showing actions per
  bucket (e.g. 5-min buckets for past 24h), useful for
  spotting bursts and quiet periods.
- `argo action recent [--tenant <id>] [--worker <id>]
  [--limit N] [--json]` — flat cross-worker list for
  operators looking at the whole org.

Companion to existing `argo activity tail` (live audit
feed); these read historical state, tail watches live.

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/action_history.rs` (NEW)
- `crates/argo-cli/src/handlers/action_timeline.rs` (NEW)
- `crates/argo-cli/src/handlers/action_recent.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub
  mods
- `crates/argo-cli/src/cli_action_cmds.rs` — narrowly:
  add `History`, `Timeline`, `Recent` variants if `Action`
  enum lives there; otherwise create siblings
- `crates/argo-store/src/action_query.rs` (NEW) — pure
  query helpers (history_for_worker / timeline_buckets /
  recent_across)
- `crates/argo-store/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/action_history_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo action history w-1 --limit 10` returns 10 newest
      action rows for w-1, ordered by created_at DESC
- [ ] `--since 1h` filters correctly (wall-clock relative)
- [ ] `--include-skill` joins `skill_used` column when present
- [ ] `argo action timeline w-1 --bucket-mins 5 --since 24h`
      renders 288-bucket ASCII histogram with `█` for
      activity-present, `·` for empty buckets
- [ ] `argo action recent --tenant tenant-a --limit 50`
      returns latest 50 across all workers in tenant-a
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 6 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-action-history-cli`
- PR title: `action: argo action history/timeline/recent — operator triage views (codex-action-history-cli)`
