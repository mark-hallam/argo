# Marketplace seller — operator decision flow

A typical worker run hits 6 decision points. This file walks
each one with concrete "approve / retry / fail / abandon"
guidance. Pair with `sample-prompts.md` (what to ask for) and
`sample-bundle/` (what the worker hands back).

## 1. After `argo work serve` finishes

**What to look at first:**

```bash
cat .skycot/outputs/<job_id>/result.json | jq .stop_reason
cat .skycot/outputs/<job_id>/result.json | jq .not_ready_to_post
cat .skycot/outputs/<job_id>/result.json | jq .actions
```

**Decision matrix:**

| `stop_reason`              | `not_ready_to_post` | What it means                                          | Do                                              |
|---------------------------|----------------------|--------------------------------------------------------|-------------------------------------------------|
| `task_complete`           | `false`              | Worker finished w/ verified evidence.                 | Read research/pricing; approve listings.        |
| `approval_required`       | `false`              | Drafts ready, evidence sufficient.                    | Read approval_checklist.md → approve.           |
| `approval_required`       | `true`               | Drafts ready BUT evidence thin → bundle BLOCKED.      | DO NOT approve. Retry with better prompt.       |
| `need_info`               | (n/a)                | Worker can't proceed — missing fact only you have.    | Read `stop_detail`, supply the fact, re-queue.  |
| `max_steps_reached`       | (n/a)                | Worker hit `--max-steps` ceiling.                     | Bump steps OR review partial output.            |
| `research_tools_unavailable` | (n/a)              | Vertical denies web tools — multi-step refused.       | Use `--max-steps 1` OR enable web tools.        |

## 2. Reviewing `approval_checklist.md`

The structure (set by the system prompt):

- **Safe to approve** — verified, sourced. Click approve.
- **Needs human verification** — plausible but partly sourced.
  YOU re-verify before approving.
- **Blocked** — worker couldn't back at all. DO NOT post.

If everything is in `Blocked`, the worker should have
stopped with `[STOP: NEED_INFO]`. If it stopped with
`[STOP: APPROVAL_REQUIRED]` instead, the checklist's BLOCKED
banner is your last line of defence.

## 3. Per-action approval

```bash
argo worker actions list --state pending --json | jq -c '.'
```

For each row:

- **autonomous-safe** (`requires_approval: false`) — `execute`
  directly; no approve step.
- **approval-gated** (`requires_approval: true`) —
  `approve` then `execute`.
- **blocked** — investigate `blocked_reason` first. Don't
  override; either fix the upstream cause + retry or fail.

**When to NOT approve:**

- `approval_checklist.md` flagged the action's evidence as thin.
- The payload has anything you didn't say in your prompt
  (workers occasionally invent product features — verify before
  approving).
- The platform isn't one you actually want to post to (typo or
  hallucination).

## 4. Carrying the action out (execute)

```bash
argo worker actions execute <action_id>
cat .skycot/outputs/<job_id>/actions/<action_id>/instructions.md
```

The instruction template walks you through the platform-
specific steps. Follow it line by line; don't skip. Common
issues:

- **Photos missing** — the worker doesn't have access to your
  camera. The instructions list what photos are required;
  upload them yourself before clicking publish.
- **Category miscategorisation** — for items the algorithm
  can't auto-detect, override.
- **Listing format** — `Auction` vs `Buy it Now` is your call;
  the worker suggests but doesn't enforce.

## 5. Recording the result

After the action lands externally:

```bash
argo worker actions complete <action_id> \
  --note "live on ebay" \
  --result-json '{"listing_url": "...", "platform": "ebay"}'
```

If it fails:

```bash
argo worker actions fail <action_id> --reason "ebay rejected: needs more photos"
```

`failed` is terminal — to retry, use `argo worker actions retry`
which emits a fresh action that references the failed one in
its evidence.

## 6. End-of-day review

```bash
argo worker actions render
cat .skycot/approval_queue.md
cat .skycot/completed_actions.md
cat .skycot/blocked_actions.md
```

Three numbers to track:

- `approval_queue.md` size — how much human review backlog.
- `completed_actions.md` — your throughput.
- `blocked_actions.md` — your investigation queue (these are
  bugs in either the prompt or the manifest).

If `blocked_actions.md` keeps growing and you can't tell why,
re-read `docs/skycot/operator_runbook.md`'s troubleshooting
section.
