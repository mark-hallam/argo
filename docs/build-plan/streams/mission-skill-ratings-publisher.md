# Mission load-bearing wire — skill ratings: operator thumbs propagate to publisher

**Owner**: claude. **Depends on**: `argo-skill::feedback::FeedbackVote` SHIPPED (verified at `crates/argo-skill/src/feedback.rs:23`); `argo-store::feedback` storage SHIPPED (`crates/argo-store/src/feedback.rs`); `argo-cli::cli_feedback_cmds::FeedbackCmd` SHIPPED (`crates/argo-cli/src/cli_feedback_cmds.rs:8`); marketplace catalog wiring SHIPPED (`marketplace.rs`).
**Authored against**: origin/main `939e3bec` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-ratings-publisher`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 #80 — "Build skill ratings (operator thumbs propagate to publisher)." Today operator thumbs (`FeedbackVote::Up` / `Down`) influence local skill-learning but NEVER propagate beyond the tenant boundary. Mission promise: when a tenant uses a published skill and the operator thumbs it, that signal aggregates upstream to the publisher's marketplace listing as a `rating_count` + `rating_score`.

## Scope

1. **NEW migration**: `<next-slot>__skill_rating.sql`:
   ```sql
   CREATE TABLE skill_rating (
       skill_id BLOB NOT NULL,
       skill_version TEXT NOT NULL,
       publisher_tenant_id BLOB NOT NULL,
       rater_tenant_id BLOB NOT NULL,
       vote TEXT NOT NULL,                  -- "up" / "down"
       rated_at INTEGER NOT NULL,
       comment TEXT,
       PRIMARY KEY (skill_id, skill_version, rater_tenant_id)
   );
   CREATE INDEX idx_skill_rating_by_skill ON skill_rating(skill_id, skill_version);
   ```
   Versioned counterpart `v<next>.rs`; bump `EXPECTED_SCHEMA_VERSION`. Verify slot vs in-flight briefs.

2. **`SkillRatingStore` API** (sibling module — keeps `skill_rating.rs` separate from existing `skill.rs`):
   - `record_rating(skill_id, version, publisher, rater, vote, comment)` — idempotent on `(skill_id, version, rater)`; updates row if exists.
   - `aggregate_ratings(skill_id, version) -> (up_count: u32, down_count: u32, score: f32)` — `score = up / (up + down)`, 0.0 when empty.
   - `list_for_publisher(publisher_tenant) -> Vec<RatingSummary>` — for publisher dashboard view.

3. **CLI**: `argo skill rate <skill_id> --version <v> --vote up|down [--comment "..."]` — operator-facing rate command. Looks up publisher_tenant from local skill metadata. If skill isn't installed from a marketplace (no publisher), error explicitly: "only published skills can be rated."

4. **`argo skill ratings <skill_id> [--version <v>] [--json]`** — show aggregated counts + score + recent comments (most-recent 10). JSON envelope per repo CLI convention.

5. **Wire into existing thumbs flow**: when an operator votes via existing `argo feedback record ...` (verify exact subcommand at `cli_feedback_cmds.rs`) on a skill-action invocation, if the underlying skill is marketplace-installed, ALSO call `record_rating` with the same vote. Same operator → same vote across surfaces (no double-counting via PK).

6. **Publisher-side aggregation**: NOT shipped in this brief — the rows accumulate in the publisher's `skill_rating` table when their skill is published. Aggregation/UI for the publisher dashboard is a follow-up (`mission-skill-ratings-publisher-ui`). The wire here is the data model + ingestion CLI; UI ships separately.

This is NOT in scope: cross-tenant transport (operator thumbs stay local until publisher pulls or tenant pushes — separate brief); UI/dashboard rendering; spam/sybil detection; comment moderation; rating reset / appeal flow.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__skill_rating.sql` (NEW) — verify slot.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned migration.
- `crates/argo-store/src/lib.rs` — narrowly scoped: bump `EXPECTED_SCHEMA_VERSION` (currently 47 at line 151).
- `crates/argo-store/src/skill_rating.rs` (NEW, ≤ 300 LOC) — SkillRatingStore impl.
- `crates/argo-store/src/skill_rating_tests.rs` (NEW, sibling, ≤ 200 LOC) — 6 unit tests: roundtrip-up; roundtrip-down; idempotent-on-pk; aggregate-empty-returns-zero; aggregate-mixed; list-for-publisher-filters.
- `crates/argo-cli/src/handlers/skill_rate.rs` (NEW, ≤ 150 LOC) — `argo skill rate` handler.
- `crates/argo-cli/src/handlers/skill_ratings.rs` (NEW, ≤ 150 LOC) — `argo skill ratings` handler.
- `crates/argo-cli/src/cli_subcommands_skill.rs` (verify path) OR `cli_command_enum.rs` — narrowly scoped: ADD `Rate` + `Ratings` subcommand variants.
- `crates/argo-cli/src/handlers/feedback_record.rs` OR sibling (verify path) — narrowly scoped: detect "is this a marketplace-installed skill?" + chain into `record_rating` when yes.

## Files you must NOT touch

- `crates/argo-skill/src/feedback.rs` — reuse FeedbackVote enum only; no behavior change
- `crates/argo-skill/src/marketplace.rs` — reuse `find_by_id` lookup only (publisher_tenant lives here)
- Any tenant-trust-graph crate (`mission-cross-tenant-trust-graph` owns that)
- Any non-store/non-cli crate

## Acceptance criteria

- [ ] Migration adds `skill_rating` table with composite PK + index
- [ ] `EXPECTED_SCHEMA_VERSION` bumped
- [ ] `SkillRatingStore::record_rating` idempotent (re-vote replaces row, no PK conflict)
- [ ] `aggregate_ratings` returns 0/0/0.0 on empty input (no division-by-zero)
- [ ] `argo skill rate <id> --vote up` works end-to-end
- [ ] Rating a non-marketplace skill returns explicit "only published skills can be rated" error
- [ ] `argo skill ratings <id>` JSON envelope shape stable
- [ ] Existing `argo feedback record` for a marketplace-installed skill ALSO writes a `skill_rating` row
- [ ] 6 store unit tests + 4 CLI unit tests + 1 integration smoke (rate + read-back)
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-store -p argo-cli` clean
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-ratings-publisher`
- PR title: `skill: skill_rating storage + argo skill rate/ratings CLI + feedback wire (mission-skill-ratings-publisher)`

## Notes for the implementer

- Migration slot: verify against in-flight briefs at claim time. Multiple `mission-skill-*` and `mission-delegation-budget-tree` may also be staging migrations; cross-check.
- The "publisher_tenant_id from skill metadata" lookup: in v1 a marketplace-installed skill has its publisher recorded somewhere on install (verify: `grep -rn 'publisher\|origin_tenant' crates/argo-skill/src/`). If that field doesn't exist yet, BLOCK and request a precursor brief — DON'T silently add it.
- The "double-write via existing feedback" surface: be careful — the existing `argo feedback record` command may already be the only entry point. If so, this is purely additive (the chain just calls `record_rating` after existing logic). Document the exact wire point in PR body.
- For the comment field: keep it ≤ 500 chars at the store layer to prevent abuse. Use a check constraint or app-level guard.
- Rate-limiting / sybil resistance is OUT of scope — operators self-police; one rating per (skill, version, rater_tenant).
