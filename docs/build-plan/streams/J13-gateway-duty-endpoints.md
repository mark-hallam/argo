# Stream J13 — Gateway duty endpoints (classify / commit / preview / toggle / delete)

**Owner**: claude. **Depends on**: J7 MERGED; J4 MERGED (Genesis-lite for classification); existing duty scheduler (`claude-worker-duty-scheduler` MERGED).
**Authored against**: origin/main `32b7c120` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh J13-gateway-duty-endpoints`.
**Tier tag**: PRODUCT_ROADMAP.md Track J brief 13 of 18 — unblocks frontend E4-h AddResponsibilityDialog + DutiesCard mutations.

## v1 posture vs enterprise target

| Concern | v1 implementation | Architectural target | Closes via |
|---|---|---|---|
| Conflict detection scope | Worker-level cron overlaps | Cross-worker / cross-team scheduling conflicts | `mission-enterprise-duty-cross-worker-conflicts` (named in E4-h) |
| Cron semantic preview | Client + server both parse; server preview includes next-fire timestamp | Same | n/a |
| Multi-approval | Single delegator commits | Department-level approval for high-stakes duties | `mission-enterprise-duty-multi-approval` (named in E4-h) |
| Cost projection | None | Per-duty cost estimate before commit | `mission-enterprise-duty-cost-projection` (named in E4-h) |
| Audit | One row per mutation | Full duty event-bundle with linked retrain/audit context | `mission-enterprise-duty-audit-bundle` |

v1 is acceptable: duty schedule + entry-action are
existing primitives; this brief adds HTTP wrappers
+ Genesis-driven classification.

## Why

E4-h's AddResponsibilityDialog calls
`POST /duties/classify` and `POST /duties/commit`.
DutiesCard also needs toggle + delete endpoints
plus the GET that returns the current list (J8
already includes duties in its bundled overview;
this brief adds the individual CRUD).

## Scope

5 JSON endpoints under
`/api/v1/workers/<id>/duties/`:

- `POST .../classify` body `{ input, cron_hint? }`
  → `{ duty_proposal, sample_outputs[],
  conflicts[] }`.
- `POST .../commit` body `{ duty_proposal,
  accepted_conflict_resolution? }`
  → `201 { duty_id, scheduled_first_fire }`.
- `POST .../preview` body `{ duty_id, sample_size }`
  → `{ samples[] }` (side-effect-free re-run).
- `PATCH /:duty_id` body `{ enabled?, schedule?,
  label? }` → `200 { duty }`.
- `DELETE /:duty_id` → `204`.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `routes_dispatch_post.rs` +
  `routes_dispatch_patch.rs` + `routes_dispatch_delete.rs`.

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_duties.rs`
  — NEW. ≤ 480 LOC.
- `crates/argo-cli/src/handlers/gateway/routes_duties_tests.rs`
  — NEW. ≥ 12 tests. ≤ 450 LOC.
- `crates/argo-cli/src/handlers/gateway/mod.rs` —
  narrowly: 1 line.
- 3 dispatch files: 1 arm each (POST x3, PATCH,
  DELETE).

## Files you must NOT touch

- `crates/argo-runtime/src/duty_scheduler.rs` —
  read-only (already supports one-shot from
  E2-i).
- Other Track-J handler files.

## Acceptance criteria

- [ ] `POST /duties/classify`:
      - Calls Genesis-lite to classify the input
        into a `Duty` shape.
      - Validates `cron_hint` if provided
        (returns 400 on malformed cron).
      - Returns `duty_proposal: Duty`,
        `sample_outputs: [...]` (3 hardcoded
        synthesized samples per duty kind),
        `conflicts: ConflictRecord[]` for any
        existing duty with cron overlap.
- [ ] `POST /duties/commit`:
      - Validates the proposal (cron parses;
        label ≥ 4 chars).
      - Resolves conflicts per
        `accepted_conflict_resolution: "replace" |
        "keep_both" | "cancel"`.
      - Inserts the duty row; computes
        `scheduled_first_fire` via duty scheduler.
      - Audit row + 201.
- [ ] `POST /duties/preview`:
      - Runs the duty's entry action against
        `sample_size` recent contexts (1-3;
        default 3).
      - Side-effect-free (trajectory-replay
        sandbox).
      - Returns array of `{ inbound_context,
        would_send }` samples.
- [ ] `PATCH /:duty_id`:
      - Updates the duty's enabled / schedule /
        label.
      - Audit row.
- [ ] `DELETE /:duty_id`: soft-deletes the duty +
      audit row.
- [ ] Cross-tenant or cross-worker → 404.
- [ ] Archived worker → 410.
- [ ] Bearer auth required.
- [ ] 12 unit tests pass.
- [ ] `cargo build -p argo-cli` clean.
- [ ] `cargo clippy -p argo-cli --all-targets --
      -D warnings` clean.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-J13-gateway-duty-endpoints`
- PR title: `gateway: duty CRUD + classify/preview endpoints (J13)`

## Notes for the implementer

- **Cron parser.** Server-side cron parser shared
  with the client side (E2-f shipped a client
  parser; the server uses `cron` Rust crate or
  similar standard impl). Reject patterns not in
  the 5-field form.

- **Conflict detection.** For each existing duty
  on the worker, expand its cron to next-N-fires
  (N=10) and check if any overlap the proposed
  cron's next-N-fires within 60s. Cap N at 10
  to bound compute.

- **Sample outputs.** Server-side hardcoded per
  classification (e.g. for "summarize end-of-day"
  duty: 3 sample summaries). v1 doesn't actually
  re-run Genesis to generate samples; uses
  fixtures keyed by classification.

- **Cron-string-to-next-fire computation.** Use
  the workspace timezone from the worker's
  workspace row. Cron `0 6 * * *` in tz
  `Pacific/Auckland` means "06:00 NZST every
  day".

- **DELETE semantics.** Soft-delete (sets
  `deleted_at`). Duty scheduler queries filter
  `WHERE deleted_at IS NULL`. Operator can
  recover via restore (not in v1 scope).

- **Duty `label` validation.** ≥ 4 chars + ≤ 100
  chars. Strip leading/trailing whitespace.

## Hardening briefs implied (Track H)

NOT blocking this PR:

- `mission-enterprise-duty-cross-worker-conflicts`
  (named).
- `mission-enterprise-duty-multi-approval`
  (named).
- `mission-enterprise-duty-cost-projection`
  (named).
- `mission-enterprise-duty-audit-bundle` — full
  duty event-bundle.
