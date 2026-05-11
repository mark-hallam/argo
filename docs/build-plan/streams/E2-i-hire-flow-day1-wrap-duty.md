# Stream E2-i — Hire flow: Day-1 wrap duty (backend cron, last E2 brief)

**Owner**: claude.
**Depends on**: E2-e MERGED (hire-commit endpoint exists); existing duty scheduler (`claude-worker-duty-scheduler` MERGED) and notify channels (`mission-notify-channels-rest-of-pack` MERGED).
**Authored against**: origin/main `1657d6f6` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E2-i-hire-flow-day1-wrap-duty`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E2 brief 9 of 9 (LAST). Backend brief — Rust, not TypeScript.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Day-1 trigger | Cron-style duty registered on hire commit; fires `now + 24h` (workspace tz) | Per-tenant configurable cadence (24h / 48h / weekly) per the operator's preference | `mission-enterprise-hire-wrap-cadence` |
| Channel pick | Uses worker's primary channel from Step 5 | Per-message channel routing rule may override (e.g. summaries always email, even if primary is Slack) | `mission-enterprise-channel-routing-policy` (already named in E2-g) |
| Output template | Hardcoded template inside the duty action | Operator-overridable workspace template for Day-1 wrap | `mission-enterprise-hire-wrap-template` |
| Quiet hours | Respects existing per-channel quiet hours (already shipped via `mission-quiet-hours-channels` MERGED) | Same | n/a |
| Skipped on inert workers | Yes — inert workers (no connectors) skip the wrap; they get a different `inert-day1-prompt` reminder instead | Same in target | n/a |

v1 is acceptable because the Day-1 wrap is informational
(no decisions, no money-moving) and degrades gracefully —
worst case the user doesn't get a summary, and they go
about their day.

## Why

After hire (E2-e), the user is told (in the celebration
banner from E2-h) "Mariah will start working in the
background. The first time she has an opinion, you'll see
it here." But what if Mariah has nothing to do for the
first 24 hours? Or what if she did 8 things autonomously
and the user wants to know how it went?

Per [journey-hire-a-teammate.md §"Step 7 — Day-1 wrap"](../../skycot/ux-research/journey-hire-a-teammate.md),
the worker sends a friendly summary 24 hours after hire:
"Here's what I did, here's what's planned tomorrow."
This is the bridge from "I just hired her" to "she's part
of how my business runs now."

This brief implements the Day-1 wrap as a worker duty —
a cron'd action that fires automatically, queries the
worker's recent activity, composes a summary, and sends
via the worker's primary channel (respecting quiet hours).

## Scope

NEW skill + duty wiring:

- A new built-in skill `worker-day1-wrap` registered with
  the workspace at install. Skill kind: `notify`; output:
  a multi-line summary message.
- A hire-commit hook in the existing
  `crates/argo-cli/src/handlers/worker_hire.rs` (or the
  equivalent file for the worker-create code path that
  E2-e's POST /api/v1/workers calls): on successful hire,
  register a one-shot duty `hire-day1-wrap` cron'd at
  `now + 24h` (workspace timezone).
- The duty action: invoke `worker-day1-wrap` skill with
  the worker's id.
- The skill implementation: query worker activity over
  the last 24h (replies sent, drafts queued, approvals
  asked, KPI deltas if surfaced); compose a summary
  message; route to the worker's primary channel.
- Inert-worker variant: if the worker has no successful
  actions in 24h AND has 0 connectors, send a different
  `inert-day1-prompt` message instead (gentle reminder
  to connect tools).
- The one-shot duty cleans up after firing (no re-fire
  on day 2).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `crates/argo-cli/src/handlers/worker_hire.rs`
  (or equivalent file owning hire commit); the worker-
  registration code path.

## Files you may touch

- `crates/argo-skills/src/day1_wrap.rs` — NEW. The
  `worker-day1-wrap` skill implementation: query +
  compose + send. ≤ 350 LOC.
- `crates/argo-skills/src/day1_wrap_tests.rs` — NEW.
  ≥ 8 unit tests. ≤ 300 LOC.
- `crates/argo-skills/src/lib.rs` — narrowly scoped:
  register the new skill module. 1-line addition.
- `crates/argo-cli/src/handlers/worker_hire.rs` —
  narrowly scoped: after worker insert, register a
  one-shot duty `hire-day1-wrap` scheduled at
  `now + 24h`. ≤ 80 LOC additions.
- `manifests/skills/worker-day1-wrap.toml` — NEW.
  Skill manifest (id, version, kind: `notify`,
  trust-grade seed). ≤ 50 LOC.
- `crates/argo-runtime/src/duty_scheduler.rs` —
  narrowly scoped: if the existing scheduler doesn't
  support one-shot duties (i.e. fires once and clears),
  add that capability. Otherwise no changes. ≤ 100
  LOC additions.

## Files you must NOT touch

- All E2-a..h files — these are frontend; this brief is
  Rust-only.
- `crates/argo-channels/**` — existing channels are
  re-used as-is. No channel-side changes.
- Existing skills in `crates/argo-skills/**` — only
  the new `day1_wrap.rs` + 1 line in `lib.rs`.
- `crates/argo-vertical/**` — no schema changes; the
  new skill is workspace-level, not vertical-bound.

## Acceptance criteria

- [ ] NEW skill `worker-day1-wrap` is registered with
      the workspace at install / migration.
- [ ] On successful hire (POST /api/v1/workers from
      E2-e), a one-shot duty `hire-day1-wrap` is
      registered for the new worker, cron'd at
      `now + 24h` (workspace tz).
- [ ] The duty action invokes the
      `worker-day1-wrap` skill with the worker's id.
- [ ] Skill behavior — active worker:
      - Query worker activity for the last 24h
        (replies, drafts, approvals).
      - Compose a summary message using the locked
        template (see Notes).
      - Route to the worker's primary channel.
      - If primary channel is in quiet hours: defer
        to the next allowed window (uses existing
        `quiet_hours.rs` infrastructure).
      - Audit-log the send.
- [ ] Skill behavior — inert worker:
      - Detect: 0 actions in 24h AND 0 connectors.
      - Send the inert variant message via the user's
        primary channel.
      - Same quiet-hours + audit treatment.
- [ ] One-shot duty: after firing, the duty row is
      marked `completed` and does NOT re-fire on day
      2.
- [ ] 8 unit tests pass:
      - Active-worker summary composition with N=0
        actions, N=1, N=5+.
      - Inert-worker variant detection.
      - Quiet-hours deferral.
      - Cron parsing for `now + 24h` in workspace tz.
      - Audit emission verification.
      - One-shot-cleanup verification.
      - Channel routing respects primary.
      - Failure recovery (channel send fails → retry
        via existing retry-with-backoff).
- [ ] `cargo build -p argo-skills` clean.
- [ ] `cargo clippy -p argo-skills --all-targets -- -D
      warnings` clean.
- [ ] `cargo test -p argo-skills` passes.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E2-i-hire-flow-day1-wrap-duty`
- PR title: `skills(day1-wrap): cron'd Day-1 hire-wrap summary duty (E2-i)`

## Notes for the implementer

- **Locked template — active worker summary:**
  ```
  Hey {operator_name},

  {worker_name}'s first day:
  • {n_actions} actions handled ({n_approved}
    approved by you, {n_autonomous} on autonomous policy,
    {n_refused} you said no to)
  • {kpi_summary_line}    // e.g. "$340 in pending offers"
                          // or "3 interviews booked"
  • {n_questions} questions for you:
    {bullet_list_of_pending_approvals}

  Tomorrow:
  {next_duty_summary}     // e.g. "morning routine at 06:30"

  — {worker_name}
  ```
  Substitutions live in `compose_day1_message()` in the
  new skill file. Keep the template easy to read in
  plain-text channels (SMS, email) and rich-text
  channels (Slack) alike — no markdown beyond bullets.

- **Locked template — inert-worker variant:**
  ```
  Hey {operator_name},

  {worker_name} is hired but I haven't been able to
  start work yet — I'm waiting for connector access.

  When you have a minute: open her profile and
  connect the tools she needs. I'll get going as soon
  as you do.

  — {worker_name}
  ```

- **Cron expression.** Workspace timezone matters here.
  Use the workspace's `timezone` field (already on the
  workspace row). Compute `now_in_tz + 24h` and emit
  the cron as a one-shot expression (the scheduler
  already supports one-shot per
  `claude-worker-duty-scheduler` — verify by reading
  that crate; if missing, add it inline in this PR).

- **Activity query.** Use the existing audit-row
  search (`audit_record` table). Query:
  ```sql
  SELECT * FROM audit_record
  WHERE worker_id = $1
    AND created_at > $2 - INTERVAL '24 hours'
    AND action_type IN ('message_sent', 'message_drafted', ...);
  ```
  Aggregate counts by outcome. Don't include sensitive
  details in the summary message — names + counts only,
  not the customer's actual message content.

- **Channel send via the existing notify infrastructure.**
  Re-use `argo-channels` send paths. Quiet-hours
  deferral is already wired in
  `mission-quiet-hours-channels` MERGED — invoke via
  the standard `NotifyChannel::send_or_defer()` API.

- **One-shot duty cleanup.** If the duty scheduler
  doesn't natively support one-shot duties, add a
  `one_shot: bool` flag to the duty row and have the
  scheduler call `duty.complete()` after a successful
  fire. Skipping cleanup means the duty re-fires
  every 24h forever — a bug, not a feature.

- **No re-fire on retry.** If the channel send fails
  (network down, channel offline), the retry-with-
  backoff path retries WITHIN the same duty
  invocation. The duty itself fires once; retries are
  internal. If all retries exhaust, the duty audit-
  logs the failure and marks itself `completed` —
  better to skip the summary than to spam the user.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-hire-wrap-cadence` — per-tenant
  configurable cadence (24h / 48h / weekly).
- `mission-enterprise-hire-wrap-template` — operator-
  overridable template per workspace.
