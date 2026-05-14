# SKYCOT Phase D Audit — Cross-Cutting Drift Sweep

**Authored**: 2026-05-14 by conductor via Explore agent (a3ac7083478ae2d3b).
**Scope**: Migration slots, parallel-name surfaces beyond Phase A, dead crates, dependency edges, schema drift, TOML invariants, serialization versioning, trait fragmentation.

## Critical findings

### 1. Migration slot collisions in argo-store (operational risk)

argo-store has 11 slot-number collisions:
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

argo-skill highest slot is 78; argo-store highest is 77. Gaps at 34→38 and 63→65 (likely where collisions were resolved by skipping).

**Risk**: sqlx migration runner may process duplicates in filesystem order, executing collisions silently.

### 2. 19 additional parallel-name surfaces (beyond Phase A's 11)

HIGH severity (semantic reuse, not just name overlap):
- `Session` — 7+ crates
- `WorkerId` — argo-identity AND argo-store
- `RunResult` / `RunRequest` — argo-sandbox AND argo-executor-computer-use
- `SkillCandidate` — argo-skill AND argo-memory
- `MemoryEntry` — argo-memory AND argo-vertical
- `JsonRpc{Request,Response,Error}` — argo-mcp AND argo-plugin
- `StepRecord` — 3 crates (Windows + Linux executors + ?)

MEDIUM severity:
- `Workflow` — argo-vertical AND argo-workflow
- `ToolSchema` — argo-provider AND argo-tool

LOW severity (10 more): TickReport, SuggestedAction, SendMessageRequest, SandboxPrewarmConfig, RoutingContext, RetryPolicy, QuietHours, PoolKey, PoolConfig, Edge, Compliance{Soc2Args,ExportArgs,DeleteArgs}.

### 3. 2 possibly-dead non-stub crates

| Crate | LOC | Status |
|---|---|---|
| argo-acp | 873 | Triaged 2026-05-15: kept as standalone ACP library; CLI serve stub was deleted separately, future server wire-up deferred |
| argo-delegate | 582 | Active source; zero workspace consumers despite Phase A claim mapping |

(MCP connectors are intentionally standalone via dynamic loading; not dead.)

### 4. 236 TOML invariant violations

All 88 crates hardcode `version = "0.0.1"` on workspace deps instead of inheriting from `[workspace.package]`. 236 lines to fix in one regex pass.

### 5. Serialization versioning gaps

No `#[serde(rename = "v1")]` or `schema_version` fields found on serialized types. Risk: future schema changes break old-row deserialization.

### 6. Trait fragmentation: 1 case

`AnthropicCaller` defined in 3 crates (argo-runtime, argo-genesis, argo-cli). Likely intentional domain narrowing; LOW severity.

## Recommended briefs (P0/P1 v1-relevant; P2 post-v1)

| Brief | Priority | LOC | Scope |
|---|---|---|---|
| `claude-migration-slot-dedup-and-validator` | P0 | 500 | Fix 11 argo-store slot collisions + add runner-side validation |
| `claude-canonicalize-parallel-names-round2-high` | P1 | 800 | HIGH-sev: WorkerId, Session, RunResult/Request, SkillCandidate, MemoryEntry, JsonRpc*, StepRecord |
| `claude-toml-inherit-workspace-versions` | P1 | 50 | Regex replace 236 version literals |
| `claude-argo-acp-triage` | P1 | 100 | Investigate; archive/delete/wire up |
| `claude-argo-delegate-triage` | P1 | 100 | Investigate; archive/delete/wire up |
| `mission-canonicalize-parallel-names-round2-low` | P2 | 800 | Low-sev bundle (10 types); post-v1 |
| `mission-schema-validation-macro` | P2 | 1000 | Compile-time argo-store schema check; post-v1 |
| `mission-serialization-versioning` | P2 | 1500 | Add `#[serde(rename = "v1")]` everywhere; post-v1 |

## Verdict

**1 P0 (migration slots) is the real operational risk in this phase.** Other findings are P1 hygiene (TOML, triage, type canonicalization round 2) or P2 post-v1 (schema macro, serialization versioning).
