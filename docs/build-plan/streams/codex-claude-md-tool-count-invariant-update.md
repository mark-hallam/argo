# codex-claude-md-tool-count-invariant-update — Update CLAUDE.md core-tool invariant to match reality

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `25821ea8` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-claude-md-tool-count-invariant-update`.
**Source**: F-008 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. Operator decision (2026-05-15): update doc to match reality, do NOT migrate the 7 overflow tools.

## Scope

P2 doc fix. `CLAUDE.md:50` states the invariant `MCP-first integration (8 in-core tools; everything else MCP)`. The current in-core surface is 15 tools (verifiable via `crates/argo-tool/src/tools/mod.rs:63` — `builtin_tools()` returns 15 items: FileRead, FileWrite, FilePatch, Terminal, WebSearch, WebExtract, Memory, DelegateTask, Vision, CodeExecute, SendMessage, SessionSearch, Todo, Clarify, Process).

Operator-locked decision 2026-05-15: keep all 15 in core, update the invariant text. The MCP-first DISCIPLINE remains — new tools default to MCP unless they meet a "must be in core" bar — but the literal "8" is replaced with the current count.

Scope:
- Update `CLAUDE.md:50` invariant text from "MCP-first integration (8 in-core tools; everything else MCP)" to something like "MCP-first integration (lean core, currently 15 in-core tools; new tools default to MCP unless they require direct AgentDeps access)".
- Update the rationale sentence below if it references the 8-tool count.
- If any other doc references the 8 count (search via `rg '8 in-core|8 core tools|canonical 8' docs/ CLAUDE.md`), update those too.
- Add a one-line decision-log note pointing at `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md` F-008 + operator decision.

LOC: ~40.

## Files you may touch

- `CLAUDE.md` — update invariant text + rationale. ≤25 modified lines.
- `docs/build-plan/PROTOCOL.md` — IF it cross-references the 8-tool invariant (check via grep first). ≤10 modified lines.
- `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md` — narrowly: append a single line under F-008 noting "RESOLVED 2026-05-15 by invariant update; see CLAUDE.md commit". ≤5 modified lines.

## Files you must NOT touch

- `crates/argo-tool/**` — no code changes. The 15 tools STAY in core; this is purely an invariant doc update.
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `CLAUDE.md:50` no longer says "8 in-core tools".
- [ ] New invariant text preserves the MCP-first discipline ("default to MCP for new tools").
- [ ] No other doc references the obsolete "8" count.
- [ ] Decision-log line added in F-008 audit row.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-claude-md-tool-count-invariant-update`
- PR title: `docs(claude-md): update core-tool invariant from 8 to current count (codex-claude-md-tool-count-invariant-update)`

## Notes for the implementer

- This is the cleanest path to close F-008. No code migration, no MCP server scaffolding, no risk.
- Preserve the discipline language ("lean core, MCP-first") — only the literal count changes.
- If you find additional docs referencing the 8-count, fix them in this PR (don't author followups).
