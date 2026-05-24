# eval-process-registry-admin-escape-hatch-sweep

**Owner**: codex.
**Depends on**: `mission-process-registry-tenant-keyed` merged via PR #2149.
**Authored against**: origin/main `8da732a7d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-process-registry-admin-escape-hatch-sweep`.

🟢 **P3 EVALUATOR FOLLOW-UP.** The process-registry tenant-key audit accepted
`ProcessRegistry::total_len()` as an explicit admin-only escape hatch, but the
follow-up evaluator note warned that it was uncalled at merge time. If it stays
uncalled after additional rounds, it should either be deleted or pinned as a
deliberate admin surface.

## Source

- `docs/skycot/evals/scheduled/2026-05-24/round-38-pr-2149-process-registry-tenant-key-anti-theater-verify.md`
  §recommendation.
- Current anchor: `crates/argo-tool/src/process_registry.rs`.

## Scope

- Audit current `origin/main` for all `total_len()` callsites.
- If there is still no production caller, choose one of two paths:
  - **Preferred path A**: remove the unused `total_len()` escape hatch and
    update tests if they referenced it.
  - **Path B**: keep it only if there is a concrete admin caller or documented
    imminent caller, then add a narrow test/report note explaining why the
    cross-tenant aggregate is intentional.
- Write a short evaluator report with the decision and evidence.

## May-touch

- `docs/skycot/evals/scheduled/2026-05-24/` — one new report.
- `crates/argo-tool/src/process_registry.rs` — only for deleting or documenting
  the escape hatch.
- `crates/argo-tool/tests/` — only if the chosen path needs a pin or cleanup.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip only.

## Must not touch

- Agent-runner process-registry wire-up.
- Gateway or terminal process dispatch behavior.
- Tenant-keyed registry filtering semantics shipped by PR #2149.

## Acceptance criteria

- [ ] Report names every current `total_len()` callsite, or states none exist
      with the exact `rg` command used.
- [ ] If no production caller exists, the escape hatch is removed unless the PR
      body gives a concrete admin-retention reason.
- [ ] If retained, a test or report assertion makes the admin-only aggregate
      intent explicit.
- [ ] `cargo clippy -p argo-tool --all-targets -- -D warnings` clean if any
      Rust file is touched; docs-only path may skip cargo and say why.
- [ ] Every modified file stays under 500 LOC.

