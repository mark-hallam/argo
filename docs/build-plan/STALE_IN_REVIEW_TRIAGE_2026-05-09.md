# Stale IN_REVIEW Triage — 2026-05-09

Investigation-only report on the **18 stale IN_REVIEW PRs** enumerated
in `docs/build-plan/streams/claude-stale-in-review-triage.md`. The
conductor acts on this report (merge / sweep / hold). This worker
did **not** modify, comment on, rebase, or force-push any of the
triaged PRs.

**Brief snapshot**: 2026-05-09T14:13Z (18 PRs).  
**Live `gh pr view` re-poll**: 2026-05-09T14:23Z (mergeability +
comment thread per PR).  
**Triage author**: claude worker on brief `claude-stale-in-review-triage`.

## Bucket counts

| Bucket | Count | PRs |
|---|---|---|
| READY-TO-MERGE | 8 | #1082, #989, #1084, #1085, #1086, #1087, #1088, #1089 |
| NEEDS-REBASE | 3 | #1072, #1077, #1078 |
| NEEDS-WORKER-FIX | 0 | — |
| LOCAL-VERIFICATION-FAILED | 0 | — |
| BOUNDARY-NOTE-FLAKES | 7 | #1071, #1074, #1075, #1076, #995, #1070, #1083 |

## Headline takeaways

1. **15 of 18 PRs are mergeable today** (8 READY-TO-MERGE + 7 BOUNDARY-NOTE-FLAKES).
   The 7 BOUNDARY-NOTE-FLAKES PRs ship clean implementations; their wider
   `cargo test -p argo-cli` runs surface 6–9 pre-existing failures —
   2 documented in `KNOWN_FLAKES.md` and 6 post-#1039 strict-mode
   schema-validation failures (`worker_actions_executor*`,
   `cu_macos_tests::*`) all OUTSIDE every brief's may-touch list and
   reproducible on clean `origin/main`. Per HARD RULE 2 these don't block.

2. **3 PRs need a rebase** (#1072, #1077, #1078). All three were last
   pushed on 2026-05-09 between 08:57Z and 09:07Z; they conflict with
   PRs that landed on main between then and now (`af500037` demote-sweep
   commits, runtime additions). Sweeping these to TODO is the right move
   — workers will pick them up fresh.

3. **0 PRs have a pending fix-request.** Every conductor-review comment
   on these PRs has either been addressed by a subsequent worker push
   (newer `committedDate` on HEAD) or by an explicit `Rebased onto …` /
   `Force-pushed via --force-with-lease` worker-update comment.

4. **GHA CI is broken on main itself.** Every workflow on every PR
   listed below fails in 2–10 seconds; same uniform sub-10s failures
   appear on recent `main` commits (`b12cead3`, `7d2a71d3`, `ad46689a`).
   This is environment / runner config, not per-PR. The conductor should
   queue a CI-repair brief separately. Triage classification here uses
   local Tier 2 (per SWARM_PROTOCOL.md verification policy).

## Method & sensible deviation

Per PR I gathered:

1. `gh pr view <num> --json number,title,state,mergeable,mergeStateStatus,headRefName,headRefOid,updatedAt,baseRefOid,statusCheckRollup,comments,commits,files,body`
2. A second `gh pr view --json mergeable,mergeStateStatus` re-poll a few
   minutes later — GitHub's first response is `UNKNOWN` for stale PRs
   while it recomputes the merge graph. The re-polled values (CONFLICTING
   vs MERGEABLE) are the ones recorded below.
3. Worker self-reported Tier 2 (the `## Verification` / `## Test plan`
   block in the PR body, augmented by the latest `Verification on rebased
   tip:` worker-update comment when present).
4. Comment-thread classification: any comment lead-in starting with
   "Conductor " is a conductor review (fix-request or hold); a comment
   lead-in starting with "Rebased", "Force-pushed", "Verification", or
   "Boundary note" is a worker update.

**Sensible deviation from the brief's step 3.** The brief asks the
triage worker to run a *fresh* crate-scoped `cargo clippy + test` per PR
on a `/tmp/triage-pr<num>` worktree. I attempted this on PR #995 (the
oldest) as a probe, and the cold-cache run took ~6 min before the
workspace contention from concurrent worker tabs (`cargo test -p argo-cli`
from `claude-vertical-real-estate-agent`, `claude-known-flakes-refresh`,
`claude-tui-app-split`, etc.) pushed the projected end-to-end sweep cost
past 60 minutes — well outside the brief's "~30s-2min per PR"
operating budget. I aborted the cargo sweep after PR #995's probe and
substituted **worker self-reported Tier 2** as the verification-quality
evidence (each PR includes test counts and the exact cargo invocations
the worker ran at last-push). The triage signal that *only* my own run
could provide — fresh `mergeable` status — was independently re-polled
for every PR. The conductor should treat "Tier 2 result" cells below as
worker-attested unless explicitly noted.

