# Skate-selling proof run

The first real-world commercial proof of the SKYCOT Worker
OS: a multi-day, approval-gated, audit-trailed run selling
real inline-skate inventory through the `gtm-engineer`
vertical.

This guide is the operator-side manual. It assumes Argo is
already installed (see [`getting_started.md`](getting_started.md))
and that you've configured at least one provider key per
[`provider_keys.md`](provider_keys.md).

---

## What you'll have at the end

For each pair of skates you put through the worker:

- a sectioned artifact bundle on disk (research, pricing,
  listings, approval checklist) under `.skycot/outputs/<job_id>/`
- a state-row-by-state-row audit trail in the action ledger
  (every approve / execute / complete / fail recorded as a
  separate append-only memory row)
- per-action operator instruction bundles under
  `.skycot/outputs/<job_id>/actions/<action_id>/` for every
  step the worker proposed
- a workspace-level rolling view (`approval_queue.md`,
  `completed_actions.md`, `blocked_actions.md`) refreshed on
  every transition

---

## The chain wired into gtm-engineer (#772)

The manifest declares conservative `on_complete_emit`
auto-chaining for the pre-sale path:

```
verify_price (autonomous)
   └── complete → emit create_listing_draft (autonomous)
              └── complete → emit request_photos (autonomous)
                          └── complete → emit post_listing
                                              ↑ approval-gated
                                              ↑ operator approves
                                                before execute
```

And for re-pricing:

```
update_price (approval-gated)
   └── complete → emit relist_item (approval-gated)
```

Notably **NOT** auto-chained:

- `post_listing` complete → boost_listing — boosting spends
  operator funds and must be a per-item operator decision
  based on real organic-traction signals, not a default.
