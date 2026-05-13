# claude-runbook-md-split — extract heaviest sections from RUNBOOK.md to per-topic files

**Owner**: claude. **Depends on**: nothing (pure docs refactor).
**Authored against**: origin/main `2e4ee70c` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-runbook-md-split`.

## Scope

Pure docs refactor. `docs/skycot/RUNBOOK.md` is at **913 LOC** — well over the 500-LOC project ceiling. This has been blocking the two **codex doc briefs** (`claude-error-dashboard-runbook` + `claude-troubleshooting-guide-expand`) since 2026-05-08, both of which need to append to this file. Today (2026-05-13) those two rows have been CLAIMED with no progress for **3+ days** — conductor is demoting them back to TODO in the same commit as authoring this brief so a fresh worker can re-claim once the split lands.

The `docs/skycot/runbook/` per-topic directory already exists (27 sub-files like `oom-restart.md`, `e2b-reap-idle.md`, `incident-response.md`, etc.) — the pattern is established and operators expect topic-level files. This brief just extends the pattern by extracting the heaviest H2 sections still living in the parent `RUNBOOK.md`.

## Approach

Extract these 5 sections to dedicated per-topic files in `docs/skycot/runbook/`:

| H2 section in RUNBOOK.md | Approx LOC | Target file |
|---|---|---|
| `## Genesis API` (lines 80-197) | 118 | `docs/skycot/runbook/genesis-api.md` |
| `## At-rest encryption` (lines 816-913) | 98 | `docs/skycot/runbook/at-rest-encryption.md` |
| `## Context-window compression` (lines 384-473) | 90 | `docs/skycot/runbook/context-window-compression.md` |
| `## Sandbox pre-warming` (lines 555-636) | 82 | `docs/skycot/runbook/sandbox-prewarming.md` |
| `## Action throttling` (lines 474-554) | 81 | `docs/skycot/runbook/action-throttling.md` |

For each extracted section in `RUNBOOK.md`, replace the section body with a brief 2-3 line stub:

```markdown
## Genesis API

See [`docs/skycot/runbook/genesis-api.md`](runbook/genesis-api.md) for the full Genesis API operator guide.

— short 1-2 sentence pitch summarising what's in the linked file.
```

Each extracted file gets a `# <topic>` H1 at the top (matching the H2 it replaced), and the H2 anchors inside become H1+H2 in the new file (one level promoted). Hyperlinks elsewhere in the repo that pointed at `RUNBOOK.md#genesis-api` should still resolve through the stub redirect; if any internal links break, the implementer fixes them in this PR.

After split:
- `RUNBOOK.md` ≤ 500 LOC (target: ~450; 913 - 469 (sum of 5 sections) + 25 (stub overhead) = ~470)
- Each new sub-file ≤ 150 LOC
- Pattern: every section in RUNBOOK.md > ~75 LOC has been considered for extraction

Headroom: future doc-append briefs (the demoted codex ones + others) can either edit RUNBOOK.md (now ~450 LOC, ~50 LOC of runway) or, more cleanly, add their content to a focused sub-file.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `docs/skycot/RUNBOOK.md` — remove the 5 sections, replace with stub redirects.
- `<NEW: docs/skycot/runbook/genesis-api.md>` — receives lines 80-197 (with H2→H1 promotion).
- `<NEW: docs/skycot/runbook/at-rest-encryption.md>` — receives lines 816-913.
- `<NEW: docs/skycot/runbook/context-window-compression.md>` — receives lines 384-473.
- `<NEW: docs/skycot/runbook/sandbox-prewarming.md>` — receives lines 555-636.
- `<NEW: docs/skycot/runbook/action-throttling.md>` — receives lines 474-554.
- Any file containing a broken `RUNBOOK.md#<anchor>` link to one of the extracted sections — narrowly scoped: fix the link. Run `rg 'RUNBOOK\.md#(genesis-api|at-rest-encryption|context-window-compression|sandbox-prewarming|action-throttling)' .` to find them. Probably 0-3 hits.

## Files you must NOT touch

- Code files — this is docs-only.
- Other sub-files in `docs/skycot/runbook/` unless they contain a broken link to one of the extracted sections.
- Any `## H2` section in `RUNBOOK.md` not listed in the table above (e.g., `## Topics`, `## Quick links by task`, `## Approvals JSON API`, etc. — those stay put).

If your task genuinely requires a file not listed, **stop and mark this brief BLOCKED in BUILD_STATUS.md** with a one-line reason.

## Acceptance criteria

- [ ] `RUNBOOK.md` ≤ 500 LOC after the split (target ~450 — comfortable headroom).
- [ ] All 5 new sub-files exist under `docs/skycot/runbook/` with the content from the corresponding section verbatim (only modification: H2 anchors inside each promoted to H1 + H2).
- [ ] Each new sub-file has an `# <Topic>` H1 line at the top.
- [ ] Each removed section in `RUNBOOK.md` is replaced with a 2-3 line stub redirect.
- [ ] No broken internal links: `rg 'RUNBOOK\.md#' .` returns no anchor-hits at one of the 5 extracted sections (anchors pointing at remaining RUNBOOK sections are fine).
- [ ] `markdownlint docs/skycot/RUNBOOK.md docs/skycot/runbook/*.md` clean (or whatever lint the project uses — check `package.json` / Makefile).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-runbook-md-split`
- PR title: `docs: extract heaviest RUNBOOK.md sections to per-topic files (claude-runbook-md-split)`

## Notes for the implementer

- Mechanical. Move section content verbatim; promote H2 anchors inside to H1+H2.
- The split-prep is `*-split`-suffixed so the pre-claim ceiling check auto-skips (`scripts/check-pre-claim-ceiling.sh:45`).
- If a section has a code block (likely — these are API + ops sections), preserve the language fences exactly.
- The 2 demoted codex doc briefs (`claude-error-dashboard-runbook` + `claude-troubleshooting-guide-expand`) will become claimable once this lands — they target `RUNBOOK.md` directly but the file will have ~50 LOC of headroom for their additions. A subsequent rewrite of those briefs to target sub-files instead is fine but not required.
- This brief unblocks 2 long-stale CLAIMED rows. Priority: ship within the next 2 ticks if possible.