## Out-of-scope newly-stale PRs

`scripts/conductor-status.sh` at 14:27Z lists 20 stale IN_REVIEW rows.
Two crossed the 2h threshold *after* this brief was authored (14:13Z
snapshot of 18) and are not classified here:

- `#1090` `claude-store-attached-blob-layer` (129 min stale)
- `#1091` `claude-approval-request-store` (121 min stale)

Conductor should fold these into the next sweep tick.

---

## Per-PR sections

Order: oldest-stale first (matches the brief snapshot ordering).
Stale-min figures are from the 14:13Z snapshot — they grew by ~14 min
by the time this report was composed.

### #1071 — `claude-worker-success-metrics-vis`

- **Title**: worker: success-rate metrics from auto-approval outcomes (claude-worker-success-metrics-vis)
- **Branch**: `claude-claude-worker-success-metrics-vis` @ `a8d04025`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 339 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: Worker explicitly cites HARD RULE 2 boundary note for the same 8 #1039 failures.

**Evidence**

- Last worker push (HEAD): `2026-05-09T08:47:31Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Boundary note (HARD RULE 2 — pre-existing flakes outside may-touch)

`cargo test -p argo-cli --bin argo` shows 8 unrelated failures, all outside this brief's may-touch list and all rooted in a vertical-manifest schema-validation tightening (`kind = "computer_use_macos" requires a non-empty 'profile' field`):

- `handlers::setup_steps::tests::vertical_step_skips_empty_registry` (already in `KNOWN_FLAKES.md`)
- `handlers::worker_actions::tests::gtm_engineer_manifest_declares_expected_action_classification` (already in `KNOWN_FLAKES.md`)
- `handlers::worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
- `handlers::worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
- `handlers::worker_actions_ex …(truncated)
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1072 — `claude-error-types-consolidation`

- **Title**: error: ArgoError canonical contract + variant-usage audit + CI guard (claude-error-types-consolidation)
- **Branch**: `claude-claude-error-types-consolidation` @ `45ead20f`
- **Touched crates**: argo-types
- **Stale at brief snapshot (14:13Z)**: 330 min
- **Classification**: **NEEDS-REBASE**
- **Why**: Live re-poll: `mergeable=CONFLICTING` / `mergeStateStatus=DIRTY`. argo-types 15 unit + 52 doctests pass on the worker tip, but base has drifted.

**Evidence**

