# claude-skill-canary-deployment — canary rollout for promoted skills

**Owner**: claude.

**Authored against**: origin/main 2026-05-09 (rewrite v2 by conductor — original v1 used `<resolver>.rs` placeholder + `skill_runs` audit-target that doesn't exist; retargeted to `argo-agent-runner/src/turn.rs` + action-ledger audit source. Migration slots also retargeted post-cascade to raw 41__ + v42). **Depends on**: skill versioning (claude-skill-versioning-ab-test);
existing `argo-agent-runner` skill lookup path.

## Why

Promotion today is a binary swap: the new version replaces the old, full
traffic, no rollback signal beyond "is the system on fire?" A canary lets
us route a small percentage of invocations to the new version and watch the
quality / refusal / latency signals diverge before going to 100%. Mission
claim 4 ("trust") is hollow without a graceful rollout primitive.

## Scope

- **NEW** `argo_skill::canary` module exposing:
  - `pub struct CanaryConfig { skill_id, baseline_version, canary_version, traffic_percent: u8, started_at, expires_at }`.
  - `pub fn pick_version(cfg: &CanaryConfig, invocation_seed: u64) -> &str` — deterministic per seed (seeded hash, not RNG).
  - `pub struct CanaryStore { ... }` persistence facade with `set/clear/list/get`.
- Schema: NEW table `skill_canary` (skill_id PK, baseline_version, canary_version,
  traffic_percent, started_at, expires_at). Migration `38__skill_canary.sql`
  (or 39 if the share PoC also lands as 38 — coordinate with conductor).
- Runtime hook: when the agent runner resolves a skill body, it consults
  `CanaryStore::get(skill_id)`. If a config exists and the per-invocation
  seed picks the canary, the runner returns the canary version's body;
  otherwise baseline. Add a single integration point in the runner.

## Approach

1. **NEW** `crates/argo-skill/src/canary.rs` (~300 LOC):
   - Config + store types; `pick_version` uses `seahash`/`std::hash::DefaultHasher`
     stable hashing — no new deps.
   - `is_canary(seed, percent)` returns `(seed_hash % 100) < percent`.
2. **NEW** migration: `38__skill_canary.sql` (or 39 — pick whichever is
   free). Coordinate with the share-cross-tenant brief if both target 38.
3. Runner hookup: read existing skill resolution path with
   `grep -rn "skill body\|fetch_skill_body\|active body" crates/argo-agent-runner/src/`,
   then add a single `if let Some(cfg) = canary.get(skill_id) { ... }` branch.
   Keep the change ≤ 30 LOC inside the runner.
4. Audit: every canary pick writes a row to `action ledger (skill_runs table doesn't exist post-cascade; use `argo-audit::AuditSink::write` with audit kind for canary picks)` with `kind=canary_pick`,
   from_version, to_version chosen, seed (hashed). Reuse existing action ledger (skill_runs table doesn't exist post-cascade; use `argo-audit::AuditSink::write` with audit kind for canary picks)
   helper.

## Files you may touch

- `crates/argo-skill/src/canary.rs` (NEW, ≤ 360 LOC).
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `pub mod canary;` + re-export.
- `crates/argo-store/migrations/41__skill_canary.sql` (NEW — slot 41; depends on `claude-skill-share-cross-tenant-poc` having taken slot 40).
- `crates/argo-store/src/migrations/v42.rs` (NEW, ≤ 80 LOC) wrapping the SQL. (v40 is cascade tombstone; v41 is the share PoC; v42 is this brief.)
- `crates/argo-store/src/migrations/mod.rs` — narrowly: 1 `mod v42;` + insertion in `all_migrations()`.
- `crates/argo-agent-runner/src/turn.rs` — narrowly: ≤ 30 LOC for the canary branch (replaces the brief's prior placeholder `<resolver>.rs` reference; turn.rs is the per-turn dispatch entry point).
- `crates/argo-skill/tests.rs` — narrowly: canary picker unit tests.
- `crates/argo-agent-runner/tests/canary_routing.rs` (NEW, ≤ 250 LOC).

## Files you must NOT touch

- CLI surface — out of scope; CLI wrapper lands in a follow-up.
- A/B testing module (`crates/argo-skill/src/ab_test.rs`) — related but
  distinct mechanism; do NOT merge them in this brief.
- `crates/argo-skill/src/marketplace.rs`.

## Acceptance criteria

- [ ] `pick_version` is deterministic for fixed seed.
- [ ] At `traffic_percent=10`, ~10% of seeds pick the canary (±2% over 10k samples).
- [ ] Expired configs (`expires_at < now`) → always baseline; never canary.
- [ ] `traffic_percent=0` → always baseline.
- [ ] `traffic_percent=100` → always canary.
- [ ] Migration applies UP cleanly; DOWN drops table cleanly.
- [ ] Audit row written on each canary pick.
- [ ] Runner falls back to baseline on missing canary config.
- [ ] 8+ tests: deterministic pick, distribution, expired, 0%, 100%, audit, runner integration, missing config.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-canary-deployment`
- PR title: `skills: canary rollout for promoted versions (claude-skill-canary-deployment)`

## Notes

- The agent runner integration is the riskiest part — keep the diff there
  small and test the fallback (no canary config) explicitly.
- `expires_at` defaults to NULL = no expiry; ops can explicitly set TTLs in
  CLI follow-up.
- Brief assumes "next free migration is 38+" per BUILD_PLAN_V11; coordinate
  numbering with the share PoC at brief-write time.
