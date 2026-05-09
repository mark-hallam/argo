# claude-runbook-skycot-split — split RUNBOOK.md into topic-grouped files

**Owner**: claude. **Depends on**: nothing. **Phase A operations narrow.**

**Authored against**: origin/main 89e16c7b on 2026-05-09 (conductor-authored to unblock 3 docs-side BLOCKED rows: claude-coverage-threshold-enforcement, claude-error-dashboard-runbook, claude-startup-health-check — each blocked because `docs/skycot/RUNBOOK.md` is currently 1088 LOC, 2× the 500-LOC ceiling).

## Why

`docs/skycot/RUNBOOK.md` is the canonical operations document but has grown to **1088 LOC**, well past the project's 500-LOC-per-file ceiling. Multiple Phase C briefs that need to *append* operational sections to it are currently BLOCKED on this single ceiling violation.

Splitting into topic-grouped sub-files unblocks the queue, makes the RUNBOOK navigable (a single 1k-line markdown file is hard to scan), and preserves the canonical reference structure via a thin index.

## Approach (concrete)

1. **Read `docs/skycot/RUNBOOK.md` end-to-end**. Identify natural section boundaries (operations, security, observability, troubleshooting, deployment, etc.). The current section headers are the cut lines.
2. **Split into 3-5 topic-grouped sub-files** at `docs/skycot/runbook/<topic>.md`. Suggested split (worker may adjust based on actual content):
   - `runbook/operations.md` — start/stop/restart, daemon, configuration
   - `runbook/security.md` — auth, secrets, rate-limit, audit
   - `runbook/observability.md` — logs, metrics, traces, alerts, dashboards
   - `runbook/troubleshooting.md` — common errors, recovery procedures
   - `runbook/deployment.md` — install, upgrade, backup, restore
3. **Reduce `RUNBOOK.md` to a thin index** (~50-80 LOC) that lists the topic files with one-line descriptions and links. Each link should be a relative path: `[Operations](runbook/operations.md)`.
4. **Update cross-references**. Run `rg 'docs/skycot/RUNBOOK\.md' --type-not rust` and update any link that targets a section now in a sub-file. Common consumers: README.md, BUILD_STATUS.md notes, brief streams.
5. **Verify every sub-file is < 500 LOC**. If any topic has more content than fits, split further (e.g. `observability/logs.md` + `observability/metrics.md`).

**Pure docs change. No Rust touched. No test verification needed beyond `wc -l`.**

## Files you may touch

- `docs/skycot/RUNBOOK.md` — reduce to thin index (~50-80 LOC)
- `docs/skycot/runbook/*.md` (NEW, each ≤ 500 LOC)
- `README.md` — narrowly: update any RUNBOOK.md section-link
- Any file caught by the cross-reference grep — narrowly: link path update only

## Files you must NOT touch

- Any Rust file
- `docs/build-plan/streams/*.md` — those are brief specifications; if a brief references a RUNBOOK section, the link still works through the index → sub-file. Do not rewrite brief texts.
- `docs/build-plan/BUILD_STATUS.md` — conductor maintains this.
- Any `.github/workflows/*.yml`

## Acceptance criteria

- [ ] `docs/skycot/RUNBOOK.md` is now an index file, ≤ 100 LOC
- [ ] Every `docs/skycot/runbook/*.md` is ≤ 500 LOC
- [ ] Total content preserved (no information lost — `wc -l` on all new files combined ≥ 1088 minus the index reduction)
- [ ] Cross-references in README.md (and anywhere else `rg` finds them) updated to the new paths
- [ ] `gh pr view --json mergeable` returns MERGEABLE post-push

## Branch + PR

- Branch: `claude-runbook-skycot-split`
- PR title: `docs: split RUNBOOK.md into topic-grouped files (claude-runbook-skycot-split)`

## Notes for the implementer

- This is the same split pattern proven 9× in the Rust source (cli.rs split-into-cli_command_enum.rs etc.). Pure docs equivalent.
- The cut lines should follow existing `## H2` boundaries in RUNBOOK.md — do not invent new structure.
- Preserve the original section ordering across sub-files (so the index reads top-to-bottom in the same order as the original RUNBOOK.md).
- If a single `## H2` section is itself > 500 LOC, split it across two sub-files with `<topic>-1.md` / `<topic>-2.md` (or finer-grained naming).

## Unblocks (downstream briefs that depend on this)

After this lands, the conductor will flip these 3 currently-BLOCKED rows to TODO:
- `claude-coverage-threshold-enforcement` (row 136)
- `claude-error-dashboard-runbook` (row 139)
- `claude-startup-health-check` (row 144)