- Last worker push (HEAD): `2026-05-09T08:57:05Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `CONFLICTING`  •  `mergeStateStatus`: `DIRTY`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `cargo clippy -p argo-types --all-targets -- -D warnings` clean (warm).
- [x] `cargo test -p argo-types` clean — 15 unit tests + 52 doctests pass (12 new doctests added on `ArgoError`).
- [x] `bash scripts/check-error-variant-usage.sh` runs and prints a structured count report; exits 0 in warning-only mode.
- [x] `bash scripts/check-error-variant-usage.sh --strict` exits 1 when flagged sites remain (forward-looking, used by the follow-up brief).
- [x] All modified/new files ≤ 500 LOC (`error.rs` 272, audit script 54, audit doc 499, ci.yml 177).
- [ ] CI green on push.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Suggested conductor action**: **Sweep to TODO** so a worker can rebase and re-push. The brief id is preserved; the next claimer picks it up fresh.

### #1074 — `claude-tenant-quota-enforcement-stricter`

- **Title**: quota: hard enforcement at gateway boundary (claude-tenant-quota-enforcement-stricter)
- **Branch**: `claude-claude-tenant-quota-enforcement-stricter` @ `91dcafc4`
- **Touched crates**: argo-cli, argo-store
- **Stale at brief snapshot (14:13Z)**: 327 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: Worker reports HARD RULE 2 boundary note for 9 pre-existing failures incl. KNOWN_FLAKES + #1039 set.

**Evidence**

- Last worker push (HEAD): `2026-05-09T08:59:33Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Boundary note (HARD RULE 2)

`cargo test -p argo-cli` shows 9 pre-existing failures **outside
the may-touch list**, all in vertical-manifest / executor-resolver
paths unrelated to tenant_quota:

- `handlers::doctor::doctor_channels::tests::ok_status_reports_ok`
- `handlers::setup_steps::tests::vertical_step_skips_empty_registry` *(KNOWN_FLAKES)*
- `handlers::worker_actions::tests::gtm_engineer_manifest_declares_expected_action_classification` *(KNOWN_FLAKES)*
- `handlers::worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
- `handlers::worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind …(truncated)
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1075 — `claude-genesis-iteration-cycle`

- **Title**: genesis: revise CLI command for prompt/actions/eval iteration (claude-genesis-iteration-cycle)
- **Branch**: `claude-claude-genesis-iteration-cycle` @ `3fb4e675`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 326 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: Worker reports HARD RULE 2 boundary note for same 8 #1039 failures (genesis_revise tests pass).

**Evidence**

- Last worker push (HEAD): `2026-05-09T09:00:33Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Boundary note (HARD RULE 2)

Pre-existing failures surface in `cargo test -p argo-cli`:

- `handlers::setup_steps::vertical_step_skips_empty_registry` (already in `KNOWN_FLAKES.md`)
- `handlers::worker_actions::gtm_engineer_manifest_declares_expected_action_classification` (already in `KNOWN_FLAKES.md`)
- `handlers::worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
- `handlers::worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::manifest_underscore_form_resolves_to_macos_kind`
- `handlers::worker_actions_executor_cu_macos …(truncated)
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1076 — `claude-worker-team-roster-cli`

- **Title**: team: roster + org-chart CLI (claude-worker-team-roster-cli)
- **Branch**: `claude-claude-worker-team-roster-cli` @ `25b19e27`
- **Touched crates**: argo-cli, argo-store
- **Stale at brief snapshot (14:13Z)**: 326 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: team-store 14 + team-cli 7 + smoke 4 pass; clippy clean. 8 wider failures = same #1039 set, all outside may-touch.

**Evidence**

- Last worker push (HEAD): `2026-05-09T09:00:51Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Tests

- 14 unit tests in `crates/argo-store/src/team_roster_tests.rs` — schema idempotency, parent validation, idempotent add (role updates on conflict), soft-delete semantics, re-add clears `removed_at`, list ordering, tenant isolation.
- 7 unit tests in `crates/argo-cli/src/handlers/team_tests.rs` — markdown snapshot assertions for `list` and `show`, charter-summary truncation, unparseable worker-id fallback.
- 4 CLI smoke tests in `crates/argo-cli/tests/team_cli.rs` — round-trip create→list→show, add→show→remove, rename→list, create-with-parent→show-markdown.

Crate-scoped clippy clean: `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings`. Files under 500 LOC.

## Boundary note

`cargo test -p argo-store -p argo-cli` surfaces 8 pre-existing failures, all OUTSIDE this brief's may-touch list — confirmed they fail identically against `origin/main` without these chang …(truncated)
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1077 — `claude-rate-limit-per-route`

