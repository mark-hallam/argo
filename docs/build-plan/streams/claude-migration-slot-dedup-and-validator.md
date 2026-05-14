# claude-migration-slot-dedup-and-validator — Fix 11 argo-store migration slot collisions + add runner validation

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-migration-slot-dedup-and-validator`.

## Scope

🔴 **OPERATIONAL RISK.** Phase D audit found 11 slot-number collisions in `crates/argo-store/migrations/`. The sqlx migration runner may process duplicates silently in filesystem order. This brief: (a) re-slots the duplicates to free numbers ≥79; (b) adds runner-side validation that REJECTS duplicate slot numbers at startup.

**Collisions** (verified at HEAD):
- Slot 19: `19__anomaly.sql`, `19__worker_fs_root.sql`
- Slot 22: `22__roster.sql`, `22__tenant_archived.sql`
- Slot 23: `23__handoff_routing.sql`, `23__workflow_checkpoints.sql`
- Slot 24: 3-way (`operator_feedback`, `skill_quality_score`, `vertical_archived`)
- Slot 26: 3-way (`data_classification`, `spend_baseline`, `worker_budget_alert`)
- Slot 27: `27__handoff_auctions.sql`, `27__spend_anomaly.sql`
- Slot 28: 3-way (`recovery_cache`, `role_duties`, `tenant_quota`)
- Slot 30: `30__learned_plan_templates.sql`, `30__worker_charter.sql`
- Slot 45: 3-way (`cu_screenshot_index`, `worker_dependency`, `worker_level`)
- Slot 51: `51__audit_worker_deleted_at.sql`, `51__worker_skill.sql`
- Slot 75: `75__scim_provisioned_tombstones.sql`, `75__siem_dispatch_cursor.sql`

**CRITICAL CONSTRAINT**: Some of these migrations are likely already applied to production databases. **Re-slotting is risky** — sqlx tracks applied migrations by slot number, so renaming `19__worker_fs_root.sql` to `79__worker_fs_root.sql` makes the runner think it's a NEW migration to apply.

**Two implementation paths** — implementer chooses:

**Path A (preferred — non-destructive):**
- Leave existing collisions in place (they've already been applied to dev/prod databases via filesystem order).
- Add a "canonical_order" mapping in code that the runner consults BEFORE filesystem-order.
- Add runner-side detection that LOGS A LOUD WARNING for any future duplicate slots but doesn't reject (to preserve existing applied state).
- Update the sqlx tracker table to record the canonical (filename, slot) pair so reapplication never happens.

**Path B (destructive — only if Path A infeasible):**
- Re-slot duplicates to 79+ in canonical order.
- Add a one-time migration that updates the sqlx `_sqlx_migrations` tracking table to reflect the new slot numbers.
- Add runner-side validation that REJECTS duplicates at startup.

LOC: ~500.

## Files you may touch

- `crates/argo-store/src/migrations/registry.rs` (461 LOC near-limit) — add slot-collision detection + canonical-order mapping. May need split (see also `claude-store-migrations-registry-split`).
- `crates/argo-store/migrations/*.sql` — DO NOT rename in Path A; rename only in Path B.
- NEW `crates/argo-store/src/migrations/slot_validator.rs` (≤300 LOC) — runner-side validator.
- NEW `crates/argo-store/src/migrations/slot_validator_tests.rs` — unit tests covering: clean state, duplicate detection, applied-but-renamed scenario.
- `crates/argo-store/src/migrations/registry.rs` — narrowly: wire validator into runner startup. ≤30 modified lines.
- `docs/skycot/operator/migrations.md` — update with collision-history note + future authoring rule (slot reservation).
- NEW `scripts/check-migration-slots.sh` — pre-commit hook script that detects new collisions in PRs.

## Files you must NOT touch

- ANY production-applied schema (additive-only changes per CLAUDE.md invariant).
- argo-skill migrations (separate registry; not affected).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Phase D's 11 slot collisions handled (Path A or B — document the choice in PR description).
- [ ] Runner validates against duplicates at startup (LOUD WARN in Path A; REJECT in Path B).
- [ ] `cargo test -p argo-store migrations` passes including new slot_validator_tests.
- [ ] If Path B: dev-environment migration applies cleanly from scratch + applies cleanly from a stale prior state (re-slot-aware migration of sqlx tracker).
- [ ] Pre-commit hook script catches a synthetic duplicate slot PR.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-migration-slot-dedup-and-validator`
- PR title: `fix(store): resolve 11 migration slot collisions + runner validation (claude-migration-slot-dedup-and-validator)`

## Notes for the implementer

- Mission-claim alignment: production hardening (data integrity for the SQLite substrate; supports all 6 claims).
- **Coordinate with operator** if Path B chosen — the migration-tracker rewrite needs operator sign-off given prod blast radius.
- Phase D audit report: `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` §1.
- Drift-prevention: brief adds slot_validator + check-script. No new pub types in other crates.
