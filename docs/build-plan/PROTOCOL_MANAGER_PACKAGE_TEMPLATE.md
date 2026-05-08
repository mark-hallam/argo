# PROTOCOL_MANAGER_PACKAGE_TEMPLATE — bounded child work

A manager package is used only when a parent brief explicitly allows
manager-worker mode in its `## Parallelism` section. The package tracks
child work inside one claimed parent brief. It is not a replacement for
`BUILD_STATUS.md`.

Save packages as:

`docs/build-plan/work-packages/<parent-brief-id>.md`

The parent row remains the only global ledger row. Child tasks do not get
their own `BUILD_STATUS.md` rows unless the conductor promotes them.

---

## Template

Copy this block into a new package file and fill in the values.

```yaml
parent_brief: <brief-id>
parent_brief_file: docs/build-plan/streams/<brief-id>.md
global_ledger_row: docs/build-plan/BUILD_STATUS.md:<brief-id>

manager:
  agent_family: <claude|codex|other>
  agent_id: <stable-agent-name>
  role: manager_worker

status: planning
spawn_budget: 3
max_depth: 1
integration_branch: <family>/<brief-id>
draft_pr: null
final_pr: null

lease:
  claimed_at: <YYYY-MM-DDTHH:MM:SSZ>
  heartbeat_at: <YYYY-MM-DDTHH:MM:SSZ>
  expires_at: <YYYY-MM-DDTHH:MM:SSZ>

spawn_policy:
  mode: automatic_if_supported
  preferred_families:
    - codex
    - claude
  max_parallel_children: 3

parent_scope:
  allowed_files:
    - <copy from parent may-touch list>
  shared_hotspots:
    - Cargo.toml
    - Cargo.lock
  acceptance_criteria:
    - <copy parent criteria or summarize exact checklist>

children:
  - id: child-1
    status: ready
    goal: <narrow executable goal>
    assigned_to:
      agent_family: null
      agent_id: null
    allowed_files:
      - <must be subset of parent_scope.allowed_files>
    forbidden_files:
      - docs/build-plan/BUILD_STATUS.md
    success_criteria:
      - <verifiable child outcome>
    output:
      branch: null
      patch: null
      summary: null
      tests_run: null
      risks: null
      remaining_work: null

spawn_requests:
  - child_id: child-1
    role: subworker
    preferred_family: codex
    status: pending
    assigned_agent: null
    prompt_template: docs/build-plan/PROTOCOL_SUBWORKER_PROMPT_TEMPLATE.md

review:
  reviewer_required: true
  reviewer_agent:
    agent_family: null
    agent_id: null
  findings: []

promotion_requests: []
```

---

## Status Values

Parent row in `BUILD_STATUS.md`:

- `CLAIMED` — manager owns the parent package.
- `IN_REVIEW` — one integrated parent PR is open.
- `MERGED` — conductor merged the parent PR.
- `BLOCKED` — manager cannot finish inside parent scope.

Manager package:

- `planning`
- `children_in_progress`
- `integrating`
- `ready_for_review`
- `blocked`
- `abandoned`

Child task:

- `ready`
- `claimed`
- `done`
- `failed`
- `superseded`

---

## Rules

- `max_depth` is always `1`; subworkers cannot spawn more agents.
- `spawn_budget` is a hard cap, not a target.
- Child `allowed_files` must be a subset of the parent brief's may-touch
  list.
- Automatic spawning is allowed only when `spawn_policy.mode` is
  `automatic_if_supported`, each `spawn_requests[].child_id` references
  a real child, and request count stays within `spawn_budget`.
- Platforms without a subagent API leave spawn requests as `pending` or
  `manual_required`; humans or wrappers may start those sessions.
- Subworkers must not edit `BUILD_STATUS.md`, parent brief files, or this
  package unless the manager explicitly assigns that child task.
- Subworkers return output to the manager; they do not open the final PR.
- The manager opens one integrated parent PR and links this package.
- Reviewer agents are read-only.
- The conductor is the only final merge authority.

---

## Promotion

A child task can be promoted to the global ledger only by the conductor.
Use `promotion_requests` when a child:

- touches files outside parent scope
- blocks another parent task
- needs human/product clarification
- exceeds the spawn budget
- becomes independently mergeable
- changes shared architecture