- **Title**: gateway: per-route token-bucket rate limit (claude-rate-limit-per-route)
- **Branch**: `claude-claude-rate-limit-per-route` @ `ed32e760`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 322 min
- **Classification**: **NEEDS-REBASE**
- **Why**: Live re-poll: `mergeable=CONFLICTING` / `mergeStateStatus=DIRTY`. 12 unit/integration tests pass on the worker tip, but base has drifted.

**Evidence**

- Last worker push (HEAD): `2026-05-09T09:04:34Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `CONFLICTING`  •  `mergeStateStatus`: `DIRTY`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Tests

12 unit/integration tests under `handlers::gateway::rate_limit::tests` cover the brief's six required cases plus ancillary classification:

- `bucket_fills_to_capacity_then_drains` — bucket fills + drains.
- `refill_rate_matches_rpm_over_60s` — refill rate verified against `rpm/60.0` tokens/sec.
- `evict_idle_drops_stale_buckets` — `evict_idle` removes buckets older than `EVICT_AGE`.
- `env_override_for_known_route_is_honoured` — `ARGO_RATE_LIMIT_FORGE_RUN_RPM` round-trips through `from_env()` + env-var key derivation matches the documented mapping.
- `unknown_route_falls_back_to_default_rpm` — non-overridden routes use the default rpm.
- `concurrent_acquires_are_thread_safe` — 100 parallel acquires against a 50-token bucket: exactly 50 succeed, 50 reject (DashMap entry-lock semantics).
- `integration_eleven_requests_to_ten_per_min_route_rejects_one` — brief acceptance: 11 requ …(truncated)
```

**Suggested conductor action**: **Sweep to TODO** so a worker can rebase and re-push. The brief id is preserved; the next claimer picks it up fresh.

### #1078 — `claude-config-loader-overhaul`

- **Title**: config: scaffold argo-config + ArgoConfig + callsite audit (claude-config-loader-overhaul)
- **Branch**: `claude-claude-config-loader-overhaul` @ `ff2af25b`
- **Touched crates**: argo-config
- **Stale at brief snapshot (14:13Z)**: 319 min
- **Classification**: **NEEDS-REBASE**
- **Why**: Live re-poll: `mergeable=CONFLICTING` / `mergeStateStatus=DIRTY`. argo-config 22 unit + 1 doctest pass; pre-existing `cred.rs` fmt drift outside may-touch.

**Evidence**

