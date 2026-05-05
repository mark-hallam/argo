# Operator runbook — `argo worker actions`

Reference for the SKYCOT operator surface. Every command in this
file maps to a state-machine transition, a ledger query, or an
artifact render.

## State machine

```
  pending ──→ approved ──→ executing ──→ completed
    │           │              │           ▲
    │           └──────────────┴───────────┘   (synchronous shortcut)
    │                                ↓
    │                              failed
    ↓
  blocked  (terminal)
```

Each transition appends a NEW `worker_action` memory row carrying
the full snapshot at the new state. The "current state" of an
action is the latest row matching its `action_id`. Append-only —
prior rows are never mutated.

| From       | Via         | To           | Notes                                              |
|------------|-------------|--------------|----------------------------------------------------|
| pending    | approve     | approved     | Only for rows where `requires_approval = true`.    |
| pending    | execute     | executing    | Only for autonomous-safe rows (refused otherwise). |
| pending    | (OS-only)   | blocked      | Set at emission time; terminal.                    |
| approved   | execute     | executing    |                                                    |
| approved   | complete    | completed    | Synchronous shortcut for offline-already-done.     |
| executing  | complete    | completed    |                                                    |
| executing  | fail        | failed       | Terminal.                                          |
| completed  | —           | —            | Terminal.                                          |
| failed     | —           | —            | Terminal.                                          |
| blocked    | —           | —            | Terminal.                                          |

## CLI reference

### `argo worker actions list`

Lists actions across the loaded verticals.

```bash
argo worker actions list \
  [--state pending|approved|executing|completed|failed|blocked|all] \
  [--job-id <uuid>] \
  [--vertical <id>] \
  [--action-type <name>] \
  [--platform <name>] \
  [--limit N] \
  [--json]
```

Defaults: `--state all`, `--limit 50`, newest-first by `created_at`.

### `argo worker actions show <action_id>`

Full latest snapshot of one action — useful before approving.

```bash
argo worker actions show <action_id> [--json]
```

Shows: action_id, job_id, item_id, action_type, state, platform,
created_at, approved_by, approved_at, executed_at, blocked_reason,
evidence list, payload (JSON), result (JSON, if any).

### `argo worker actions approve <action_id>`

Transitions `pending → approved`. Refuses autonomous-safe actions
(use `execute` directly) and unknown ids.

```bash
argo worker actions approve <action_id> \
  [--approved-by <name>] \
  [--note <text>] \
  [--json]
```

`--approved-by` defaults to `$USER`. The note is recorded into
`result.approval_note` for audit context.

### `argo worker actions approve-batch <id1> <id2> ...`

Bulk-approve. Per-id failures don't stop the rest; the command
exits nonzero if any id failed.

```bash
argo worker actions approve-batch <id1> <id2> ... \
  [--approved-by <name>] \
  [--note <text>] \
  [--json]
```

### `argo worker actions execute <action_id>`

Transitions `pending|approved → executing` AND invokes the
executor declared by the manifest. v1 supports `executor =
"human"` only — the human executor renders a per-action artifact
bundle to `.skycot/outputs/<job_id>/actions/<action_id>/`.

```bash
argo worker actions execute <action_id> [--json]
```

Refuses `pending → executing` for `requires_approval = true` rows
(approve first). The state-machine matrix permits the transition
but the OS-level check in `execute()` enforces the safety boundary
in code, not via prompts.

### `argo worker actions complete <action_id>`

Transitions `approved|executing → completed`. Records
`executed_at` (if not already set) and stores the operator's
result.

```bash
argo worker actions complete <action_id> \
  [--note <text>] \
  [--result-json '<json>'] \
  [--json]
```

`--note` lands in `result.summary`. `--result-json` is a structured
blob the operator can use to record arbitrary outcomes (listing
URL, sale price, buyer username — anything the action produced).
The two are merged: when both are set, `summary` is added to the
JSON object.

### `argo worker actions fail <action_id> --reason <text>`

Transitions `executing → failed`. Records the reason into
`result.failure` for retry / triage.

```bash
argo worker actions fail <action_id> --reason "<why>" [--json]
```

`failed` is terminal. Retries create a NEW action that can
reference the failed one in its `evidence` list.

### `argo worker actions status <action_id>`

