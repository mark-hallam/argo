# claude-worker-rotation-guidance — frame 2-4 PR sessions as healthy norm

**Owner**: claude. **Depends on**: nothing. **Phase A — process improvement.**

## Why

3 worker self-exits this session converged on the same observation: context-pressure surfaces meaningfully after ~4 multi-crate PRs in one session. Two prior tabs exited at 2 PRs explicitly citing "historical 2-PR exits are healthy here." The third exited at 3 PRs citing "context window meaningful after 4 implementation cycles."

Currently CLAUDE.md frames this as graceful degradation in STOP CONDITIONS. The empirical pattern is that 2-4 PRs is the **healthy session norm**, not a failure mode. Workers may be holding on past their best work because the protocol implies "exit only when degraded."

This brief reframes worker-tab rotation as expected operational behavior.

## Scope

Narrow CLAUDE.md amendment to:

1. **GRACEFUL EXIT PROTOCOL section** — add positive framing: "Workers commonly observe productive sessions of 2-4 PRs before context-pressure surfaces. Exit cleanly within that range; rotate to a fresh tab. Pushing past 4 PRs in one session is not an error but watch self-degradation indicators tightly."
2. **STOP CONDITIONS section** — soften STOP CONDITION 4 (self-degradation 4+) doc-comment to note: "Note: most healthy sessions exit before this threshold fires, via PR-count-based natural rotation."
3. **PROTOCOL_WORKER_PROMPT_*.md** — add 1-line note: "Healthy sessions ship 2-4 PRs then exit cleanly. Don't push past 4 with multi-crate work unless you're tight on indicators."

**Do NOT** add a hard PR-count cap. The intent is permission, not mandate. Workers shipping a productive 6-PR session shouldn't feel they violated protocol.

## Files you may touch

- `CLAUDE.md` (GRACEFUL EXIT PROTOCOL + STOP CONDITION 4 narrow rewording)
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CLAUDE.md` (1-line addition)
- `docs/build-plan/PROTOCOL_WORKER_PROMPT_CODEX.md` (1-line addition)

## Files you must NOT touch

- BUILD_STATUS.md (no status change)
- Any brief file (this is process, not features)

## Acceptance criteria

- [ ] GRACEFUL EXIT PROTOCOL section explicitly states 2-4 PR sessions are healthy norm
- [ ] STOP CONDITION 4 reworded to note natural rotation typically fires first
- [ ] Both worker prompts updated with 1-line guidance
- [ ] No hard PR cap introduced
- [ ] `cargo build` clean (pure docs change)

## Branch + PR

- Branch: `claude-worker-rotation-guidance`
- PR title: `protocol: 2-4 PR sessions framed as healthy worker norm (claude-worker-rotation-guidance)`

## Notes

- Evidence base: 3 worker self-exit notes this session (placid-curie, quirky-noether, plus the latest 3-PR one) all converged on same pattern.
- This is permission, not mandate. The wording matters — "common norm" not "expected limit."
