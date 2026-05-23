# round 45 — PR #2169 anti-theater verify (post-merge)

- **Date**: 2026-05-23T22:27Z (Melbourne local Sun 2026-05-24 08:27 AEST)
- **Round**: 45
- **Pillar / Claim**: Claim 2 (Delegation is the unit) — `argo-cron` substrate surface lock
- **PR**: [#2169](https://github.com/mark-hallam/argo/pull/2169) `eval-argo-cron-crate-surface` — MERGED 2026-05-23T22:30:13Z (during this round, sha `e03680c99`)
- **Verdict**: 🟢🟢 **PASS** — surface contract is real structural pin, not smoke. Severity tier: P2 (claim-2 substrate audit closure).

## Why this PR matters

`argo-cron` is the scheduling substrate underneath claim 2 ("Delegation is the unit"). Direct downstream consumers — gateway cron daemon, `argo-cli system_jobs` handlers, `argo-orchestrator::DutyScheduler` — all depend on these primitives. A rename, signature change, or removal of any of the 24 `pub use` items would silently break callers at HEAD. This PR adds 23 contract tests that fail at compile time on any surface drift, locking the substrate while the brief author audits whether the right shape is exposed.

## Setup

```bash
git fetch origin pull/2169/head:pr-2169-verify
git checkout pr-2169-verify
cargo test -p argo-cron --test surface_contract --test system_job_registry_contract
```

Branch state at verify: `mergeable=UNKNOWN` (UNKNOWN means GitHub hadn't computed yet; merged shortly after at e03680c99). PR diff: 2 NEW test files (816 LOC combined) + doc-tree only; **zero production source touched**. This is a pure surface-pin substrate audit.

## Observed

### Test results — cold, both files

```
running 11 tests   (surface_contract.rs)
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 12 tests   (system_job_registry_contract.rs)
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

23 tests total — matches the PR body claim. Build went from cold (artifact-cache lock contention from parallel swarm builds) but completed in ~90s.

### Surface inventory pin

- **24-name `pub use` inventory** — import block at `surface_contract.rs:60-69` IS the compile-fence (removing any re-export from lib.rs fails compilation). Sentinel array at `:84-110` cross-checks the count → "pub_use inventory drift" failure message guides the maintainer to re-update both surfaces. The runtime block at `:115-130` further pins each constant/function to the `argo-cron` crate-home (forces type signature lock).
- **CronExpr parse + serde round-trip** — 5 user-shaped cadences (daily / Sunday-midnight / 15-min comma-list / weekday-range / first-of-month). Each parses → asserts source preserved → JSON round-trip → deep-equality. `*/N` step syntax intentionally excluded with honest rustdoc reference to upstream rustdoc deferral.
- **CronParseError exhaustive variants** — `match` over all 3 variants with constructor cases + `to_string()` + `&dyn std::error::Error` cast. Plus 3 real-parse error cases (`* * * *`, `60 * * * *`, `* * 5-2 * *`) asserting matches map to right variant. Adding a variant fails compilation; deleting a variant fails the runtime case.
- **CronTarget struct literal + serde** — both pub fields (`platform`, `chat`) struct-literal pinned + JSON round-trip.
- **TickLock contract** — uses `tempfile::tempdir` for real fs2 file lock; first acquire wins + creates parent dirs; second `try_acquire` returns `Err(TickLockError::AlreadyHeld { path })` with the contended path surfaced; drop releases. Exhaustive variant match over both error variants.
- **Scheduler signature pin** — `Send + Sync + Clone + Copy + Default` bounds; `TickReport` 3-field struct literal pin; happy path (due job fires + recorded in `last_fired_at`) + idle path (no attempt/fire/error/runner invocation); `tick_with_audit` signature pinned via `None` sink (TickReport-equivalent to plain `tick`).

### Sibling registry/handler file

- **CronJob struct-literal pin** — 8 pub fields struct-literal use + per-field type pin. Adding a field forces this callsite to be updated.
- **CronJob::new constructor** — asserts default-enabled, no last_fired_at, non-nil UUIDv7.
- **Golden serde vs cron/system/*.json** — 5 real fixtures (ab_promotion, audit_evidence_archival, dreams_curation, forge_optimize, skill_quality_recompute), each: parse → assert platform == SYSTEM_PLATFORM → assert chat non-empty → re-serialize → re-parse → deep field equality. Catches schema drift between code and shipped JSON fixtures.
- **Required-field rejection** — mutates forge_optimize_tick.json by removing `target` key, asserts error msg contains "target". Catches accidental field-default additions that would let bad fixtures pass.
- **Malformed variant rejection** — injects `"NotARealFieldVariant"` into schedule.minute, asserts serde reject.

### Sensible-deviation honesty

The PR's module-level rustdoc enumerates 5 brief-vs-real deviations explicitly:

1. Brief says `Job`; real type is `CronJob`. Pinned as `CronJob`.
2. Brief says `CronTarget::{User { duty_id, worker_id }, System { job_kind }}` enum; real `CronTarget` is a struct `{ platform, chat }` with `platform="system"` convention.
3. Brief says `tick_lock returns Locked within claim_ttl`; real `TickLock::try_acquire` returns `Err(TickLockError::AlreadyHeld)` while file lock held. Same intent (exactly one tick wins) pinned via real API.
4. Brief says `SystemJobRegistry::list_due|mark_claimed|mark_complete`; real registry exposes `empty|get|has|names`. Pinned as real API.
5. Brief lists 4 canonical jobs; cron/system/ has 5 fixtures (4 in CANONICAL_SYSTEM_JOBS + audit_evidence_archival_tick which is shipped but not in the const). All 5 round-trip; count assertion pins `CANONICAL_SYSTEM_JOBS.len() == 4`.

This is the model brief-author-deviation handling we want: **pin the real surface, document the deviation, explain the intent-equivalence**. Conductor brief author got it wrong; worker fixed it without forcing a brief rewrite cycle.

## Anti-theater check

The 23 tests are not vacuous-pass theater:

- **Inventory test** — both halves are load-bearing: import block is the compile-fence (catches removed re-exports), sentinel array is the runtime cross-check (catches count drift). Either-side failure mode is surfaced.
- **Parse/serde tests** — assert deep equality after round-trip, not just `is_ok()`.
- **Exhaustive match tests** — `match` over enum forces compile-fail on variant addition/removal; the runtime case array proves each variant is constructible + Display-able + `error::Error`-castable.
- **TickLock** — uses real fs2 file lock against a tempdir, not a mocked semaphore; the second-acquire-fails behavior is the actual production semantic; drop-releases proves RAII guard works.
- **Scheduler happy/idle** — real `InMemoryJobStore` + real cron expression + real wall-clock `Utc.with_ymd_and_hms(...)` schedule; happy + idle paths exercise the real branch logic.
- **Golden fixtures** — reads real JSON files from the workspace (`workspace_root().join("cron/system")`), not synthetic fixtures embedded in the test. Catches schema drift between the runtime parser and the shipped JSON files.
- **Rejection tests** — assert that loosening serde (e.g., adding `#[serde(default)]`) would allow malformed docs through; this is the kind of safety-net the surface-pin tests exist to catch.

The brief author also did **not** add FINDINGS-only tests — they explicitly say "audit found no drift → no FINDINGS-ONLY tests + no fix-brief." This is an honest audit verdict, not a "we didn't look hard enough" pass.

## Gap / follow-up brief candidates

- **NONE** — the audit verdict is "no drift found." The substrate surface is locked, the deviation list documents the brief→reality gap honestly, and there's nothing for a follow-up brief to fix. The next claim-2 substrate audit should target the *consumer side* (gateway cron daemon, argo-cli system_jobs handlers) to verify they handle the real surface correctly.

22nd anti-theater-honest substrate contract on main this week. (Pre-merge bless of #2131 will be the 23rd.)
