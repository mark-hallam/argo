# fix-pr-2290-rebase-kpi-rollup-against-event-kind-split

**Owner**: claude or codex.
**Authored against**: origin/main `08920d212` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pr-2290-rebase-kpi-rollup-against-event-kind-split`.
**Blocks**: `mission-claim-2-kpi-rollup-rebase-and-resubmit` (PR #2290) — currently CONFLICTING.

🟡 **P1 OPS — rebase PR #2290 against current main to clear 3 event_kind.rs conflicts.**

Round-59 evaluator VERIFIED #2290's intent (SQL touches only `kpi_json`, 6th regression test mirrors round-55 probe non-vacuously) but PR went CONFLICTING because #2292 (event-kind split) merged 10min after #2290 opened. Both touch `crates/argo-audit/src/event_kind.rs`. Comment posted on PR #2290 with the conflict-resolution recipe — this brief lets a fresh worker take it over if the original author doesn't respond.

## Scope

`gh pr checkout 2290 && git fetch origin main && git rebase origin/main`. 3 conflicts at L50-63 / L103-112 / L226-235, all in `event_kind.rs`:

- **L50-63** — adding `WorkerKpiRolled` variant: keep HEAD's compact 1-line `///` doc style (post-split). Add the variant in compact form. Move the verbose doctext to `crates/argo-audit/src/event_kind_docs.rs` (canonical home post-#2292).
- **L103-112 + L226-235** — `ContextWindowCompressed` + `WorkplaceKnowledgeDocUpdated`: take HEAD (compact docs). Verbose doctext already in `event_kind_docs.rs` post-#2292.

Then: `cargo check -p argo-audit --tests` clean + `cargo test -p argo-audit kpi_rollup -- --include-ignored` passes (6 tests incl. `rollup_does_not_clobber_existing_charter`) + `git push --force-with-lease` + comment "rebased, ready for re-merge".

## Acceptance criteria

- [ ] PR #2290 is MERGEABLE per `gh pr view 2290 --json mergeable`.
- [ ] `event_kind.rs` ≤ 225 LOC.
- [ ] `WorkerKpiRolled` variant present + compact-style doc.
- [ ] `event_kind_docs.rs` has the WorkerKpiRolled verbose-doc section.
- [ ] 6 tests pass cold (incl. `rollup_does_not_clobber_existing_charter`).

## May-touch

- `crates/argo-audit/src/event_kind.rs` (rebase resolution)
- `crates/argo-audit/src/event_kind_docs.rs` (add WorkerKpiRolled verbose-doc section)
- This brief file (status updates)

## Out of scope

- Touching the rollup scheduler / set_kpis API (already correct per round-59)
- Adding new variants beyond WorkerKpiRolled

## Effort

~15-30 min — mechanical rebase + cargo verify.
