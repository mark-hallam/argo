# Round 41 — anti-theater verification of conductor tick-121 P1 brief `fix-skill-worker-overlay-blob-text-join-mismatch`

**Date**: 2026-05-23 (Melbourne local Sun May 24 04:27 AEST)
**Round**: 41
**Pillar/Claim**: Claim 4 (skills shared and learned — the per-worker learning loop the operator demanded in round 19 as "for claim 4 we got to achieve the learned skills")
**Verdict**: 🔴 **FAIL — P1 confirmed live on `origin/main`** (severity = real production data-integrity bug, not flake; brief is correctly scoped)
**Repo head verified against**: `bc78952a7 status: tick-122 — 5 replenishment briefs`

## Why this scenario this round

Conductor tick-121 (commit `14f44211d`) authored a NEW P1 brief
`fix-skill-worker-overlay-blob-text-join-mismatch` with a striking
claim: **a `sqlite3 :memory:` repro proves that
`per_worker_learning_cron::attach_worker_overlay` writes overlays
that the worker reader (`list_for_worker`) never picks up**, attributing
it to a BLOB vs TEXT type-affinity mismatch between `skill.id` and
`worker_skill.skill_id`. The brief calls it the root cause of a
"6-PR boundary-note carry-chain spanning #1855/#1869/#2110/#2122/#2143/#2152".

Accepting a swarm-internal repro at face value is exactly the kind
of theatre this scheduled-evaluator role exists to guard against.
The evaluator's job is to independently confirm or refute, since the
brief is conductor-authored (one voice in the loop) and any future
worker who claims it will rely on the diagnosis being correct.

## Setup — what I ran

1. **Schema audit**:
   - `grep -rn "CREATE TABLE.*worker_skill\|CREATE TABLE skill" crates/argo-store/src/migrations/`
   - `cat crates/argo-store/migrations/101__worker_skill.sql`
   - Read `crates/argo-store/src/migrations/v1.rs:107-119` (skill table)

2. **Code paths**:
   - `grep -rn "attach_worker_overlay\|fn attach_worker_overlay" crates/`
   - `grep -rn "JOIN.*skill\|FROM worker_skill" crates/ --include="*.rs"`
   - `grep -rn "list_for_worker" crates/`
   - Read `crates/argo-skill/src/per_worker_learning_cron.rs:228-243` (writer)
   - Read `crates/argo-skill/src/store_lookups.rs:104-130` (reader JOIN)
   - Read `crates/argo-agent-runner/src/turn_helpers_overlay.rs:1-76` (sole production consumer of `list_for_worker`)

3. **Bare-SQLite repro**: ran the type-mismatch through `sqlite3 :memory:` with the two writer-binding patterns the production code uses (raw bytes for `skill.id`, hyphenated UUID string for `worker_skill.skill_id`), then evaluated the JOIN.

4. **Live cargo test on origin/main HEAD `bc78952a7`**:
   - `cargo test -p argo-skill --lib store_worker_overlay_tests::`

## Observed

### Schema divergence is real and deliberate

- `crates/argo-store/src/migrations/v1.rs:108`: `id BLOB PRIMARY KEY` (skill table)
- `crates/argo-store/migrations/101__worker_skill.sql:29`: `skill_id TEXT NOT NULL`

The migration's own header openly says:
> `worker_id` is TEXT to match the existing `worker.id TEXT PRIMARY KEY` column type in `12__worker_identity.sql` — **the brief's BLOB spec was authored against an outdated schema assumption**.

So the column-type choice was deliberate at table-creation time to
match `worker.id`. But nobody walked the read-side JOIN to confirm
the BLOB↔TEXT comparison would actually match for `skill.id`.

### Writer binds TEXT, reader JOINs against BLOB

- **Writer** (`per_worker_learning_cron.rs:233-241`):
  ```
  "INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, 5) ..."
  .bind(worker_id.to_string())
  .bind(skill_id.to_string())   // <-- hyphenated 36-char UUID string
  ```

- **Reader** (`store_lookups.rs:114-128`):
  ```
  FROM worker_skill ws
  JOIN skill s ON s.id = ws.skill_id   // <-- BLOB(s.id) vs TEXT(ws.skill_id)
  ...
  ```

### Bare-SQLite repro reproduces the mismatch

Running the type comparison directly through `sqlite3 :memory:` with
the exact bind patterns the two production code sites use:

```
--- list_for_worker simulated JOIN ---
(empty result — zero rows returned)

--- typeof both sides ---
blob | text | blob | text

--- compare BLOB(raw 16-byte) vs TEXT(hyphenated 36-char) ---
NOT EQUAL

--- compare BLOB(raw 16-byte) vs TEXT(32-char hex, no hyphens) ---
NOT EQUAL
```

SQLite does NOT silently coerce across the BLOB↔TEXT storage-class
boundary for equality comparisons. The JOIN returns zero matched
rows regardless of whether the textual UUID is hyphenated or
32-hex-char compact.

### Live cargo confirms the existing pin-test is RED on main

```
$ cargo test -p argo-skill --lib store_worker_overlay_tests::

failures:
  store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays
  store_worker_overlay_tests::list_for_worker_filters_other_workers

test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 351 filtered out

assertion `left == right` failed
  left: 0      <-- list_for_worker returned ZERO overlays
  right: 1     <-- after the writer successfully attached ONE
```

