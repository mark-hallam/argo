# Stream J11 — Gateway worker settings endpoints (channels / quiet / budget / autonomy / danger zone)

**Owner**: claude. **Depends on**: J7 MERGED.
**Authored against**: origin/main `f5ac001a` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J11-gateway-worker-settings-endpoints`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 11 of 18 — unblocks frontend E4-f, plus pause/resume from E3-b.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Budget caps | Per-worker daily + monthly | Hierarchical: tenant → department → team → worker; tightest wins | `mission-enterprise-budget-hierarchical-caps` (named in E4-f) |
| Autonomy granularity | 3 levels (conservative/balanced/aggressive) | Per-action-type policy matrix | `mission-enterprise-per-action-autonomy-policy` (named in E4-f) |
| Channel routing | Per-worker multi-select + primary | Per-action-type routing rules | `mission-enterprise-channel-routing-policy` (already named) |
| Quiet hours | Single range per worker | Per-channel-kind ranges | `mission-enterprise-per-channel-quiet-hours` (named in E4-f) |
| Delete ACL | Operator role required server-side | Tenant-policy-defined ACL chain (operator + compliance-officer dual-approval for production) | `mission-enterprise-worker-delete-acl` (named in E4-f) |
| Archive recovery | 30-day window before hard-delete | Same | n/a |
| Manifest versioning | Operator can view raw TOML; v1 doesn't support edit-then-save-as-new-version yet | Versioned manifest with rollback | `mission-enterprise-role-manifest-versioning` (named in E4-f) |

v1 is acceptable: settings mutations all flow
through the existing approval-action audit
infrastructure; danger-zone ACL gating is the
server's responsibility.

## Why

E4-f's Settings tab calls multiple endpoints under
`/api/v1/workers/<id>/` for its 6 sections.
E3-b's pause/resume context menu uses the same
endpoints. Until they exist, both fall back to
mock data + mutations. This brief lands all of
them in one batch.

## Scope

8 JSON endpoints, all under
`/api/v1/workers/<id>/`:

- `GET /settings` → all settings bundled.
- `PATCH /channels` body `{ channels: string[],
  primary: string }`.
- `PATCH /quiet-hours` body `{ start: "22:00",
  end: "07:00", tz: "Pacific/Auckland" }`.
- `PATCH /budget` body `{ daily_usd: number,
  monthly_usd: number }`.
- `PATCH /autonomy` body
  `{ level: "conservative" | "balanced" | "aggressive" }`.
- `POST /pause` (no body).
- `POST /resume` (no body).
- `POST /archive` (no body) → 30-day recovery.
- `DELETE /` (operator-only with type-name-to-
  confirm header).

(That's 9 endpoints; close enough to "8" in the
header — finer-grained for fewer schema changes.)

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: dispatch files for GET/POST/PATCH/DELETE.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_worker_settings.rs`
  — NEW. All 9 handlers. ≤ 500 LOC (cap).
- `crates/argo-cli/src/handlers/gateway/routes_worker_settings_tests.rs`
  — NEW. ≥ 18 tests (one per endpoint + ACL + edge
  cases). ≤ 500 LOC (cap).
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_get.rs`
  — narrowly: 1 arm (GET /settings).
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`
  — narrowly: 3 arms (pause / resume / archive).
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_patch.rs`
  (NEW if absent) — 4 arms (channels / quiet / budget
  / autonomy). ≤ 250 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_delete.rs`
  — narrowly: 1 arm.

## Files you must NOT touch

- `crates/argo-identity/**` — read-only; consumed.
- Other Track-J handler files.
- The approval state machine.

## Acceptance criteria

- [ ] All 9 endpoints accept Bearer auth (401
      otherwise).
- [ ] Tenant scoping: cross-tenant → 403.
- [ ] Unknown worker id → 404.
- [ ] Archived worker (except `POST /resume` after
      recover): 410.
- [ ] `GET /settings` returns:
      ```json
      {
        "channels": { "primary": "slack", "all": ["slack", "sms"] },
        "quiet_hours": { "start": "22:00", "end": "07:00", "tz": "..." },
        "budget": { "daily_usd": 5, "monthly_usd": 100, "used_today_usd": 0.43, "used_month_usd": 12.50 },
        "autonomy": { "level": "balanced" },
        "advanced": {
          "namespace_id": "...",
          "worker_id_full": "...",
          "manifest_url": "/api/v1/workers/<id>/manifest"
        } | null
      }
      ```
      (`advanced` only visible to operators.)
- [ ] PATCH endpoints update the field +
      audit-log the change.
- [ ] PATCH /autonomy with `conservative` resets
      the trust-ratchet for the worker.
- [ ] POST /pause: changes status → "paused";
      audit row.
- [ ] POST /resume: changes status → "idle";
      audit row.
- [ ] POST /archive: sets archived_at NOW(); audit;
      worker no longer fires actions; recoverable
      for 30 days via POST /resume.
- [ ] DELETE: 
      - Requires `permissions.can_delete = true` on
        the Principal (operator-only).
      - Requires `X-Confirm-Worker-Name: <name>`
        header that matches the worker's name
        case-sensitively. Mismatch → 400.
      - Hard-deletes memory + skills assignments;
        audit row remains.
      - 204 on success.
- [ ] Budget exceeded enforcement: gateway emits a
      `worker.budget.exceeded` event on cap breach
      (separate brief wires the kill-switch; this
      brief just emits).
- [ ] 18 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file ≤ 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J11-gateway-worker-settings-endpoints`
- PR title: `gateway: worker settings + danger-zone endpoints (J11)`

## Notes for the implementer

- **Single combined file size cap.** ~500 LOC for 9
  handlers is tight but doable: most handlers are
  ~40 LOC. If the file exceeds 500 LOC, split into
  `routes_worker_settings_read.rs` (GET + PATCH)
  and `routes_worker_settings_danger.rs` (POST
  pause/resume/archive + DELETE). Don't fight the
  500-LOC rule.

- **Trust-ratchet reset on conservative.** When
  PATCH /autonomy receives `conservative`, the
  server clears the worker's per-action-type trust
  counters. The retrain journey doc specifies this
  behavior. v1 implementation: reset all counters;
  v1.5 per-action selectivity.

- **Archive vs delete distinction.** Archive sets
  `archived_at` and removes the worker from the
  team list. Memory + skills + duties stay
  intact; recovery via POST /resume re-activates.
  Delete is irreversible: tombstone the worker
  row, clear memory namespace, drop duty rows.
  Audit log keeps everything for compliance.

- **Type-name-to-confirm header.** Frontend's
  TypeNameToConfirm dialog sends:
  ```
  X-Confirm-Worker-Name: Mariah
  ```
  Server compares case-sensitively against
  `worker.name`. Mismatch → 400 with
  `{ error: { code: "confirmation_mismatch", message: "Worker name confirmation didn't match" } }`.

- **`can_delete` permission.** Resolved via J7's
  permission helper; only operator-role
  Principals get `true`. Delete by other roles →
  403.

- **Budget breach event.** On any approval-action
  commit, the post-commit handler checks daily +
  monthly used vs cap. On breach, emit
  `worker.budget.exceeded` event. This brief
  emits; the consumer (auto-pause? warn?) is a
  Track-H follow-up.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- All those named in E4-f (carried through).
