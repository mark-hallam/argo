# Mission load-bearing wire — Dashboard planning-visibility additions

**Owner**: claude. **Depends on**: none.
**Authored against**: origin/main `36a5904d` on `2026-05-13`.

> SOURCE: operator-pasted commit-message spec 2026-05-13. Four
> orthogonal improvements to `tools/mission-dashboard/refresh.py`
> + the embedded HTML template. Pure operator-tooling — generator
> script for the operator's mission dashboard. Not part of the
> shipped `argo` binary.

## Scope

Four orthogonal improvements (each can land independently; ship in
one commit unless one fails review):

### 1. Track ↔ Mission-claim coverage cross-link

`docs/build-plan/PRODUCT_ROADMAP.md` has a "closes" column mapping
each Track (E, J, F, G, etc.) to mission claims (e.g. "E closes
claims 1, 2, 3, 6"). Today `refresh.py` parses Track cards from the
roadmap but doesn't surface the closes-mapping on the Mission-claim
side.

- Each Mission-claim row in the rendered dashboard gains an inline
  "tracks: E, J" badge listing which Tracks declare they close it.
- Each Track card's existing `closes` field gets explicit
  `closes:` prefix in the rendered card so the relationship is
  symmetric and skim-readable.

### 2. Conductor-advisory tracker panel

There are 4 `docs/build-plan/conductor-advisory-*.md` files:
- `conductor-advisory-E0-E1.md`
- `conductor-advisory-E2-E5.md`
- `conductor-advisory-F-reconciliation.md`
- `conductor-advisory-G-J.md`

Each carries a STATUS: ACTIONED banner with the date. Parse them
and render a new panel listing each advisory's filename, status,
and date. Read-only (just file parse + render).

### 3. Archive-aware burn-up

`docs/build-plan/BUILD_STATUS_ARCHIVE_2026Q2.md` (created
2026-05-13) holds 463 rows pruned from the live ledger. The
existing burn-up green line previously included all MERGED rows;
post-archive it cliffs at the pruning point because the script
only reads the live ledger.

Fix: read the archive file's MERGED count once at script-load,
treat it as a constant baseline added to the live burn-up curve.
Curve semantics stay continuous: "total briefs merged ever".

### 4. Status-weight completeness

Two small correctness fixes in the ledger-parsing logic:

a) `LEDGER_COMPLETION_WEIGHT` currently has entries for `MERGED`,
   `TODO`, `CLAIMED`, `IN_REVIEW`, `BLOCKED`, `DEFERRED-v1.x`
   (line ~183). Add `DEFERRED-operator` with `weight=None` (same
   semantics as `DEFERRED-v1.x` — excluded from denominator). The
   `claude-approval-mobile-deep-link-improvements` row uses the
   `DEFERRED-v1.x` flavour but other operator-deferred rows may
   adopt `DEFERRED-operator` over time; both should round-trip.

b) Phantom sub-divider rows starting with `▼` (e.g. `| **▼ MVP
   v1.0** | — | — | — | — |` inside Phase C) are currently being
   parsed by the row-iterator as briefs with status `'—'` (string),
   inflating denominators. Add a filter that skips any row whose
   first cell starts with `▼` after stripping markdown emphasis.

## Files you may touch

- `tools/mission-dashboard/refresh.py` (currently 2772 LOC — adding
  ~150 LOC; will go over 500-LOC ceiling but this is operator-
  tooling, not shipped code, so the ceiling doesn't apply. Verify
  by checking the file ISN'T under `crates/`.)
- `tools/mission-dashboard/dashboard.html` if regenerated; the
  script auto-writes it so worker just needs to run `python
  refresh.py` once after editing and commit the regenerated HTML.

## Files you must NOT touch

- `crates/**` — no Rust code change required
- `verticals/**` — no vertical template change
- `docs/build-plan/PRODUCT_ROADMAP.md` or any of the
  conductor-advisory files — read-only inputs
- `BUILD_STATUS.md` / `BUILD_STATUS_ARCHIVE_2026Q2.md` — read-only

## Acceptance criteria

- [ ] Mission-claim rows render "tracks: …" inline
- [ ] Track cards prefix existing closes field with "closes:"
- [ ] New advisory panel renders 4 advisories with status + date
- [ ] Burn-up curve no longer cliffs at the archive boundary
- [ ] `LEDGER_COMPLETION_WEIGHT` includes `DEFERRED-operator`
- [ ] Rows starting with `▼` are filtered from the iterator
- [ ] `python tools/mission-dashboard/refresh.py` runs clean
- [ ] Regenerated `dashboard.html` committed
- [ ] BUILD_STATUS.md row updated

## Branch + PR

- Branch: `claude-mission-dashboard-planning-visibility`
- PR title: `tools/mission-dashboard: planning-visibility additions (claim/track coverage, advisory tracker, archive-aware burn-up, status-weight completeness)`

## Notes

- This is operator-tooling, not product code. The 500-LOC ceiling
  rule applies to `crates/`, not `tools/`. Don't split the file.
- The commit message (use verbatim) is in the operator-paste of
  2026-05-13 that authored this brief. PR title above is the
  short form; PR body should expand to the 4-point list.
- Test by running `python refresh.py` locally and opening the
  generated `dashboard.html` in a browser. Verify each of the
  4 acceptance items by visual inspection. No automated tests
  exist for this script today; don't author them in this brief.
