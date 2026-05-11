# Mission load-bearing wire — skill explanation (provenance + improvement history)

**Owner**: claude. **Depends on**: existing `crates/argo-skill/` (skill catalog, version history, TrustGrade ladder), `crates/argo-store/src/forge_meta.rs` (per-forge-run metadata), `crates/argo-runtime/src/dreams_plan_templates.rs` (Dreams-derived templates).
**Authored against**: origin/main `d1b6501b` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-explanation-provenance`.

> SOURCE: MISSION_BACKLOG.md Section 5 Claim 7-9 #199 ("Build skill-explanation (provenance + improvement history)"). **Tier M (self-improvement transparency).** Today operators can list skills (`argo skill list`) and see TrustGrade, but the LINEAGE of a skill — who proposed it, which Forge runs improved it, which trajectories trained it, what its parent skill (if any) was — is scattered across `skill_versions`, `forge_runs`, and `dreams_plan_templates`. This brief surfaces a single explain view: `argo skill explain <name>` returns the full provenance + improvement history as a structured JSON envelope.

## Scope

### NEW `crates/argo-skill/src/explanation.rs` (≤ 300 LOC)

- `pub struct SkillExplanation` — full envelope:
  ```rust
  pub struct SkillExplanation {
      pub name: String,
      pub current_version: u32,
      pub current_trust_grade: TrustGrade,
      pub provenance: SkillProvenance,           // who-proposed-this
      pub version_history: Vec<SkillVersionEntry>, // per-version log
      pub forge_runs: Vec<ForgeRunSummary>,      // most-recent-N
      pub dreams_lineage: Option<DreamsLineage>, // if Dreams-derived
      pub trust_history: Vec<TrustObservation>,  // most-recent-N
      pub usage_summary: UsageSummary,           // recent invocation counts
  }
  ```
- `pub enum SkillProvenance` — `BootstrapShipped { ts }`, `OperatorAuthored { actor, ts }`, `DreamsDerived { parent_template_id, ts }`, `ForgePromoted { from_version, ts }`, `Imported { source_url, ts }`.
- `pub struct SkillVersionEntry { version, ts, change_reason, author, trust_grade_at_version, breaking_change }`.
- `pub struct ForgeRunSummary { run_id, ts, judge_score, mutations_applied, accepted }`.
- `pub struct DreamsLineage { parent_template_id, trajectory_count, promotion_judge_score }`.
- `pub struct UsageSummary { invocations_last_7d, invocations_last_30d, p50_latency_ms, p99_latency_ms, error_rate_pct }`.
- `pub async fn explain_skill(store: &Store, name: &str) -> ArgoResult<SkillExplanation>` — orchestrates the query across 4-5 tables.

### CLI: `argo skill explain <name>` subcommand

EXTEND `crates/argo-cli/src/cli_command_enum.rs` (≤ 15 LOC delta) — new `SkillCmd::Explain { name, json, history_depth }` variant.

EXTEND `crates/argo-cli/src/handlers/skill.rs` (≤ 80 LOC delta) — handler that calls `explain_skill` + emits via existing `json` / human-readable formatter pattern.

### Per-skill provenance write-through on existing mutation paths

EXTEND the FEW existing skill-mutation paths to write a structured `SkillProvenance` entry to a new (or existing) `skill_provenance` log table:

NEW migration `vNN.rs` (verify slot — as of brief authoring, v58/SQL 62 is the live tail; pick v59+/SQL 63+ coordinating w/ parallel `mission-planner-bias-from-template` and `mission-gateway-request-audit-log` briefs that reserve slots in this range):

```sql
CREATE TABLE IF NOT EXISTS skill_provenance (
    skill_name      TEXT    NOT NULL,
    skill_version   INTEGER NOT NULL,
    provenance_kind TEXT    NOT NULL,    -- "Bootstrap" | "OperatorAuthored" | "DreamsDerived" | "ForgePromoted" | "Imported"
    actor           TEXT,                  -- operator-id, or NULL for system
    parent_id       TEXT,                  -- template_id / source_url / prior-version depending on kind
    change_reason   TEXT,                  -- one-line summary
    breaking_change INTEGER NOT NULL DEFAULT 0,  -- 0 | 1
    ts              INTEGER NOT NULL,
    PRIMARY KEY (skill_name, skill_version)
);
CREATE INDEX IF NOT EXISTS idx_skill_provenance_name ON skill_provenance(skill_name);
```

EXTEND the 4 known skill-mutation sites to append a provenance row alongside the version bump:

- `crates/argo-skill/src/dreams_distillation.rs` (≤ 15 LOC delta) — on Dreams-derived skill creation, append `DreamsDerived { parent_template_id }`
- `crates/argo-cli/src/handlers/skill_propose.rs` (or `skill_publish.rs`; verify) (≤ 15 LOC delta) — on operator-authored skill, append `OperatorAuthored { actor }`
- `crates/argo-forge/src/forge.rs` (verify path; ≤ 15 LOC delta) — on Forge-promoted version, append `ForgePromoted { from_version }`
- Bootstrap-shipped skills get back-filled by the migration itself: `INSERT INTO skill_provenance (skill_name, skill_version, provenance_kind, ts) SELECT name, version, 'Bootstrap', strftime('%s','now') FROM skill_versions WHERE ...` (idempotent guard).

### Tests

- 7 unit tests in `crates/argo-skill/src/explanation_tests.rs` (≤ 250 LOC): empty-skill, single-version, multi-version-w-forge-promote, dreams-derived w/ template-id, missing-skill error, history-depth limit honored, usage-summary calculation correct.
- 1 integration smoke in `crates/argo-cli/tests/skill_explain_smoke.rs` (≤ 180 LOC): seed 3 skills w/ different provenance kinds; `argo skill explain --json` returns valid envelope; back-fill works on fresh DB.

NO Rust source rewrites in `argo-forge` / `argo-skill::dreams_distillation` beyond the small append-call additions.

This is NOT in scope: the operator-facing UI for browsing provenance (separate productization brief — likely #197 or #200 series); cross-tenant provenance privacy (provenance is per-tenant by construction); time-travel "what did this skill look like at version N" content reconstruction (the version-content is queried via existing skill_versions table); GraphQL / gRPC API surface (CLI + JSON only in v1; HTTP gateway endpoint is a follow-up); ML-explainability ("why did the Forge run accept this mutation") — that's a separate evaluation-suite brief #207.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-skill/src/explanation.rs` (NEW, ≤ 300 LOC)
- `crates/argo-skill/src/explanation_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-skill/src/lib.rs` (EXTEND, ≤ 10 LOC — re-export `SkillExplanation` etc.)
- `crates/argo-cli/src/cli_command_enum.rs` (EXTEND, ≤ 15 LOC)
- `crates/argo-cli/src/handlers/skill.rs` (EXTEND, ≤ 80 LOC)
- `crates/argo-cli/tests/skill_explain_smoke.rs` (NEW, ≤ 180 LOC)
- `crates/argo-store/src/migrations/vNN.rs` (NEW; verify slot — v59+ at brief author time)
- `crates/argo-store/migrations/<NN>__skill_provenance.sql` (NEW; verify slot ≥ 63)
- `crates/argo-store/src/migrations/mod.rs` (EXTEND, register new migration)
- `crates/argo-store/src/skill_provenance.rs` (NEW, ≤ 200 LOC) — `SkillProvenanceRow` + `insert` / `query_for_skill` API
- `crates/argo-store/src/skill_provenance_tests.rs` (NEW, ≤ 150 LOC)
- `crates/argo-skill/src/dreams_distillation.rs` (EXTEND, ≤ 15 LOC delta)
- `crates/argo-forge/src/forge.rs` (EXTEND, ≤ 15 LOC delta — append Forge-promoted provenance)
- `crates/argo-cli/src/handlers/skill_propose.rs` OR `skill_publish.rs` (EXTEND, ≤ 15 LOC delta — verify which is the canonical operator-authored path)

