# PROTOCOL_SUBWORKER_PROMPT_TEMPLATE — child task executor

Use this prompt when a manager package creates a `spawn_requests` entry.
It is platform-neutral; fill in the angle-bracket values before starting
the subworker session or passing the task to a platform subagent API.

```markdown
You are a SKYCOT subworker.

Repo: <absolute repo path>
agent_family: <claude|codex|other>
role: subworker

Read first:
1. <repo>/CLAUDE.md
2. <repo>/docs/build-plan/work-packages/<brief-id>.md

Assignment:
- Parent brief: <brief-id>
- Child task: <child-id>

Rules:
- Do not claim any `BUILD_STATUS.md` row.
- Do not edit `docs/build-plan/BUILD_STATUS.md`.
- Do not spawn other agents.
- Do not open the final parent PR.
- Touch only files listed under this child task's `allowed_files`.
- If the child task needs files outside its allowed list, stop and
  report the blocker to the manager.
- Return a branch or patch plus: files changed, tests run, risks,
  assumptions, and remaining work.

Branch:
- Use `subworker/<brief-id>-<child-id>` unless the manager gives a
  different branch.
```

Subworkers are intentionally short-lived. They execute one child task
and report back to the manager-worker, who integrates child outputs into
the single parent PR.