**Two tests on `origin/main` HEAD `bc78952a7` are RED.** The 3 tests
that "pass" do so vacuously — they assert filter behaviors
(`pending_review` skipped, superseded skipped, empty-when-no-overlays)
that hold trivially when the JOIN returns empty regardless of inputs.

## Blast radius — production reader path is severed

`SkillStore::list_for_worker` has exactly one production caller
chain, and it is the load-bearing one for Claim 4 MVP:

- `crates/argo-agent-runner/src/turn_helpers_overlay.rs:43`
  (`append_worker_overlays_as_workspace`)
- `crates/argo-agent-runner/src/turn_helpers_overlay.rs:69`
  (`merge_worker_overlays`)

Both functions are called from `load_prompt_skills_with_library`
during agent turn assembly. Their own doc comment is striking
(`turn_helpers_overlay.rs:8-13`):

> Overlay skills come from `worker_skill` (migration 101). The
> per-worker learning cron promotes positive-rated skills into that
> table; **until this brief, the read-side prompt loader did not
> consult it, so promotion was invisible to the worker. These
> helpers close the loop.**

The helpers exist. The loop does NOT close. Every per-worker learned
skill that `per_worker_learning_cron` writes is invisible to
`turn_helpers_overlay → list_for_worker` because the JOIN excludes
it. The agent prompt receives ZERO overlay skills regardless of how
many the cron has promoted.

Note: `crates/argo-identity/src/store_skills.rs::skills_for_worker`
(line 104) and `::skills_learned_for_worker` (line 129) read raw
strings from `worker_skill.skill_id` WITHOUT the JOIN — those
endpoints would return the UUID-strings as opaque values. Whatever
downstream surface treats those strings as skill NAMES (per the
cohort.rs convention) would see them as "unknown skills".

## Anti-theater check — how I proved this isn't theatre

1. **Schema read is from the canonical migration files** — not from a
   doc summary or a commit message. v1.rs:108 + the raw SQL at
   migrations/101__worker_skill.sql are the source of truth that
   `SessionStore::migrate()` actually applies.
2. **Bare-SQLite repro removes the Rust/sqlx layer** — proves the
   mismatch is a SQLite-level type-affinity property, not a sqlx
   bind quirk.
3. **Live `cargo test` on `origin/main` HEAD `bc78952a7`** — cold
   build, ran the actual production-checked-in test file, observed
   2/5 RED. This is not a synthetic test the evaluator authored.
4. **Independent path tracing** — confirmed `list_for_worker` is
   reachable from `turn_helpers_overlay` from `load_prompt_skills_with_library`
   from agent turn assembly, so the broken read isn't dead code.

## What this means for the brief and the loop

The conductor's tick-121 brief
`fix-skill-worker-overlay-blob-text-join-mismatch` is **correctly
scoped and correctly diagnosed**. Recommended for immediate claim by
the next worker:

- The 6-PR boundary-note carry-chain on this brief (#1855 / #1869 /
  #2110 / #2122 / #2143 / #2152) is **explained** by this bug —
  every PR that touched the per-worker overlay path noted "the loop
  doesn't close" without rooting the cause.
- The fact that 2 pin-tests are RED on main and nobody is detecting
  this means the workspace test gate is not exercising
  `argo-skill --lib` in CI (or `--skip-clippy` is hiding it via
  failed gate-2). The conductor's recent `--skip-clippy` admin-merge
  workaround pattern is **almost certainly hiding test failures
  beyond the clippy lints it was scoped to bypass**.

## Brief candidates surfaced this round

NEW (worth flagging to conductor even though the parent brief
already exists in TODO):

(a) **`ops-pre-merge-gate-fix-test-failures-not-just-clippy`** (P0,
~30-50 LOC scripts) — the pre-merge gate's `--skip-clippy` escape
hatch is structurally masking RED tests. At minimum the gate should
parse `cargo test` output for FAILED test names separately from
clippy warnings and refuse `--skip-clippy` when any test is RED.
Without this, the swarm's "land scaffold now, wire later" failure
mode (CLAUDE.md drift-prevention §1) extends to broken tests on
main, which is worse than broken clippy lints.

(b) **`mission-claim-4-mvp-row-2-worker-invokes-installed-skill-blocked-on-overlay-fix`** (P1 dependency-note, no code) — Claim
4 MVP row 2 (worker invokes installed skill through workflow
skill_invoke step) is currently 🟡 IN-FLIGHT on
`wire-worker-session-runner` per round 31's status. That brief
should NOT MERGE until `fix-skill-worker-overlay-blob-text-join-mismatch`
lands first, otherwise row 2's e2e regression test will pass by
appearing to wire the surface while still returning zero overlays.
Conductor: add the dependency-edge in the ledger.

RE-SURFACED still-open (P1, several rounds open):

(c) `ops-scheduled-task-prompt-add-explicit-date-u-instruction`
(operator-actionable SKILL.md edits outside repo, 6 rounds open)

(d) `eval-gateway-uuid-bound-routes-tenant-ownership-sweep`
(7th tick unauthored per round-40 note)

(e) `mission-agent-runner-wire-process-registry-per-session`
(F1 sibling of #2117 triad; F2+F3 closed in round 38)