## Files you must NOT touch

- `crates/argo-store/migrations/**/v[0-9]*.sql` (existing migrations stay)
- `crates/argo-skill/src/catalog.rs` (existing catalog API stays; explain composes via existing query)
- Any vertical manifest

## Acceptance criteria

- [ ] `argo skill explain bootstrap-skill-name --json` returns valid `SkillExplanation` envelope
- [ ] `argo skill explain --history-depth 5 bootstrap-skill-name` limits version_history + forge_runs + trust_history each to 5 entries
- [ ] All 7 unit tests in `explanation_tests.rs` pass
- [ ] Integration smoke `cargo test -p argo-cli --test skill_explain_smoke` passes
- [ ] Bootstrap back-fill migration inserts a `Bootstrap` provenance row for every pre-existing skill_version
- [ ] DreamsDerived / ForgePromoted / OperatorAuthored provenance appends work end-to-end (smoke verifies)
- [ ] `argo skill explain <missing-skill>` returns a typed error, not a panic
- [ ] No file > 500 LOC
- [ ] Migration is idempotent (UP/DOWN walks cleanly)
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-explanation-provenance`
- PR title: `skill: explain command + provenance log (lineage / forge-history / dreams-derivation / trust-history) (mission-skill-explanation-provenance)`

## Notes for the implementer

- The 5 `SkillProvenance` variants cover every way a skill enters the system today (bootstrap, operator, dreams, forge, import). Don't add speculative variants — extend later when actually needed.
- The skill_provenance table is APPEND-ONLY in v1. The (skill_name, skill_version) primary-key is intentional: one provenance row per version. If a single version were ever re-classified (e.g. an "OperatorAuthored" later annotated as "DreamsAssisted"), the v2 migration adds an `annotation` column; don't anticipate.
- `usage_summary` field aggregates from existing `audit_chain` / `worker_action_history`-style tables — re-use the existing per-skill invocation query path; don't build new aggregation.
- `history_depth` CLI flag defaults to 20 — enough for operators to see meaningful recent activity without flooding a terminal. Document the default in `--help`.
- The 4 mutation sites get ~15 LOC each — the additive call is `store.skill_provenance_insert(SkillProvenanceRow { ... }).await?;`. Keep the diff surgical; don't refactor the surrounding code.
- The bootstrap back-fill runs ONCE during the v59 (or wherever-it-lands) migration. Idempotent guard: `INSERT OR IGNORE` so re-running the migration on an already-migrated DB is a no-op.
- DO NOT add a CHANGED-FILES diff of the skill content in v1. "What changed between v3 and v4" is a separate diff-view brief (interesting, but scope creep).
- DO NOT expose the provenance through the HTTP gateway in v1. CLI + JSON is sufficient for the explain use-case. Gateway exposure is a follow-up productization brief.
- The `change_reason` field is FREE TEXT, max 200 chars. Don't enforce a controlled vocabulary; operators write whatever helps them remember.
- Migration slot: as of 2026-05-12, the live tail is v58 / raw 62 (v58 operator-risk-overrides #1361). Two parallel briefs (`mission-planner-bias-from-template`, `mission-gateway-request-audit-log`, `mission-audit-chain-watchdog`) reserve slots in this range. Coordinate at claim time — pick the next free slot via `ls crates/argo-store/src/migrations/v*.rs | sort -V | tail -3`.
