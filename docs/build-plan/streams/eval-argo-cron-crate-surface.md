# eval-argo-cron-crate-surface

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `a39e6d6ee` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-cron-crate-surface`.

🟢 **P2 CLAIM-2 SUBSTRATE.** argo-cron public-API surface contract pin.

## Why

`argo-cron` is the scheduling substrate for claim-2 ("Delegation is the unit") — every duty fire flows through `Scheduler` or `argo_orchestrator::DutyScheduler` (which delegates to argo-cron primitives `CronExpr` + `Job` + `tick_lock` + `SystemJobRegistry`). The gap-manifest already flipped argo-orchestrator's `DutyScheduler` to 🟢 via `eval-cron-duty-fires-agent-runner-end-to-end` (#2106 wired it; #2088 closed the broken-wire), but the **argo-cron crate itself has no surface-contract pin** — the only direct tests live in `crates/argo-cron/src/tests.rs` (in-module unit tests). Sibling pattern: `eval-argo-skill-sdk-surface` (#2132) — surface-pin tests at `tests/surface_contract.rs` enumerate pub items, lock invariants, fail-on-drift.

## Scope

Walk every pub export in `crates/argo-cron/src/lib.rs` and pin the contract that downstream callers depend on. Specifically:

1. **`CronExpr` parse contract** — golden round-trip: every cadence string in `cron/system/*.json` (4 canonical jobs) + 5 user-cadence shapes (`@every Nm`, `@every Nh`, `0 6 * * *`, `0 0 * * 0`, `*/15 * * * *`) parses, serializes, and round-trips. `expr.rs` is ~LOC; deserved a serde + Display round-trip pin.
2. **`Job` shape pin** — every pub field of `Job` named in the surface_pin test; serde round-trip via `cron/system/forge_optimize_tick.json` golden fixture; reject malformed JSONs (missing `target`, unknown `target` variant, malformed cadence).
3. **`CronTarget` exhaustiveness** — `User { duty_id, worker_id }` + `System { job_kind }` variants; exhaustive-match sentinel test that fails to compile if a 3rd variant is added without test coverage.
4. **`tick_lock` claim semantics** — given a `SystemJobRegistry` + a fixed `now`, exactly one tick wins claim; subsequent ticks within `claim_ttl` return `Locked`. Pin the contract; the file already has unit tests, lift them to integration shape.
5. **`SystemJobRegistry` trait contract** — `list_due` returns deterministic order; `mark_claimed`/`mark_complete` are idempotent on second call; trait object is `Send + Sync` (needed by gateway boot). Compile-time bounds check.
6. **`scheduler.rs` `Scheduler::tick` happy + idle paths** — at least one positive test (with mock `argo-store`'s `DutyStore`) that asserts a due job produces a tick row.

## Anti-theater requirement

Each pin assertion must fail loud if the production behaviour drifts. Forbidden: `assert!(true)`, `assert!(!result.is_err())` without inspecting Ok value, `assert_eq!(x.len() > 0, true)`. Pin shape mirrors `eval-argo-skill-sdk-surface` PR #2132's `surface_contract.rs`: field-by-field, variant-by-variant, exhaustive enumeration.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cron/tests/surface_contract.rs` (NEW)
- `crates/argo-cron/tests/system_job_registry_contract.rs` (NEW, optional split)
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production source in `crates/argo-cron/src/`. Audit-only; if a real bug is found, ship as `#[ignore]`'d FINDINGS-ONLY test + name a fix-brief.
- `argo-orchestrator::DutyScheduler` (covered by `eval-cron-duty-fires-agent-runner-end-to-end`).
- `argo-cli/src/system_jobs/*` handlers (covered by `mission-cron-tick-maintenance-jobs` #1379).

## Acceptance criteria

- [ ] `cargo test -p argo-cron` includes a new `surface_contract` test suite, all passing.
- [ ] Every pub item in `lib.rs` re-exports appears in the pin (count-matched against `cargo doc` JSON, or comparable structural enumeration).
- [ ] `cron/system/*.json` golden fixtures round-trip through `Job` serde.
- [ ] If real cross-tenant leak / broken-wire / drift found: ship as `#[ignore]`'d FINDINGS-ONLY test + name a fix-brief.
- [ ] `cargo clippy -p argo-cron --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Estimated size

~250-450 LOC of test code, single crate, no production touch.