Full state-row history in time order — every persisted transition
for an action_id, oldest-first. The most useful command when
auditing "who approved this and when".

```bash
argo worker actions status <action_id> [--json]
```

### `argo worker actions render`

Re-renders the workspace-level per-state ledger files at
`.skycot/`:

```bash
argo worker actions render [--vertical <id>]
```

Writes:

- `.skycot/approval_queue.md` — currently-pending-approval rows.
- `.skycot/completed_actions.md` — terminal `completed` audit
  trail with operator summaries.
- `.skycot/blocked_actions.md` — terminal `blocked` rows with
  rejection reasons inline (the investigation queue).

Re-run any time the ledger changes; the files are deterministic
projections of the current latest-row-per-action_id state.

## Per-job artifact bundle

Every worker job writes under `.skycot/outputs/<job_id>/`:

| file                     | what                                                                    |
|--------------------------|-------------------------------------------------------------------------|
| `transcript.md`          | full user/assistant chat                                                |
| `assistant_output.md`    | final assistant text                                                    |
| `research.md`            | merged-across-steps research section                                    |
| `pricing.md`             | merged-across-steps pricing section                                     |
| `listings.md`            | merged-across-steps listings section                                    |
| `approval_checklist.md`  | the operator's go/no-go list. Carries a BLOCKED banner when evidence-thin. |
| `execution_plan.md`      | per-job action plan grouped by state.                                   |
| `actions.json`           | machine-readable mirror of execution_plan.md.                           |
| `job.json`               | metadata (provider/model/timestamps/stop_reason).                       |
| `result.json`            | pointers to every other file + counts.                                  |
| `actions/<id>/instructions.md` | per-action operator brief (rendered by the executor).             |
| `actions/<id>/evidence.md`     | per-action cited sources.                                          |
| `actions/<id>/action.json`     | per-action full snapshot.                                          |

## Common operator flows

### "What's awaiting me?"

```bash
argo worker actions render
cat .skycot/approval_queue.md
```

### "Approve everything pending for the eBay platform"

```bash
ids=$(argo worker actions list --state pending --platform ebay --json | jq -r '.action_id')
argo worker actions approve-batch $ids --note "weekly batch approval"
```

### "Walk one action all the way through"

```bash
argo worker actions show <id>
argo worker actions approve <id> --note "comp prices verified"
argo worker actions execute <id> --json
# ... operator carries action out offline ...
argo worker actions complete <id> --note "live" \
  --result-json '{"listing_url": "..."}'
argo worker actions status <id>
```

### "Why did this land blocked?"

```bash
argo worker actions show <id> --json | jq .blocked_reason
```

Common reasons:

- `"no capability grant for action_type 'X' in vertical 'Y'"` —
  the worker emitted an action_type the vertical's manifest doesn't
  declare. Add the declaration if you want it; otherwise the action
  was probably hallucinated.
- `"research evidence insufficient — see result.json's evidence
  summary"` — the worker emitted an approval-gated action without
  enough web_search hits + web_extract chars to back it up. The
  fix is upstream: re-run with better prompts, more sources, or a
  vertical that actually has research tools.

### "I want to retry a failed action"

`failed` is terminal. Don't try to mutate the row — that's not
allowed by the state machine. Instead, re-emit a fresh action from
the worker (or queue a new job that will re-emit) and reference
the failed one in the new action's `evidence`:

```toml
evidence = ["...", "argo://action/<failed-action-id>"]
```

The worker's prompt should explain to the model what failed so it
adapts (different platform, different message, different price).

## Troubleshooting

| symptom                                        | likely cause                                                              | fix                                                  |
|------------------------------------------------|---------------------------------------------------------------------------|------------------------------------------------------|
| `approve` says "autonomous-safe"               | The action's `requires_approval = false` — no approval needed.            | Use `execute` directly.                              |
| `execute` says "approval required"             | Approval-gated and not yet approved.                                       | Run `approve` first.                                 |
| `fail` says "illegal transition"               | Action isn't in `executing` state.                                         | Check `status`; only executing rows can fail.        |
| `render` reports zeros for all categories      | Latest snapshot per action is in some non-displayed state (e.g. approved). | This is correct — `approved` isn't terminal yet.     |
| `status` says "no action with action_id"       | Wrong id, or the id is from a different workspace.                        | Confirm via `list --json`; pwd should match the workspace. |
