# codex-tokio-task-spawn-error-propagation-audit

**Authored against**: `origin/main` `32631c0ae` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-tokio-task-spawn-error-propagation-audit`.

**Fast-mode eligible**: yes — mechanical audit.

## Summary

`tokio::spawn` returns a `JoinHandle<T>` whose error state is dropped if the handle isn't awaited. A spawned task that panics silently disappears unless someone catches the `JoinError`. Audit every `tokio::spawn` callsite in the workspace and confirm the error is either awaited, logged, or explicitly intentional (e.g. fire-and-forget background work that's documented as such).

## Acceptance

- [ ] `rg -n 'tokio::spawn\|task::spawn' crates/` enumerates every callsite.
- [ ] PR body classifies each:
  - **AWAITED-OR-LOGGED** — `JoinHandle` is `.await?`-ed or wrapped in a `tokio::spawn(async move { match work().await { Err(e) => tracing::error!(...) } })` pattern. OK.
  - **STORED-FOR-LATER-AWAIT** — handle is stored in a struct and awaited at shutdown. OK.
  - **FIRE-AND-FORGET-DOCUMENTED** — handle dropped intentionally with a `// fire-and-forget: ...` comment explaining why. OK.
  - **FIRE-AND-FORGET-UNDOCUMENTED** — handle dropped silently, no comment. **BUG-LIKE**.
- [ ] For each UNDOCUMENTED-FIRE-AND-FORGET: either (a) add a 1-line comment explaining the intent, or (b) add `tracing::error!` on the `JoinError` if the spawned task should never silently fail. If ≤5 callsites, fix in-brief; larger gaps spawn follow-up briefs.
- [ ] No semantic change to existing task lifecycles.
- [ ] PR body has audit table.

## May touch

- Crate files where a comment-add or tracing-call is the fix.
- Test files only if surfacing a real bug.

## Must NOT touch

- Task scheduling logic (the spawned task's body).
- Any production semantics — pure observability addition.

## Notes

- Pre-flight: `rg 'tokio::spawn\|task::spawn' crates/ --count-matches | sort -t: -k2 -nr | head -10` to find the busiest crates.
- v1.0 launch hygiene — silent panics in spawned tasks are the kind of bug that produces "the app was running fine then suddenly stopped responding" reports.
- Sizing: 100–300 LOC.