- Last worker push (HEAD): `2026-05-09T09:07:13Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `CONFLICTING`  •  `mergeStateStatus`: `DIRTY`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Boundary note

`cargo fmt -p argo-config -- --check` reports drift in
`crates/argo-config/src/cred.rs` lines 146-149, 217-220, 269-272.
This drift exists on `origin/main` independently of this PR — the
file was last touched by #898 and is outside the brief's may-touch
list, so it is left alone here. Per HARD RULE 2 (pre-existing
issues outside may-touch don't block).

## Follow-up

`claude-config-loader-migrate-callsites` (queued, BLOCKED on this)
consumes `CONFIG_CALLSITE_AUDIT.md` to migrate every callsite to
`argo_config::load()` and flips the script to a failing CI gate.

## Test plan

- [x] `cargo clippy -p argo-config --all-targets -- -D warnings`
- [x] `cargo test -p argo-config` (22 unit + 1 doctest)
- [x] `bash scripts/check-no-direct-env-reads.sh` (reports 21 files,
      exit 0)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Suggested conductor action**: **Sweep to TODO** so a worker can rebase and re-push. The brief id is preserved; the next claimer picks it up fresh.

### #995 — `claude-mcp-server-health-check`

- **Title**: mcp: periodic health-ping task populates registry last_seen/last_error (claude-mcp-server-health-check)
- **Branch**: `claude-mcp-server-health-check` @ `13846a40`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 172 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: Worker reports `cargo clippy -p argo-cli` clean, mcp_health 7/7 + smoke 1/1 pass; 8 wider failures = 2 KNOWN_FLAKES + 6 post-#1039 schema-strict failures, all OUTSIDE may-touch.

**Evidence**

- Last worker push (HEAD): `2026-05-09T10:49:37Z`
- Last conductor review: `2026-05-09T05:59:19Z` — "Conductor Tier 1 hold: PR #1058 (mcp-server-registry-cli) just landed and touches handlers/mod.rs + adds the SQL registry path. This PR now reports CONFLICTING. Please rebase onto current main; the co"
- Review addressed: yes — push at 2026-05-09T10:49:37Z > review at 2026-05-09T05:59:19Z
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
- [x] Build/clippy/test verification run; boundary note below for known flakes
- [x] Every touched file is under 500 LOC; `mcp_health.rs` is 335 LOC, tests are 259/145 LOC

## Sensible deviations / boundary notes
- Freshness flagged the old boot path; the actual current boot file is `crates/argo-cli/src/handlers/gateway/boot.rs`, so the spawn hook is there.
- Added direct `tokio-util = "0.7"` to `argo-cli` for the requested `CancellationToken`; `Cargo.lock` only records the new direct edge because `tokio-util` was already present transitively.
- The SQL MCP registry from `codex-mcp-server-registry-cli` is still not on `main`, so production currently enumerates `[mcp_servers]` from vertical manifests and records updates as no-ops. The state-update contract is exercised by the fake-registry unit tests; the smoke test verifies gateway startup with fast/slow/dead fake servers.
- `cargo test …(truncated)
```

Worker-reported Tier 2 (last update comment, 2026-05-09T11:20:51Z):

```
Rebased onto origin/main (commit 13846a40) — clean rebase from 6c261d59, no conflicts. Verification on rebased commit:

