# fix-skill-worker-overlay-blob-text-join-mismatch

**Priority**: P1 (production data-integrity bug — per-worker learned overlays are silently invisible to the runtime that's supposed to consume them)
**Owner**: claude
**Type**: Production fix — SQL JOIN type-affinity mismatch
**Size**: ~80-200 LOC (production SQL + 2 currently-failing tests un-ignored)
**Authored against**: origin/main `576e48f5c` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skill-worker-overlay-blob-text-join-mismatch`

## Why

`crates/argo-skill/src/store_worker_overlay_tests.rs` carries two
tests that have been **silently failing on `main` since PR #1844
(2026-05-19)**:

- `list_for_worker_returns_attached_active_overlays` →
  `assertion left==right failed: left=0 right=1`
- `list_for_worker_filters_other_workers` → same shape

Every argo-skill PR since 2026-05-19 has carried a boundary-note
("pre-existing flake confirmed on clean main, outside may-touch") —
PR #2110, PR #2122, PR #2143, PR #1869, PR #1855, others. Six
consecutive PRs is the signal that **this is no longer a flake; it
is a real production bug masked by the boundary-note exemption**.

**Root cause (confirmed via `sqlite3 :memory:` repro):** the
`skill` table stores `id BLOB PRIMARY KEY` (`v1.rs:107`), the
`worker_skill` table stores `skill_id TEXT NOT NULL`
(`migrations/101__worker_skill.sql:28`). The production write path
`per_worker_learning_cron::attach_worker_overlay`
(`crates/argo-skill/src/per_worker_learning_cron.rs:237`) binds
`.bind(skill_id.to_string())` (TEXT, dashed-UUID form like
`"2acf17b3-987c-4d97-8c7b-8a45ad6a0f12"`). The skill table holds
the same id as 16 raw bytes (`.bind(id.0.as_bytes().as_slice())`
at `store_write.rs:84`). The read path's JOIN
`s.id = ws.skill_id` at `store_lookups.rs:118-119` compares BLOB
to TEXT — SQLite type-affinity rules return 0 matches.

**Production impact:** every per-worker overlay skill the cron
promotes (mission claim 4 — "skills shared + learned") is invisible
to the worker on its next session. The learning loop is wired but
the read-back is silently broken. This is a direct hit on a load-
bearing mission-claim-4 surface.

Repro (run on clean `main`):

```bash
sqlite3 :memory: <<'EOF'
CREATE TABLE skill (id BLOB PRIMARY KEY, name TEXT);
CREATE TABLE worker_skill (worker_id TEXT, skill_id TEXT);
INSERT INTO skill (id, name) VALUES (X'2acf17b3987c4d978c7b8a45ad6a0f12', 'foo');
INSERT INTO worker_skill (worker_id, skill_id) VALUES ('w', '2acf17b3-987c-4d97-8c7b-8a45ad6a0f12');
SELECT s.name FROM worker_skill ws JOIN skill s ON s.id = ws.skill_id WHERE ws.worker_id='w';
EOF
```

Expected: `foo`. Actual: empty result.

## What

Pick the **safer** of two repair paths; both leave the system in a
correct state, the differences are migration cost vs API surface.

**Path A — fix the read path only (recommended, smaller blast
radius).** Change the `worker_skill ⨝ skill` JOIN in
`crates/argo-skill/src/store_lookups.rs:118-119` to coerce the
BLOB→hex on the skill side, or TEXT→BLOB on the worker_skill side,
so the comparison succeeds regardless of how callers happen to
have bound the column. Two viable shapes:

  - `WHERE hex(s.id) = REPLACE(ws.skill_id, '-', '') COLLATE NOCASE`
    (works if `worker_skill.skill_id` holds dashed-UUID text — the
    current production shape via `.to_string()`).
  - Or, cleaner: change the JOIN to bind through `SkillId`'s byte
    form when reading and compare `s.id = X'…'` via a parameter
    bind — i.e. let the caller resolve the UUID-to-bytes coercion
    rather than encode it in SQL.

  Path A keeps the existing `worker_skill` rows intact — no
  migration needed. It does mean every future read site has to use
  the same coercion idiom; centralize it in a helper.

**Path B — fix the schema (broader change, more durable).** Add
migration `133__worker_skill_skill_id_blob.sql` that renames the
existing `worker_skill` to `worker_skill_legacy_text_skill_id`,
creates a fresh `worker_skill` with `skill_id BLOB NOT NULL`, and
back-fills via `INSERT INTO worker_skill SELECT worker_id,
unhex(replace(skill_id,'-','')), proficiency, created_at FROM
worker_skill_legacy_text_skill_id` (SQLite has no `unhex` — use
`randomblob(16)` + binary stitching, or do the conversion in Rust
during a one-shot back-fill). Then update every production write
site (`per_worker_learning_cron.rs:237`,
`identity/store_skills.rs:48,254`, `scheduled_reeval_cron_tests
.rs:171`) to bind bytes not text. Path B is the "right" fix but
touches ~6 files and a migration; ship it only if Path A turns out
to need too many JOIN sites.

**Either path:** un-`#[ignore]`-equivalent the two failing tests.
They already exist as `#[tokio::test]` (no `#[ignore]`) at
`crates/argo-skill/src/store_worker_overlay_tests.rs:35` +
`:51` — they're FAILING, not ignored. After the fix, both must
pass without modification (the test bodies pin the
operator-observable contract).

**Also fix** (small, in scope): add a sibling test
`per_worker_learning_cron_attach_overlay_round_trips_through_list_for_worker`
in `crates/argo-skill/src/per_worker_learning_cron_tests.rs` that
calls `attach_worker_overlay` directly (the production write path)
and then `list_for_worker` and asserts the row is returned. This
pins the production wire-up that the unit tests in
`store_worker_overlay_tests.rs` only approximate.

## Acceptance

- `cargo test -p argo-skill --lib store_worker_overlay_tests` →
  **5/5 pass** (currently 3 pass + 2 fail).
- `cargo test -p argo-skill --lib per_worker_learning_cron` →
  existing tests pass + new round-trip test passes.
- `cargo clippy -p argo-skill --tests -- -D warnings` clean.
- Workspace clippy fast-fail unchanged (no new lints).
- PR body explicitly calls out which Path (A or B) was taken and
  why; includes the `sqlite3 :memory:` repro confirming the bug
  pre-fix and a parallel confirmation post-fix.
- The 6 carry-over boundary notes can be retired by future PRs;
  this brief's PR body should explicitly say "closes the
  store_worker_overlay flake boundary chain spanning PRs #1855,
  #1869, #2110, #2122, #2143 (and #2152)".

## May-touch

- `crates/argo-skill/src/store_lookups.rs` (Path A) OR
- `crates/argo-store/migrations/133__*.sql` +
  `crates/argo-store/src/migrations/vN.rs` (Path B; pin slot 133
  per the live migration registry — verify at claim time).
- `crates/argo-skill/src/per_worker_learning_cron.rs` (Path B).
- `crates/argo-identity/src/store_skills.rs` (Path B).
- `crates/argo-skill/src/scheduled_reeval_cron_tests.rs` (Path B
  only).
- `crates/argo-skill/src/per_worker_learning_cron_tests.rs` (add
  one sibling test, either path).
- `crates/argo-skill/src/store_worker_overlay_tests.rs` is the
  pin-test; do **not** modify the test bodies.

## Source

- 6 consecutive boundary notes carry-chain (PRs #1855, #1869,
  #2110, #2122, #2143, #2152).
- Bug confirmed live on origin/main `576e48f5c` via
  `cargo test -p argo-skill --lib store_worker_overlay_tests`
  (3 pass, 2 fail) and via standalone `sqlite3` repro.
- Mission claim 4 (skills shared + learned) — load-bearing.

Fast-mode candidate **only if Path A is chosen**. Path B should be
authored deliberately by a worker with migration-registry context.
