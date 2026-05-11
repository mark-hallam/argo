# Stream J6 — Gateway team list endpoint (`GET /api/v1/team`)

**Owner**: claude. **Depends on**: J5 MERGED (workers exist); E3-c MERGED (the team-headline backend generator).
**Authored against**: origin/main `7e5b75e7` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J6-gateway-team-list-endpoint`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 6 of 18 — unblocks frontend E3-a from mock fallback.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| RBAC | All workers in tenant; no per-role filtering | Per-role visibility (delegators see workers they own; operators see all in tenant) | `mission-enterprise-team-rbac-scoping` (named in E3-a) |
| Pagination | Returns all workers ≤ 200; pagination for > 200 | Server-paginated at 50 per page when workspace > 50 workers | `mission-enterprise-team-pagination` (already named) |
| Org-hierarchy fields | `department` + `team` returned when present (nullable) | Required for enterprise tenants; surfaced in the API contract | `mission-enterprise-org-hierarchy` (Track I) |
| Audit | Each list query audit-logged | Same | n/a |
| Sort | Server returns workers sorted by status priority (waiting → working → idle → paused → blocked) then last-active-at desc | Same | n/a |
| Cache | None — recomputed per request | 60s server-side cache per tenant with invalidation on action commit | `mission-enterprise-team-headline-cache` (named in E3-c) |

v1 is acceptable: tenant-scoped list with E3-c's
headline generator producing per-worker bullets.
RBAC scoping deferred to the hardening track.

## Why

E3-a's frontend brief calls `GET /api/v1/team`. Until
this endpoint exists, E3-a falls back to mock data.
This brief unblocks production team-page rendering.

## Scope

- 1 JSON endpoint:
  - `GET /api/v1/team` →
    `{ workers: WorkerSummary[] }`.
- Tenant-scoped from Principal.
- For each worker: call
  `argo_team_headline::generate_team_headline`
  (from E3-c MERGED) to fill the `headline` field.
- Sort by status priority + last_active_at desc.
- Audit row written per call.
- 200-worker cap in v1 (returns ≤ 200; pagination
  brief is the v1.5 follow-up).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots:
  `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  (narrowly add 1 arm).

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_team_list.rs`
  — NEW. The endpoint handler. ≤ 300 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_team_list_tests.rs`
  — NEW. ≥ 10 unit tests. ≤ 400 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1-line `pub mod` addition.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly scoped: 1 arm for GET /api/v1/team.
  ≤ 230 LOC after.
- `docs/skycot/RUNBOOK.md` — narrowly scoped: add
  §"Team API".

## Files you must NOT touch

- `crates/argo-team-headline/**` — locked from
  E3-c.
- `crates/argo-identity/**` — read-only.
- Other Track-J handler files.

## Acceptance criteria

- [ ] `GET /api/v1/team` returns 200 + `{ workers:
      [...] }`.
- [ ] Each `WorkerSummary` JSON includes:
      ```json
      {
        "id": "...",
        "name": "...",
        "avatar_url": null,
        "role": { "tagline": "...", "vertical": "..." },
        "status": "working" | "waiting" | "idle" | "blocked" | "paused",
        "department": { "id": "...", "name": "..." } | null,
        "team": { "id": "...", "name": "..." } | null,
        "headline": {
          "bullets": ["8 replies sent", "$340 in pending offers"],
          "tomorrow": "morning routine at 06:30" | null
        },
        "last_active_at": "<ISO>" | null,
        "pending_approval_count": 0
      }
      ```
- [ ] Workers sorted: waiting > working > idle >
      paused > blocked; tie-break by last_active_at
      desc.
- [ ] Cap at 200 workers in v1; if tenant exceeds,
      truncate + add an `x-total-count` response
      header.
- [ ] Bearer auth required (401 otherwise).
- [ ] Tenant scoping: cross-tenant query returns
      403.
- [ ] Audit row written per list query (event_type:
      `team.list.queried`).
- [ ] Empty tenant: returns
      `{ workers: [] }` with 200.
- [ ] 10 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J6-gateway-team-list-endpoint`
- PR title: `gateway: GET /api/v1/team endpoint (J6)`

## Notes for the implementer

- **Implementation pattern:**
  ```rust
  pub async fn handle_team_list(
      principal: Principal,
      identity: &IdentityStore,
      audit: &mut AuditEmitter,
      headline_gen: &dyn TeamHeadlineGenerator,
  ) -> Result<Json<TeamList>> {
      let workers = identity.list_workers(principal.tenant_id, 200)?;
      let mut summaries = Vec::with_capacity(workers.len());
      for w in workers {
          let headline = headline_gen.generate(w.id, w.vertical_id, principal.tenant_id)?;
          summaries.push(WorkerSummary {
              id: w.id,
              name: w.name,
              // ... etc
              headline: Some(headline),
          });
      }
      summaries.sort_by(sort_key);
      audit.emit("team.list.queried", json!({ "actor": principal.id, "count": summaries.len() }))?;
      Ok(Json(TeamList { workers: summaries }))
  }
  ```

- **Sort key:**
  ```rust
  fn sort_key(a: &WorkerSummary, b: &WorkerSummary) -> Ordering {
      let priority = |s: &Status| match s {
          Status::Waiting => 0,
          Status::Working => 1,
          Status::Idle => 2,
          Status::Paused => 3,
          Status::Blocked => 4,
      };
      priority(&a.status).cmp(&priority(&b.status))
          .then(b.last_active_at.cmp(&a.last_active_at))
  }
  ```

- **`pending_approval_count`** — query the approval
  state machine for pending count per worker.
  Cached / batch-queried per request to avoid N+1.

- **PII handling.** Worker names, role taglines,
  and department/team names are NOT PII by default.
  No redaction needed at this level.

- **Audit content.** Just the count + actor + tenant
  — not the worker list itself (that would be
  noisy).

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-team-rbac-scoping` (already
  named).
- `mission-enterprise-team-pagination` (already
  named).
- `mission-enterprise-team-headline-cache` (already
  named in E3-c).

Track I (org hierarchy):
- `mission-enterprise-org-hierarchy` — required
  org binding.