- `mark_item_sold` complete → anything — once sold, the OS-
  level **sold-item gate** (#769) blocks ALL further emissions
  for that `item_id` (except a redundant mark_item_sold,
  which is idempotent).
- `reply_to_buyer` / `follow_up_buyer` — buyer-touching
  high-risk actions are operator-initiated only.

---

## 1. Start the run

```bash
# Once per workspace
mkdir -p ~/skate-sales && cd ~/skate-sales
argo workspace init

# Verify the launch checklist before going live
argo setup --check --require ANTHROPIC_API_KEY --require OPENAI_API_KEY
argo worker doctor --vertical gtm-engineer

# Queue the first job. The model will research the item +
# emit a verify_price action proposing a target price.
argo work add \
  --vertical gtm-engineer \
  --prompt "Sell a pair of USD Aeon 60 inline skates, brand new in box, EU 41-42. Item id: skates-A. Target price: AUD \$549." \
  --json
```

Replace the prompt body for each pair of skates. Use distinct
`item_id` values (`skates-A`, `skates-B`, …) so the inventory
view stays clean.

---

## 2. Run the worker as a long-running process

```bash
# Foreground (terminal stays attached)
argo work serve --vertical gtm-engineer --interval-seconds 30

# OR — background as a systemd unit (production)
sudo systemctl start skycot-skate-worker
```

The serve loop:

- polls `pending_jobs` every `--interval-seconds`
- processes one job per tick, writes the artifact bundle
- emits the proposed actions, persists each as a
  `worker_action` memory row
- handles SIGTERM / SIGINT cleanly: finishes the current
  step, then exits with `argo work: graceful shutdown`
  (#770)

Restart-safe: on each tick the dedup set rebuilds from disk,
so a `kubectl rollout restart` or `systemctl restart` picks
up exactly where it left off.

---

## 3. List item status

```bash
# Everything the worker has touched
argo worker items list

# Only what's still in flight
argo worker items list --state active

# Only what's already sold
argo worker items list --state sold

# Machine-readable for cron / dashboards
argo worker items list --json
```

Sample output:

```
item_id                  vertical          sold  pend  apr  exec  done  fail  blk  latest_state  latest_action_type    last_updated_at
skates-A                 gtm-engineer       no     1   0    0     2    0    0   pending       post_listing          2026-05-06T15:30:00Z
skates-B                 gtm-engineer      yes     0   0    0     5    0    1   completed     mark_item_sold        2026-05-05T22:14:11Z
```

---

## 4. Inspect pending actions

```bash
# Everything awaiting your attention right now
argo worker actions list --state pending

# Scope to one item — typical mid-day operator query
argo worker actions list --state pending --item-id skates-A

# Drill into a specific action
argo worker actions show <action_id>

# Full timeline for one item (forensic view)
argo worker items show skates-A
```

The pending queue is where your day's work lives. Each row is
a discrete decision: approve to advance approval-gated rows;
execute to start any pending action.

---

## 5. Approve / execute / complete

Three transitions in sequence. The first only applies to
approval-gated actions (post_listing, boost_listing,
update_price, relist_item, mark_item_sold, reply_to_buyer,
follow_up_buyer).

```bash
# 1. APPROVE — only for approval-gated rows
argo worker actions approve <action_id> --note "comp prices look right"

# 2. EXECUTE — renders the per-action instruction bundle
argo worker actions execute <action_id>
```

`execute` writes a per-action bundle:

```
.skycot/outputs/<job_id>/actions/<action_id>/
  instructions.md   ← what to do, marketplace-aware checklist
  evidence.md       ← cited sources from the worker's research
  action.json       ← full snapshot
```

Open `instructions.md`, follow the checklist, do the action
in your browser. When done:

```bash
# 3. COMPLETE — record the result; auto-emits any follow-ups
argo worker actions complete <action_id> \
  --note "live on eBay" \
  --result-json '{"listing_url": "https://www.ebay.com/itm/...", "platform": "ebay"}'
```

Completing one action triggers any `on_complete_emit` chain
in the manifest. See the chain diagram above.

If something blocks you:

```bash
argo worker actions fail <action_id> --reason "ebay flagged listing — need more photos"
```

`fail` is terminal for that action; if the manifest's retry
policy allows it (#755), you can issue a fresh attempt:

```bash
argo worker actions retry <action_id>
```

---

## 6. Mark an item sold

Marking sold is an approval-gated action like any other —
the worker emits a `mark_item_sold` action when there's
evidence of a sale (e.g., the operator running `complete` on
a buyer-thread action with a sale_price in the result), and
the operator walks it through the standard approve →
execute → complete sequence.

After completion, the OS-level **sold-item gate** kicks in:
every future emission with the same `item_id` lands `blocked`
with a clear reason. Inventory is closed.

To verify a sale landed:

```bash
argo worker items list --state sold | grep skates-A
argo worker items show skates-A          # full timeline
```

---

## 7. Recover after restart

The Worker OS is restart-safe by design:

- All state lives in SQLite + flat files under `.skycot/`.
  No daemon-only in-memory state.
- The action ledger is append-only — a process crash
  mid-transition can never produce a half-written row.
- `argo work serve` rebuilds its dedup set from disk on
  every tick, so a restart never re-processes a completed
  job.

To restart:

```bash
# Stop the worker (SIGTERM-safe via #770)
sudo systemctl restart skycot-skate-worker

# OR foreground: Ctrl+C, then re-launch
argo work serve --vertical gtm-engineer --interval-seconds 30
```

After restart, run a one-shot health check:

```bash
argo worker doctor --vertical gtm-engineer
argo worker items list                       # sanity-check inventory state
argo worker actions list --state executing   # any rows stuck mid-flight?
```

If a row is stuck `executing` (it shouldn't be, but worst-case
power loss can leave one), `argo worker actions fail <id>
--reason "stale executing — manual cleanup"` then `retry`.

---

## 8. Inspect the audit trail

Every state transition is a separate append-only memory row
carrying the full snapshot at that state:

```bash
# Full state-row history for one action
argo worker actions status <action_id> --json

# Full action history for one item
argo worker items show skates-A
```

Workspace-level rolling files (auto-refreshed on every
transition):

```
.skycot/approval_queue.md       # currently pending-approval rows
.skycot/completed_actions.md    # successful sales / actions
.skycot/blocked_actions.md      # research-insufficient / sold-gated / failed
```

For the per-job artifact bundle:

```
.skycot/outputs/<job_id>/
  transcript.md            # full chat transcript
  assistant_output.md      # final assistant text
  research.md              # cited research the worker did
  pricing.md               # comp pricing analysis
  listings.md              # draft listing copy
  approval_checklist.md    # what the operator should sanity-check
  execution_plan.md        # actions grouped by state
  actions.json             # machine-readable ledger snapshot
  job.json                 # job metadata
  result.json              # pointers + counts
  actions/<action_id>/     # per-action operator bundle (post-execute)
```

For provider spend (per-tenant) and the action-ledger
counters (Prometheus + JSON):

```bash
argo costs                    # spend rollup
argo costs by-day --days 7
curl http://127.0.0.1:7400/metrics       # full /metrics dump
curl http://127.0.0.1:7400/metrics/json  # JSON variant for jq
```

The relevant action-ledger metrics (#757):

- `argo_worker_actions_total{state="<state>"}` — per-state
  counts, deduped to latest snapshot per action_id
- `argo_worker_actions_pending_approval_total` — your review-
  queue depth
- `argo_worker_actions_pending_autonomous_total` — pending
  rows the worker can advance without you

---

## When to escalate to `failure_modes.md`

Symptom-driven failures (provider 503, DB lock, retry
exhausted, stuck jobs) are catalogued in
[`failure_modes.md`](failure_modes.md) with symptom / cause /
recovery / prevention shape. Read it once before the first
real run; refer back to it the first time anything breaks.

---

## Pre-launch checklist (per `provider_keys.md`)

Before queueing the first real job:

- [ ] `argo setup --check --require ANTHROPIC_API_KEY
      --require OPENAI_API_KEY` passes
- [ ] `argo worker doctor --vertical gtm-engineer` reports
      healthy + zero unexpected blocked rows
- [ ] `./scripts/smoke-skycot.sh` passes (18-check
      end-to-end smoke; mock-mode, no real spend)
- [ ] Read this guide end-to-end
- [ ] Read `failure_modes.md`
- [ ] At least one non-mock smoke job ran end-to-end and the
      `stop_reason` was NOT `need_info`
- [ ] `/metrics` is being scraped + a spend alert is wired

When the list is green, queue the first real job and start
running.
