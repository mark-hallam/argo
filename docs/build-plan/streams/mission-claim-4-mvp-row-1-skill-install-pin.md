# mission-claim-4-mvp-row-1-skill-install-pin

**Owner**: unclaimed.
**Depends on**: nothing (row-1 substrate exists; this pins the byte-shape).
**Authored against**: origin/main `952e42591` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-claim-4-mvp-row-1-skill-install-pin`.

🟡 **P2 — Claim 4 MVP row 1 (skill install) substrate-thin → 🟢🟢 e2e pin.** Per evaluator round 31c (SHARED.md 2026-05-23T23:30Z), only Claim 4 MVP rows 1 + 3 remain 🟡 substrate-thin (rows 4/5/6/7 are 🟢🟢, row 2 is ⏳ in flight). This brief closes row 1 — the byte-shape pin for the skill install flow.

## Why this brief exists

Claim 4 MVP row 1 from `docs/skycot/evals/scheduled/2026-05-23/claim-dashboards.md`:

> Row 1: skill install installs a versioned skill into a tenant's library; subsequent runtime lookup returns it; cross-tenant install does not leak.

Today the install flow is wired (`argo skill install` CLI + gateway POST `/api/v1/workers/<id>/skills/install` + memory-store layer); existing tests cover individual SQL pieces. But the row is NOT pinned by an e2e regression test that:
- Drives the full install through `SkillStore::install_skill` (or the CLI handler)
- Asserts the installed skill appears in `list_skills_for_vertical` with version + lifecycle stamps preserved
- Asserts cross-tenant: tenant_b can't see tenant_a's installed skill (a tenant-scope refactor regression would flip this red)
- Asserts an audit row (`skill.installed`) is emitted with the install payload shape

This brief lands that regression test alongside the existing Claim 4 row pins (rounds 28/29/31b/31c).

## What this brief should do

Author `crates/argo-skill/tests/claim_4_mvp_row_1_skill_install_pin.rs`. 4 tokio tests:

1. **Install round-trip**: install a versioned skill (name=`marketplace-research`, version=`1.0.0`, vertical=`marketplace-seller`); assert `list_skills_for_vertical(vertical)` returns the row with name+version+lifecycle=Active.
2. **Install + re-install behavior**: re-installing the same name+version is idempotent OR returns a defined error; either contract is fine, but the test pins whichever it is so a silent flip would catch a regression.
3. **Cross-tenant isolation**: tenant_a installs → tenant_b's `list_skills_for_vertical(vertical)` returns ZERO rows; tenant_a sees its own.
4. **Audit emission**: install emits a `skill.installed` (or equivalent) row with payload carrying skill_id + name + version + vertical + installed_by + tenant_id; chain-verifies clean.

If the `skill.installed` audit kind doesn't exist yet (verify before authoring), file a sibling brief `mission-add-skill-installed-audit-kind` as a dependency and test #4 stays out of this brief.

## Files you may touch

- `crates/argo-skill/tests/claim_4_mvp_row_1_skill_install_pin.rs` (new test file, ~150-220 LOC).

## Files you must NOT touch

- Production install code — this is a pin, not a wire-up brief.
- Existing `claim_4_mvp_row_*` test files — separate row contracts.

## Acceptance criteria

- [ ] 4 tokio tests landed, all green on origin/main HEAD at claim time (or 3 tests + a sibling brief filed if the audit kind doesn't exist).
- [ ] Cargo test runs in <30s cold.
- [ ] Cross-tenant isolation asserted with both list-side AND raw-SQL GROUP BY tenant_id confirms (mirroring round 24's pattern).
- [ ] BUILD_STATUS.md row updated.

## Parallelism

- Manager-worker allowed: no.
- Max children: 1.

## LOC budget

~150-220 LOC (one test file). Below the 500 LOC per-file cap.
