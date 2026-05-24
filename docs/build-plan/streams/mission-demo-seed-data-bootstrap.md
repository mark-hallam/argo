# Stream mission-demo-seed-data-bootstrap — seed REAL workspace data via argo demo seed CLI

**Owner**: claude.
**Depends on**: none (independent of the SPA-side fixes; the seeded data flows through the same gateway endpoints the SPA already consumes).
**Authored against**: origin/main `109214a70` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-data-bootstrap`.
🔴 **P0 DEMO-GATE OPERATOR-LOCKED — NO-THEATER REQUIREMENT.** Per the `🚨 NO THEATER IN THE DEMO` amendment in BUILD_STATUS.md header (operator decision 2026-05-24): the v1.0 demo MP4 must be recorded against a real workspace with real data and real agent behavior — NOT against `web/lib/sample-*.ts` mock fixtures shown via `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`. This brief implements the real-backend seed path.

## Scope

Add a real CLI subcommand `argo demo seed` that inserts the voiceover-aligned demo data into a real workspace via the real gateway primitives — same code paths a real customer signup would use, just with a richer initial state.

The CLI populates:
- 6 named workers (Mariah, Priya, Jamal, Nori + 2 more) — created via existing `WorkerStore::create` with real vertical pack assignments, real personas, real role/charter from the existing sample-personas data (which becomes the SEED SOURCE for the CLI, not the rendered output).
- 7 pending approval rows — created by triggering real agent turns against sandbox accounts (the agent runner is invoked, produces real `WorkerAction` rows, the auto-approval gate creates real `approval_request` rows). NOT hand-inserted into the approvals table.
- Mariah's "today's KPIs" (3 listings live / 11 buyer replies / 2 items flagged below floor) — emerges from real agent turns over the past N seed-days, audit_record rows reflecting each. The KPI strip on `/team` reads from `compute::activity_counts_for_worker` which already SUMs real audit rows.
- 1000+ audit_record rows — produced by real agent turns. The seeder runs a configurable number of turns per worker (default ~150 each) over a backdated time window so the audit feed has populated history.

The result: when the demo opens `/team`, the data shown is exactly what the gateway returns from real DB queries against a real workspace. When the operator clicks Approve on Slack, a real `decide_request` SQL UPDATE fires. When the audit feed loads, real `audit_record` rows render. The voiceover's claims (1,122 weekly actions, 72 hours saved) are derived from real data, not painted on.

This is a **substantive backend addition** — ~600-1000 LOC in `argo-cli` + new `crates/argo-demo-seed/` library crate. NOT a fixture file edit. NOT a TypeScript change.

## Parallelism

- Manager-worker allowed: yes
- Max children: 3
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: docs/build-plan/work-packages/mission-demo-seed-data-bootstrap.md (manager creates from PROTOCOL_MANAGER_PACKAGE_TEMPLATE.md)
- Shared hotspots: [`crates/argo-cli/src/cli.rs` (one new subcommand variant)]

## Files you may touch

- `NEW: crates/argo-demo-seed/**` — new library crate hosting the seed orchestration. Takes a `tenant_id` + a `seed_profile` (default = "v1-demo-canonical") and orchestrates worker creation + agent turn execution + Slack channel registration.
- `crates/argo-cli/src/cli.rs` — narrowly scoped: add `DemoCmd::Seed { ... }` variant.
- `NEW: crates/argo-cli/src/handlers/demo_seed.rs` — CLI handler invoking `argo-demo-seed::run_canonical_v1_seed(...)`.
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: add `mod demo_seed;` line.
- `Cargo.toml` — narrowly scoped: add workspace member `crates/argo-demo-seed`.
- `web/lib/sample-personas.ts` — read-only INPUT: the seed CLI reads the persona spine (name + role + traits + headline + vertical_id) from this file and inserts those as real worker rows. The file itself is unchanged.
- `docs/skycot/operator/demo-runbook.md` — add a pre-flight bullet: "Run `argo demo seed --tenant <demo-tenant-id>` before the recording session. Verify `/team` shows 6 workers via curl http://127.0.0.1:7400/api/v1/team and the count matches."
- `scripts/demo-dry-run.sh` — narrowly scoped: drop the SAMPLE_TEAM mock paths from the dry-run scene assertions. Scenes now verify real backend rows.

## Files you must NOT touch

- `web/lib/sample-personas.ts` for ANY edit (read-only input)
- `web/lib/sample-*.ts` other than to confirm they are NOT used at runtime when seeded data is present
- `crates/argo-store/src/migrations/**` — no new migration is in scope; the seed uses existing tables/columns
- Anything in `crates/argo-skill/` or `crates/argo-orchestrator/` other than call-site invocations through existing public APIs

## Anti-theater acceptance criteria

The PR must demonstrate that:
- [ ] After `argo demo seed --tenant <demo>`, `curl http://127.0.0.1:7400/api/v1/team` returns 6 real `Worker` rows with ULID worker IDs
- [ ] After seeding, `curl http://127.0.0.1:7400/api/v1/approvals` returns 7 real pending `approval_request` rows, each linked to a real `WorkerAction` row, each WorkerAction created by a real agent turn (audit-traceable to a `WorkflowStepCompleted` row)
- [ ] After seeding, `curl http://127.0.0.1:7400/api/v1/audit?limit=100` returns 100+ real `audit_record` rows, each with a real `worker_id` + real `tenant_id` + real `created_at` + a real `payload` from a real agent turn (not synthetic JSON)
- [ ] Mariah's `/workers/{id}/overview` page numbers (listings live, buyer replies, flagged-below-floor) derive from real SQL aggregations over real audit rows, NOT from a `sample-worker-overview.ts` fixture
- [ ] The dry-run script's all-8 scenes pass when `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` is UNSET — i.e. the SPA is talking to the real backend, no mock fallback
- [ ] PR body includes a 10-line transcript of `argo demo seed --tenant <id>` showing the agent turns running and audit rows being created
- [ ] PR body includes the vacuous-pass guard the evaluator's round-37 pattern requires: a negative test asserting that a DIFFERENT tenant sees ZERO rows after seeding (cross-tenant invisibility) AND the seeded tenant sees N>=6 (positive baseline)
- [ ] `cargo clippy -p argo-demo-seed -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-demo-seed` passes
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-demo-seed-data-bootstrap`
- PR title: `demo: argo demo seed CLI for real-backend v1.0 demo workspace (mission-demo-seed-data-bootstrap)`

## Notes for the implementer

- **This brief was REWRITTEN 2026-05-24 from Option A (mock-bypass fixtures) to Option B (real backend seed) per the operator's `🚨 NO THEATER IN THE DEMO` amendment in BUILD_STATUS header.** Do not revert to the mock approach.
- The agent turns the seeder triggers MUST hit real LLMs. Use a sandbox provider key configured in the seed profile. The seeder fails closed if no real provider is configured — don't fall back to canned responses.
- For Mariah's persona (marketplace seller), use a sandbox Facebook Marketplace API or a stub that emits realistic-looking action payloads but with `is_sandbox: true` in the metadata — the agent turns are real but the external side effects are bracketed.
- The 1000+ audit rows: spread across the past 14 days (configurable). Use the WorkerSessionRunner to actually drive each turn — don't just `INSERT INTO audit_record VALUES (...)`. That would be theater.
- Manager-worker mode allowed because the work spans:
  - (1) the new `argo-demo-seed` crate
  - (2) the CLI handler wiring
  - (3) the runbook + dry-run updates
- Coordinate with `fix-web-mock-bypass-env-var-rename` — once that merges and the bypass mode is fixed, demo recording USES this seed CLI + the bypass mode DEFAULT-OFF (or removed entirely, operator's call).
- The pattern is similar to how SaaS apps "seed a demo account on signup" — Stripe, Notion, Linear all do this. Look at how OSS projects like Cal.com or Plane seed their demo workspaces for prior art.