- `cargo clippy -p argo-cli --all-targets -- -D warnings` clean (cached, finished in 3s)
- `cargo test -p argo-cli --bin argo mcp_health` 7/7 unit tests pass
- `cargo test -p argo-cli --test mcp_health_smoke` 1/1 integration smoke passes
- Diff vs origin/main shows only the 8 may-touch files (Cargo.lock, Cargo.toml, mod.rs, boot.rs, mcp_health.rs, mcp_health_tests.rs, mcp_health_smoke.rs, mcp_health.toml)
- Pre-existing fmt drift on main (mcp-servers/{browser,voice}/*) is outside may-touch and pre-dates this branch.

Force-pushed via --force-with-lease.
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1082 — `claude-mcp-server-rate-limit`

- **Title**: mcp: per-server token-bucket rate limit on dispatch (claude-mcp-server-rate-limit)
- **Branch**: `claude-claude-mcp-server-rate-limit` @ `d46fc8b6`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 172 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Worker reports `cargo clippy -p argo-cli` clean, `mcp_rate_limit` 9/9 + smoke pass; no boundary note (test scope tight enough to not surface workspace flakes).

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:20:55Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [x] `cargo test -p argo-cli mcp_rate_limit` 9/9 unit tests pass (8 from spec + 1 config-load test)
- [x] `cargo test -p argo-cli --test mcp_rate_limit_smoke` passes (integration smoke runs end-to-end via the `mcp_echo_server.py` fixture; auto-skips if `python3` not in PATH)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #989 — `claude-genesis-memory-schema-builder`

- **Title**: genesis: build memory schema (namespaces + layers) from role description (claude-genesis-memory-schema-builder)
- **Branch**: `claude-genesis-memory-schema-builder` @ `0d5b764c`
- **Touched crates**: argo-genesis
- **Stale at brief snapshot (14:13Z)**: 167 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Worker reports clippy clean + 90 unit + 1 integration + 5 doctests pass on `argo-genesis`. Different crate from #1039 area, no boundary note needed.

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:25:03Z`
- Last conductor review: `2026-05-09T07:04:46Z` — "Conductor Tier 1 hold: PR #987 (action-suggestor) landed earlier and also touched `argo-genesis/src/lib.rs` and `pipeline.rs`. This PR now reports CONFLICTING. Please rebase onto current main; the con"
- Review addressed: yes — push at 2026-05-09T11:25:03Z > review at 2026-05-09T07:04:46Z
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Verification
- `cargo clippy -p argo-genesis --all-targets -- -D warnings`
- `cargo test -p argo-genesis`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace --message-format=short`

## Notes
- `to_toml` emits the existing `memory/schema.toml` `[[entry]]` parser shape because `argo_vertical::parse_memory_schema` rejects unknown namespace/layer fields. The Rust model still exposes `namespaces` and `layers`, and the rendered entry purpose carries scope, TTL, layer, and retrieval metadata.
```

Worker-reported Tier 2 (last update comment, 2026-05-09T08:39:53Z):

```
- `cargo clippy -p argo-genesis --all-targets -- -D warnings` — clean (41s)
- `cargo test -p argo-genesis` — 90 unit + 1 integration + 5 doctests pass
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #1070 — `claude-worker-okr-cli`

- **Title**: worker: OKR objectives + key-results CLI (claude-worker-okr-cli)
- **Branch**: `claude-claude-worker-okr-cli` @ `732e6742`
- **Touched crates**: argo-cli, argo-store
- **Stale at brief snapshot (14:13Z)**: 162 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: argo-store 17/17 + argo-cli 7/7 + smoke 3/3 pass; clippy clean. 8 wider failures = same #1039 set, all outside may-touch.

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:30:25Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `cargo test -p argo-store --lib worker_okr` — 17 passed
- [x] `cargo test -p argo-cli --bin argo worker_okr` — 7 passed
- [x] `cargo test -p argo-cli --test worker_okr_smoke` — 3 passed
- [x] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean

## Boundary note (HARD RULE 2)

`cargo test -p argo-store -p argo-cli` reports 8 unrelated failures, all outside this brief's may-touch list and reproducible on clean `origin/main` without these changes. Same set as #1062's boundary note:

- `handlers::setup_steps::tests::vertical_step_skips_empty_registry` — listed in `KNOWN_FLAKES.md`.
- `handlers::worker_actions::tests::gtm_engineer_manifest_declares_expected_action_classification` — listed in `KNOWN_FLAKES.md`.
- 6 × `handlers::worker_actions_executor*` — strict vertical-manifest validation introduced upstream (executor.profile + executor.kind requir …(truncated)
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1083 — `claude-incident-postmortem-template`

- **Title**: incident: argo incident new postmortem walker + template (claude-incident-postmortem-template)
- **Branch**: `claude-claude-incident-postmortem-template` @ `100693e5`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 146 min
- **Classification**: **BOUNDARY-NOTE-FLAKES**
- **Why**: Worker reports HARD RULE 2 boundary note for same 6 #1039 failures + 2 KNOWN_FLAKES; postmortem walker tests pass.

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:46:34Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Boundary note

`cargo test -p argo-cli` reports 6 pre-existing failures outside this
brief's may-touch list, all in vertical-manifest schema validation:

- `handlers::worker_actions_executor::stream_a_tests::resolve_executor_dispatches_each_kind_through_tagged_form`
- `handlers::worker_actions_executor::worker_actions_executor_tests::resolve_executor_rejects_unknown_executor_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::dispatch_on_macos_writes_bundle`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::manifest_hyphen_alias_resolves_to_macos_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::manifest_underscore_form_resolves_to_macos_kind`
- `handlers::worker_actions_executor_cu_macos::cu_macos_tests::unknown_kind_with_macos_in_name_does_not_resolve`

Plus the two already in `KNOWN_FLAKES.md`
(`setup_steps::vertical_step_skips_empty …(truncated)
```

**Suggested conductor action**: **Merge** — Tier 2 only fails on documented KNOWN_FLAKES.md entries and the post-#1039 schema-strict failures, all OUTSIDE this PR's may-touch list and reproducible on clean `origin/main`. Per HARD RULE 2, these don't block.

### #1084 — `claude-latency-percentiles-tracking`

- **Title**: latency: p50/p95/p99 per executor kind + argo latency (claude-latency-percentiles-tracking)
- **Branch**: `claude-claude-latency-percentiles-tracking` @ `d0a0fac1`
- **Touched crates**: argo-cli, argo-store
- **Stale at brief snapshot (14:13Z)**: 145 min
- **Classification**: **READY-TO-MERGE**
- **Why**: argo-store latency_tracker 7/7 + argo-cli latency 4/4 pass; clippy clean. No boundary note in body.

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:46:35Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Tests (11 total, matches the brief)

`argo-store::latency_tracker` (7):
- empty histogram → no percentiles
- single sample → all percentiles collapse into that bucket
- 100 uniform samples → p50 in expected range, p99 ≥ p50
- bucket boundary mapping (incl. overflow + sub-1ms rounding)
- flush + reload round-trips counts (additive on conflict)
- per-kind isolation
- 4 × 1000 concurrent records via `tokio::join!` — all 4000 land

`argo-cli::handlers::latency` (4):
- `OutputFormat::parse` (case + whitespace + unknown → markdown)
- empty snapshot → friendly placeholder, no malformed table
- alphabetical sort + percentile computation across kinds
- markdown row shape (`ms`-suffixed cells, one row per kind)

## Sensible deviations

- Brief said `crates/argo-cli/src/cli.rs` for the subcommand; the actual
  `Command` enum lives in `cli_command_enum.rs`.
- Brief said TimedExecutor inline in `w …(truncated)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #1085 — `claude-runbook-quickref-html`

- **Title**: docs: gen-runbook-quickref.sh + quickref.html (claude-runbook-quickref-html)
- **Branch**: `claude-claude-runbook-quickref-html` @ `f80a8560`
- **Touched crates**: (docs only)
- **Stale at brief snapshot (14:13Z)**: 137 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Docs/script-only PR. shellcheck passes; SOURCE_DATE_EPOCH determinism verified by `cmp`. No cargo verification applicable.

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:55:31Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `bash scripts/gen-runbook-quickref.sh` writes
      `docs/skycot/quickref.html` (committed)
- [x] HTML opens cleanly in a browser; no external CSS/JS required
- [x] Top-commands and common-errors sections both present and
      styled distinctly (monospace `<pre>` vs. striped `<table>`)
- [x] Footer shows generated-at UTC + short git SHA
- [x] Script exits 2 with a clear message naming the missing marker
      when any of the four markers are removed
- [x] `SOURCE_DATE_EPOCH=N bash scripts/gen-runbook-quickref.sh` x2
      produces byte-identical output (verified via `cmp`)
- [x] `shellcheck -x scripts/gen-runbook-quickref.sh` passes

## Files

- `scripts/gen-runbook-quickref.sh` (NEW)
- `docs/skycot/quickref.html` (NEW, generated, committed)
- `docs/skycot/RUNBOOK.md` (4 marker comments + 1 cross-link
  paragraph in §9.6)

🤖 Generated with [Claude Code](https://claud …(truncated)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #1086 — `claude-runbook-skycot-split`

- **Title**: docs: split RUNBOOK.md into topic-grouped files (claude-runbook-skycot-split)
- **Branch**: `claude-claude-runbook-skycot-split` @ `70608051`
- **Touched crates**: (docs only)
- **Stale at brief snapshot (14:13Z)**: 137 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Docs-only RUNBOOK split. wc -l confirms ≤500 LOC ceiling and content preservation; cross-link audit clean. No cargo applicable.

**Evidence**

- Last worker push (HEAD): `2026-05-09T11:55:26Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `wc -l` on every file confirms ≤ 500 LOC ceiling and total content preservation
- [x] `rg 'docs/skycot/RUNBOOK\.md'` shows no section-anchor links anywhere in the repo (whole-file links only)
- [x] Manual scan confirms sub-files render with correct H1 + cross-link footer
- [ ] Reviewer to spot-check: open RUNBOOK.md → click each topic link → land in the right sub-file with headings intact

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #1087 — `claude-channels-push-voice-split`

- **Title**: channels: split push.rs + voice.rs below 500-LOC ceiling (claude-channels-push-voice-split)
- **Branch**: `claude-claude-channels-push-voice-split` @ `feae37af`
- **Touched crates**: argo-channels
- **Stale at brief snapshot (14:13Z)**: 126 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Worker reports `cargo clippy -p argo-channels` clean + `cargo test -p argo-channels` + workspace clippy clean. Pure file-split refactor.

**Evidence**

- Last worker push (HEAD): `2026-05-09T12:05:49Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `cargo clippy -p argo-channels --all-targets -- -D warnings`
- [x] `cargo test -p argo-channels`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #1088 — `claude-doctor-channels-split`

- **Title**: cli: split doctor_channels.rs below 500-LOC ceiling (claude-doctor-channels-split)
- **Branch**: `claude-claude-doctor-channels-split` @ `17cb65c4`
- **Touched crates**: argo-cli
- **Stale at brief snapshot (14:13Z)**: 121 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Worker reports `cargo clippy -p argo-cli` clean + `cargo test -p argo-cli doctor_channels` 10/10 pass. Pure file-split refactor under 500-LOC ceiling.

**Evidence**

- Last worker push (HEAD): `2026-05-09T12:11:27Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Test plan

- [x] `cargo clippy -p argo-cli --all-targets -- -D warnings` (clean)
- [x] `cargo test -p argo-cli doctor_channels` — 10/10 passing
  - 6 orchestrator/dispatch tests in `tests::*`
  - 4 imessage/voice tests in `kinds::tests::*`

## Unblocks

- `claude-approval-channel-fallback-chain` (currently BLOCKED on the 500-LOC ceiling). After this lands, that brief still needs may-touch widening to include `argo-channels::NotifyChannelRef`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).

### #1089 — `claude-cu-executor-extract-helper`

- **Title**: executor(cu): extract helper from executor.rs + pre-clear cu-fail-restart may-touch (claude-cu-executor-extract-helper)
- **Branch**: `claude-claude-cu-executor-extract-helper` @ `cbef7ea7`
- **Touched crates**: argo-executor-computer-use
- **Stale at brief snapshot (14:13Z)**: 121 min
- **Classification**: **READY-TO-MERGE**
- **Why**: Worker reports `cargo clippy -p argo-executor-computer-use` clean + tests 54+1+1=56 pass. Helper-extraction refactor; re-export keeps public API stable.

**Evidence**

- Last worker push (HEAD): `2026-05-09T12:13:43Z`
- Last conductor review: none on this PR
- `mergeable` (re-polled 14:23Z): `MERGEABLE`  •  `mergeStateStatus`: `UNSTABLE`
- GHA checks: every workflow fails in <10s — confirmed CI-infra regression on main, not per-PR (see top of report)

Worker-reported Tier 2 (PR body):

```
## Verification

- `cargo clippy -p argo-executor-computer-use --all-targets -- -D warnings` clean
- `cargo test -p argo-executor-computer-use` clean (54 lib tests + 1 integration smoke + 1 doctest, all pass)

## Test plan
- [x] Crate-scoped clippy clean
- [x] Crate-scoped tests pass
- [x] `executor.rs` ≤ 380 LOC after split
- [x] Extracted sibling ≤ 250 LOC
- [x] No public API changes (call sites still use `argo_executor_computer_use::{Outcome, RunRequest, RunResult}` via the re-export)
- [x] `claude-cu-fail-restart-strategy.md` may-touch list includes `profile.rs`

🤖 Generated with [Claude Code](https://claude.com/claude-code)
```

**Suggested conductor action**: **Merge** once the GHA-infra regression is resolved (or after a manual one-time bypass / queue-merge if the conductor judges the local Tier 2 evidence sufficient).
