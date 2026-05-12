# Track L — Skill Canvas (overview)

**Authored**: 2026-05-12 against origin/main.
**Status when authored**: NOT STARTED (planned).
**Trigger to start**: DEPLOYMENT-PATH-v3 directive reaches §5 stop condition.
**Directive that launches it**: `SKILL-CANVAS-v1` (to be authored separately).

## Mission

Build a **canvas as a fine-tuner, not a builder**. Workers learn primarily
through natural interaction (observation, on-the-job conversation, computer-use
recording). Track L gives operators a structured view of what was learned —
a graph they can inspect, refine, version, and AB-promote. The canvas is never
the primary builder; it is the X-ray + scalpel for skills the worker already has.

## Why this exists

- StackAI, UiPath Studio, Lindy, Relevance — all treat the visual canvas as the
  primary builder. Operator stares at a blank grid and assembles a flow. That
  pattern doesn't match how line managers think about AI workers.
- The Skycot mental model is "hire a worker, watch it learn, refine when
  needed." The canvas must serve refinement, not construction.
- Current skill substrate (`crates/argo-skill/`) stores skills as `SKILL.md`
  with YAML frontmatter + opaque body. No structured representation exists for
  the runtime to introspect, diff, or visualize. The IR closes this gap.

## Architecture — Skill IR (intermediate representation)

Every skill, regardless of how it entered the system, normalizes to a **Skill Graph**:

```
                            ┌─────────────────┐
   Watch (observe user)  ──▶│                 │
   Demo (record CU)      ──▶│                 │
   Natural-lang teaching ──▶│   Skill IR      │──▶  Worker executes
   Canvas editor         ──▶│   (graph)       │──▶  Live trace overlay
   Brief/manifest        ──▶│                 │──▶  Diff vs prior version
   "Gen from prompt"     ──▶│                 │──▶  AB-promote (existing)
   (Ghostwriter)            └─────────────────┘
```

**Skill IR schema (locked in L1):**

- **Nodes**: action / condition / branch / await-approval / fetch / transform /
  sub-skill-call / loop / try-catch
- **Edges**: transitions tagged (always / on-success / on-failure / on-pattern)
- **Variables**: scoped inputs/outputs flowing through edges
- **Annotations per node**: confidence, sample-count, last-used, success-rate,
  policy (auto vs ask), tool binding (MCP server + tool name)
- **Sub-graphs**: skills compose — a node can be `sub-skill-call` pointing at
  another skill ID + version
- **Storage**: parallel to `SKILL.md`. Existing skills get a one-time backfill
  to IR via L3; new skills emit IR as canonical form with `.md` regenerated as
  a human-readable rendering.
- **Versioning**: IR is version-stamped. Edits create a new version; existing
  `argo-skill/version.rs` + `version_migration.rs` extends to IR.

## Bi-directional canvas

Route: `/workers/<id>/skills/<skill-id>/canvas`

1. **Read-only X-ray**: nodes show confidence + sample count + last-used. Open
   any skill to see exactly what the worker knows.
2. **Live trace overlay**: during a run, current node pulses. Tie to existing
   replay sandbox (E5-d) — replay IS canvas-trace playback.
3. **Edit mode**: drag to refine. Save = skill v+1 → existing AB-promote.
4. **Diff mode**: v(n) vs v(n-1) side-by-side, red/green node deltas.
5. **Suggested edits**: when worker hits low-confidence nodes repeatedly,
   banner: *"the Recruiter hit step 4 with <60% confidence 12 times this week.
   Suggested: insert approval gate."* One-click apply.

**Critical UX invariant**: canvas is never blank. If operator opens canvas on
a skill that doesn't exist, redirect to *"teach this skill first"* (recording
mode or Ghostwriter). Kills the StackAI "blank-grid paralysis" failure mode.

## Phasing

| Phase | Briefs | Ships | LOC | Swarm-hours (at 5/hr) |
|---|---|---|---|---|
| **1 — Foundation + viewer + Ghostwriter** | L1–L6 | Read-only canvas demo + gen-from-prompt | ~3500 | ~1.2 |
| **2 — Editor + diff** | L7–L10 | Operator can refine + AB-promote | ~3000 | ~0.8 |
| **3 — Observation extraction** | L11–L13 | CU recording → skill graph | ~2800 | ~0.8 |
| **4 — Composition + library** | L14–L17 | Sub-skills + canonical library + e2e | ~2000 | ~0.5 |

Total: ~17 briefs, ~11,300 LOC, ~3.3 swarm-hours at 5/hr (one work-day at 10/hr peak).
L15 shrank ~500 LOC after audit confirmed all 6 canonical sub-skills wrap
existing primitives rather than implementing new logic.

## Brief specs

---

### L1 — `skill-ir-schema-contract` (CONTRACT, Tier 0)

**Branch**: `claude-L1-skill-ir-schema`
**PR title**: `feat(skill-ir): IR schema + types (L1 CONTRACT)`
**LOC**: ~500
**Depends on**: nothing.

**Files**:
- `crates/argo-skill/src/ir/mod.rs` — NEW. Module entrypoint.
- `crates/argo-skill/src/ir/types.rs` — NEW. Node/Edge/Graph types.
- `crates/argo-skill/src/ir/serde.rs` — NEW. JSON serialization with stable
  field ordering for hash chain.
- `crates/argo-skill/src/ir/types_tests.rs` — NEW. Schema roundtrip tests.
- `crates/argo-skill/src/lib.rs` — pub mod ir.
- `web/lib/types/skill-ir.ts` — NEW. TS mirror types (hand-maintained;
  Rust is canonical, TS must match field-for-field).

**Scope**:
- Define `SkillGraph`, `Node`, `Edge`, `NodeKind` (9 variants), `EdgeCond`,
  `NodeAnnotations`, `Variable`, `SubSkillRef`.
- JSON serialization with `serde(rename_all = "camelCase")` for TS interop.
- Canonical JSON form for hashing (sorted keys, no whitespace).
- No storage, no runtime, no UI — pure types only.

**Acceptance**:
- [ ] All 9 node kinds typed + JSON-roundtrip-tested.
- [ ] Canonical hash function: `sha256(canonical_json(graph))` → 256-bit hex.
- [ ] TS types compile + structurally match Rust (one fixture round-trips
      Rust→JSON→TS→JSON→Rust).
- [ ] Cargo doc clean; rustdoc on every public type.
- [ ] No file > 500 LOC.

---

### L2 — `skill-ir-storage` (Tier 1)

**Branch**: `claude-L2-skill-ir-storage`
**PR title**: `feat(skill-ir): sqlite storage + versioned writes (L2)`
**LOC**: ~700
**Depends on**: L1 MERGED.

**Files**:
- `crates/argo-skill/src/ir/store.rs` — NEW. `SkillIrStore` trait + sqlite impl.
- `crates/argo-skill/src/ir/store_tests.rs` — NEW.
- `crates/argo-skill/migrations/NNNN_skill_ir.sql` — NEW. Additive migration.
- `crates/argo-skill/src/version.rs` — TOUCH. Extend `SkillVersion` to carry
  optional `ir_hash` field.

**Scope**:
- Table `skill_ir(skill_id, version, ir_json, ir_hash, created_at, tenant_id)`.
- `put(skill_id, version, graph) -> ir_hash`.
- `get(skill_id, version) -> Option<SkillGraph>`.
- `latest(skill_id) -> Option<(version, SkillGraph)>`.
- `list_versions(skill_id) -> Vec<version>`.
- All writes hash-chained against prior version (`prev_hash` column).
- Multi-tenant: tenant_id required on every row.

**Acceptance**:
- [ ] Migration applies cleanly to a fresh DB.
- [ ] Migration applies cleanly on top of an existing populated DB.
- [ ] Round-trip: write graph, read back, hash matches.
- [ ] Hash-chain: tampering with row N detected by hash-mismatch in row N+1.
- [ ] 6 unit tests pass.
- [ ] No file > 500 LOC.

---

### L3 — `skill-ir-backfill` (Tier 1)

**Branch**: `claude-L3-skill-ir-backfill`
**PR title**: `feat(skill-ir): one-shot backfill of existing skills (L3)`
**LOC**: ~600
**Depends on**: L2 MERGED.

**Files**:
- `crates/argo-skill/src/ir/backfill.rs` — NEW.
- `crates/argo-cli/src/commands/skill_ir_backfill.rs` — NEW.
- `crates/argo-skill/src/ir/backfill_tests.rs` — NEW.

**Scope**:
- CLI: `argo skill ir backfill [--dry-run]`.
- Reads all existing `SKILL.md` files.
- For each: parse frontmatter + body, **synthesize a minimal IR**:
  - Single `action` node containing the skill body verbatim as instruction.
  - Annotations: `confidence: 1.0` (curator-authored), `sample-count: 0`,
    `policy: auto`.
  - Skill metadata from frontmatter copied into graph metadata.
- Writes via L2's store.
- Idempotent (re-running produces same hashes).
- Reports "N skills backfilled, M skipped (already have IR)".

**Acceptance**:
- [ ] Backfill runs clean against repo's existing skills.
- [ ] Idempotent — second run = no writes.
- [ ] Dry-run reports counts without writing.
- [ ] No file > 500 LOC.

---

### L4 — `skill-canvas-viewer` (Tier 2)

**Branch**: `claude-L4-skill-canvas-viewer`
**PR title**: `feat(web/canvas): read-only skill canvas viewer (L4)`
**LOC**: ~1200
**Depends on**: L1 MERGED; E0 MERGED.

**Files**:
- `web/app/(app)/workers/[id]/skills/[skillId]/canvas/page.tsx` — NEW. ≤ 350.
- `web/components/skycot/canvas/SkillCanvas.tsx` — NEW. React Flow renderer.
  ≤ 400.
- `web/components/skycot/canvas/NodeRenderer.tsx` — NEW. Per-kind node UI. ≤ 350.
- `web/components/skycot/canvas/EdgeRenderer.tsx` — NEW. ≤ 150.
- `web/components/skycot/canvas/AnnotationsPanel.tsx` — NEW. Side panel.
  ≤ 250.
- `web/lib/api/skill-ir.ts` — NEW. `fetchSkillIr(skillId, version?)`. ≤ 150.
- `web/lib/hooks/use-skill-ir.ts` — NEW. ≤ 80.
- `web/components/skycot/canvas/SkillCanvas.test.tsx` — NEW. ≥ 6 tests.

**Scope**:
- Route loads, fetches IR via gateway (L5-gateway brief, sibling).
- Renders the graph with React Flow (or equivalent — `@xyflow/react`).
- Each node kind has a distinct visual (icon + color).
- Side panel shows annotations for the selected node.
- Read-only — no editing affordances.
- Empty-state: "Worker hasn't learned this skill yet. [ Teach via observation ]
  [ Generate from prompt ]" — second button routes to Ghostwriter (L6).

**Acceptance**:
- [ ] Route loads, renders backfilled IR graphs.
- [ ] All 9 node kinds render distinct.
- [ ] Selecting a node shows annotations.
- [ ] Empty-state CTA wired to L6 placeholder.
- [ ] 6 unit tests pass.
- [ ] `pnpm typecheck` clean. Bundle delta < 25KB (react-flow is heavyish).
- [ ] No file > 500 LOC.

---

### L4-gw — `gateway-skill-ir-endpoint` (Tier 1, sibling to L4)

**Branch**: `claude-L4gw-gateway-skill-ir`
**PR title**: `feat(gateway): GET /api/v1/skills/<id>/ir + /versions (L4-gw)`
**LOC**: ~500
**Depends on**: L2 MERGED.

**Files**:
- `crates/argo-gateway/src/adapters/skill_ir.rs` — NEW.
- `crates/argo-gateway/src/router.rs` — TOUCH. Mount routes.
- `crates/argo-gateway/src/adapters/skill_ir_tests.rs` — NEW.

**Scope**:
- `GET /api/v1/skills/<skillId>/ir?version=<v>` — returns IR JSON.
- `GET /api/v1/skills/<skillId>/ir/versions` — returns version list with hashes.
- Tenant-scoped via bearer token.
- Audit-emitting (read access logged).

**Acceptance**:
- [ ] Both endpoints return valid IR JSON with TS-compatible field names.
- [ ] 404 on missing skill or missing version.
- [ ] 403 on cross-tenant access.
- [ ] 5 integration tests pass.
- [ ] No file > 500 LOC.

---

### L5 — `skill-canvas-live-trace` (Tier 2)

**Branch**: `claude-L5-skill-canvas-live-trace`
**PR title**: `feat(web/canvas): live-trace overlay via WebSocket (L5)`
**LOC**: ~700
**Depends on**: L4 MERGED; E1-f WebSocket infra MERGED (or stubbed).

**Files**:
- `crates/argo-gateway/src/ws/skill_trace.rs` — NEW. WS handler emitting
  `node.entered` / `node.completed` / `node.failed` events.
- `crates/argo-agent-runner/src/instrumentation/ir_trace.rs` — NEW. Runtime
  hook: when a worker executes a node, emits trace event with skill_id +
  node_id + tenant.
- `web/components/skycot/canvas/LiveTraceOverlay.tsx` — NEW.
- `web/lib/hooks/use-skill-trace.ts` — NEW.

**Scope**:
- Runtime emits an event per node enter/complete/fail.
- WS topic: `skill-trace.<skill_id>.<run_id>`.
- Canvas subscribes when viewing a live run; pulses the current node.
- Completed nodes get a green check; failed nodes red X.
- Decoupled from L4 viewer — L4 works without L5.

**Acceptance**:
- [ ] Synthetic run emits trace events through to UI in <500ms.
- [ ] Disconnect/reconnect handled cleanly.
- [ ] Tenant-scoped (worker A's run does not leak to worker B's view).
- [ ] 4 integration tests pass.
- [ ] No file > 500 LOC.

---

### L6 — `ghostwriter-prompt-to-ir` (Tier 2)

**Branch**: `claude-L6-ghostwriter-prompt-to-ir`
**PR title**: `feat(skill-ir): Ghostwriter — gen-from-prompt → IR (L6)`
**LOC**: ~900
**Depends on**: L1 MERGED; L2 MERGED.

**Files**:
- `crates/argo-skill/src/ir/ghostwriter.rs` — NEW.
- `crates/argo-skill/src/ir/ghostwriter_prompts.rs` — NEW. Prompt templates.
- `crates/argo-skill/src/ir/ghostwriter_tests.rs` — NEW.
- `crates/argo-gateway/src/adapters/ghostwriter.rs` — NEW. `POST
  /api/v1/skills/draft-from-prompt`.
- `web/app/(app)/workers/[id]/skills/new/page.tsx` — NEW. Prompt entry surface.
- `web/components/skycot/canvas/GhostwriterForm.tsx` — NEW.
- `web/lib/api/ghostwriter.ts` — NEW.

**Scope**:
- Operator types: *"Build a skill that reconciles bank statements weekly: fetch
  the bank export, match line items against the GL, flag deltas over $500 for
  CFO approval."*
- Backend: LLM call (provider via existing manifest system) with structured-
  output prompt that returns a `SkillGraph` JSON.
- Validates against L1 schema; rejects malformed output.
- Stores via L2; returns skill_id + version.
- Frontend: redirects to L4 canvas with draft loaded.

**Acceptance**:
- [ ] Five sample prompts (one per role — Recruiter / AP Specialist /
      Vendor Manager / Support Specialist / Deal Desk Analyst) generate valid
      IR graphs.
- [ ] Generated graphs have ≥ 4 nodes for non-trivial prompts.
- [ ] Schema-validation failures retry once with a self-correction prompt.
- [ ] Tenant-scoped + audit-emitting.
- [ ] 8 unit tests pass.
- [ ] No file > 500 LOC.

---

### L7 — `skill-canvas-editor` (Tier 3)

**Branch**: `claude-L7-skill-canvas-editor`
**PR title**: `feat(web/canvas): editor mode + save → AB-promote (L7)`
**LOC**: ~1400
**Depends on**: L4 MERGED; L6 MERGED.

**Files**:
- `web/app/(app)/workers/[id]/skills/[skillId]/canvas/page.tsx` — TOUCH.
  Mode toggle (read / edit).
- `web/components/skycot/canvas/EditMode.tsx` — NEW.
- `web/components/skycot/canvas/NodePalette.tsx` — NEW. Draggable source.
- `web/components/skycot/canvas/SaveBar.tsx` — NEW.
- `web/lib/api/skill-ir-save.ts` — NEW. `saveSkillIr(skill_id, graph)`.
- `crates/argo-gateway/src/adapters/skill_ir.rs` — TOUCH. Add `POST
  /api/v1/skills/<id>/ir`.
- `crates/argo-skill/src/ir/save_pipeline.rs` — NEW. Save → validate →
  hash-chain → AB-test schedule.

**Scope**:
- Edit mode: drag from palette to canvas; reroute edges; delete nodes.
- Save → new IR version → existing `ab_test.rs` schedules promotion eval.
- Topology validation: graph must have entry + at least one terminal; no
  orphaned nodes.
- Per-node policy toggle (auto vs ask).
- Tool-binding picker per action node (lists MCP tools available to the tenant).

**Acceptance**:
- [ ] Insert / delete / reroute work.
- [ ] Save rejects invalid topology with clear error.
- [ ] AB-test auto-scheduled after save.
- [ ] Edit history visible (last 5 versions).
- [ ] 10 unit tests + 1 Playwright e2e.
- [ ] No file > 500 LOC.

---

### L8 — `skill-canvas-node-palette-primitives` (Tier 3)

**Branch**: `claude-L8-skill-canvas-node-primitives`
**PR title**: `feat(web/canvas): 9 node-kind primitives + inspector (L8)`
**LOC**: ~800
**Depends on**: L7 MERGED.

**Files**:
- `web/components/skycot/canvas/nodes/ActionNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/ConditionNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/BranchNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/AwaitApprovalNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/FetchNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/TransformNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/SubSkillCallNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/LoopNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/TryCatchNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/NodeInspector.tsx` — NEW. Per-kind form.

**Scope**:
- One file per node kind, ≤ 150 LOC each.
- Each renders the node body + an inspector panel for editing its config.
- Inspector forms validate against L1 schema.

**Acceptance**:
- [ ] All 9 node kinds render + edit cleanly.
- [ ] Inspector validation surfaces L1 schema errors inline.
- [ ] Storybook-style component test for each (or equivalent).
- [ ] No file > 500 LOC.

---

### L9 — `skill-canvas-diff-view` (Tier 3)

**Branch**: `claude-L9-skill-canvas-diff`
**PR title**: `feat(web/canvas): version diff view v(n-1) vs v(n) (L9)`
**LOC**: ~800
**Depends on**: L4 MERGED.

**Files**:
- `web/app/(app)/workers/[id]/skills/[skillId]/canvas/diff/page.tsx` — NEW.
- `web/components/skycot/canvas/DiffView.tsx` — NEW. Side-by-side renderer.
- `web/components/skycot/canvas/diff-engine.ts` — NEW. Graph-diff algorithm.
- `web/components/skycot/canvas/diff-engine.test.ts` — NEW.

**Scope**:
- Compare two IR versions: nodes added / removed / changed; edges rewired;
  annotations shifted.
- Side-by-side rendering with red/green/yellow highlights.
- Header summary: "3 nodes added, 1 removed, 2 modified."
- Reachable from canvas via version-picker dropdown.

**Acceptance**:
- [ ] Diff engine: 8 unit tests covering all change classes.
- [ ] Visual rendering: red/green/yellow on the right side, gray on the left.
- [ ] Performance: < 200ms for 100-node graphs.
- [ ] No file > 500 LOC.

---

### L10 — `skill-canvas-export-import` (Tier 3)

**Branch**: `claude-L10-skill-canvas-export-import`
**PR title**: `feat(web/canvas): export/import skill IR as JSON (L10)`
**LOC**: ~500
**Depends on**: L7 MERGED.

**Files**:
- `web/components/skycot/canvas/ExportImport.tsx` — NEW.
- `web/lib/api/skill-ir-export.ts` — NEW.
- `crates/argo-gateway/src/adapters/skill_ir_export.rs` — NEW. Endpoint.
- `crates/argo-skill/src/ir/portable.rs` — NEW. Bundle format.

**Scope**:
- Export: download canvas as `.skill.json` (canonical IR + metadata).
- Import: upload a `.skill.json`, validate, preview diff against existing,
  optional adopt.
- Portable across tenants (tenant-id stripped on export).

**Acceptance**:
- [ ] Round-trip: export → import produces identical hash.
- [ ] Bad files surface validation errors.
- [ ] No file > 500 LOC.

---

### L11 — `skill-cu-recording-to-ir` (Tier 4)

**Branch**: `claude-L11-cu-recording-to-ir`
**PR title**: `feat(skill-ir): synthesize graph from CU recording (L11)`
**LOC**: ~1600
**Depends on**: L1 MERGED; L2 MERGED; existing `argo-executor-computer-use`
recording infrastructure.

**Files**:
- `crates/argo-skill/src/ir/from_recording.rs` — NEW.
- `crates/argo-skill/src/ir/from_recording_synth.rs` — NEW. LLM-driven step
  abstraction.
- `crates/argo-skill/src/ir/from_recording_tests.rs` — NEW.
- `crates/argo-cli/src/commands/skill_from_recording.rs` — NEW.
- `crates/argo-gateway/src/adapters/skill_from_recording.rs` — NEW.

**Scope**:
- Input: CU session recording (sequence of screenshots + actions + DOM dumps).
- Pipeline: segment into logical steps → LLM identifies node-kind per step →
  build edges from order + branch detection → annotate confidence.
- Output: IR graph stored via L2.
- Operator surface (frontend in L13): "Train the Recruiter by demonstrating
  the task."

**Acceptance**:
- [ ] Three recorded fixtures (one per role) synthesize valid IR with
      ≥ 4 nodes each.
- [ ] Confidence annotations populated; low-confidence nodes flagged.
- [ ] Idempotent within same recording.
- [ ] Tenant-scoped + audit-emitting.
- [ ] 6 unit tests pass.
- [ ] No file > 500 LOC.

---

### L12 — `skill-ir-confidence-telemetry` (Tier 4)

**Branch**: `claude-L12-skill-ir-confidence`
**PR title**: `feat(skill-ir): confidence + sample-count telemetry (L12)`
**LOC**: ~600
**Depends on**: L5 MERGED.

**Files**:
- `crates/argo-skill/src/ir/telemetry.rs` — NEW.
- `crates/argo-skill/migrations/NNNN_skill_ir_telemetry.sql` — NEW.
- `crates/argo-skill/src/ir/telemetry_tests.rs` — NEW.

**Scope**:
- Table `skill_ir_node_telemetry(skill_id, version, node_id, sample_count,
  success_count, failure_count, last_used_at, tenant_id)`.
- Trace events from L5 update telemetry rows.
- Confidence = success_count / sample_count (with smoothing for low samples).
- Queryable via `GET /api/v1/skills/<id>/ir/telemetry`.
- Canvas annotations read live confidence from this table.

**Acceptance**:
- [ ] Telemetry rows update within 5s of trace events.
- [ ] Confidence calc handles zero-sample case (returns null, not divide-by-zero).
- [ ] Tenant-isolated.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.

---

### L13 — `skill-canvas-suggested-edits` (Tier 4)

**Branch**: `claude-L13-skill-canvas-suggested-edits`
**PR title**: `feat(web/canvas): low-confidence suggested-edits banner (L13)`
**LOC**: ~900
**Depends on**: L11 MERGED; L12 MERGED.

**Files**:
- `crates/argo-skill/src/ir/suggestion_engine.rs` — NEW. Heuristics that
  detect low-confidence repeated-failure patterns.
- `crates/argo-skill/src/ir/suggestion_engine_tests.rs` — NEW.
- `crates/argo-gateway/src/adapters/suggestions.rs` — NEW. Endpoint.
- `web/components/skycot/canvas/SuggestionsBanner.tsx` — NEW.
- `web/components/skycot/canvas/SuggestionApplyDialog.tsx` — NEW.

**Scope**:
- Cron-tick or on-demand scan: skill IRs with nodes where
  `success_count / sample_count < 0.6` AND `sample_count > 10`.
- Suggested edits are templated patterns: "insert approval gate before low-
  confidence action," "split branch into two paths by observed outcome," etc.
- Banner on canvas: shows top 3 suggestions, one-click apply (creates new
  version, schedules AB-promote).

**Acceptance**:
- [ ] Engine generates ≥ 3 suggestion kinds.
- [ ] Suggestions only surface for skills past the sample-count threshold.
- [ ] Apply creates valid new IR version.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.

---

### L14 — `skill-ir-composition-subgraphs` (Tier 5)

**Branch**: `claude-L14-skill-ir-composition`
**PR title**: `feat(skill-ir): sub-skill-call execution + canvas (L14)`
**LOC**: ~1000
**Depends on**: L7 MERGED.

**Files**:
- `crates/argo-skill/src/ir/composition.rs` — NEW.
- `crates/argo-skill/src/ir/composition_tests.rs` — NEW.
- `crates/argo-agent-runner/src/ir_runtime/sub_skill.rs` — NEW. Runtime
  resolves sub-skill-call at execute time.
- `web/components/skycot/canvas/SubSkillNode.tsx` — TOUCH (from L8).

**Scope**:
- `SubSkillCall` node references another skill by ID + version.
- Canvas renders as a clickable composite node — double-click to drill into
  the sub-skill's canvas.
- Runtime resolves sub-skill at execute time; cycle detection.
- Versioning: pinning a sub-skill version vs floating.

**Acceptance**:
- [ ] 3-skill composition runs end-to-end.
- [ ] Cycle detection rejects A→B→A.
- [ ] Canvas drill-down works.
- [ ] 5 unit tests + 1 e2e.
- [ ] No file > 500 LOC.

---

### L15 — `skill-canonical-library` (Tier 5)

**Branch**: `claude-L15-skill-canonical-library`
**PR title**: `feat(skill-ir): canonical sub-skill library (L15)`
**LOC**: ~700 (revised down — most primitives already exist)
**Depends on**: L14 MERGED.

**This is packaging work, not greenfield.** All 6 canonical sub-skills wrap
existing Rust primitives as IR documents so any skill can compose them via
`sub-skill-call`. The Rust functions exist; L15 authors the IR wrappers + the
canvas picker.

**Existing primitives (substrate L15 wraps):**

| Sub-skill | Existing primitive | Wrap effort |
|---|---|---|
| `retry-with-backoff` | `argo-mcp-connector` shared backoff (PR #1301) | Thin wrapper exposing maxAttempts + backoff strategy |
| `csv-to-records` | `crates/argo-executor-document/src/csv.rs` | Wrapper + schema validation node |
| `escalation-pattern` | `crates/argo-orchestrator/src/escalation.rs` | Wrapper exposing channel chain + timeouts |
| `approval-with-timeout` | E1-* approvals + `argo-orchestrator` routing (+ M8 multi-channel composition) | Wrapper combining the two |
| `document-extraction` | M1 Claude vision extractor | Wrapper that takes a schema + a document handle |
| `auth-flow` | Per-connector OAuth across `argo-mcp-connector-*` | **Only real synthesis work** — abstract the common handshake → token → refresh → store shape into one sub-skill that delegates to the right connector implementation |

**Files**:
- `crates/argo-skill/src/library/canonical/` — NEW directory.
  - `auth_flow.skill.json`
  - `document_extraction.skill.json`
  - `escalation_pattern.skill.json`
  - `approval_with_timeout.skill.json`
  - `csv_to_records.skill.json`
  - `retry_with_backoff.skill.json`
  - `manifest.toml`
- `crates/argo-skill/src/library/loader.rs` — NEW.
- `crates/argo-skill/src/library/loader_tests.rs` — NEW.

**Scope**:
- 6 canonical sub-skills shipped as built-in IR documents that delegate to
  existing Rust primitives.
- Loader on startup ensures each tenant has access to the canonical set.
- Canvas: "Insert canonical sub-skill" picker (one-line addition to L8).

**Acceptance**:
- [ ] 6 canonical sub-skills load + execute end-to-end.
- [ ] Each delegates to the existing primitive (no logic duplication).
- [ ] Canvas picker wired and inserts the right sub-skill IR.
- [ ] Each has at least one integration test composing it inside a parent skill.
- [ ] No file > 500 LOC.

---

### L16 — `skill-marketplace-canvas-templates` (Tier 5)

**Branch**: `claude-L16-skill-marketplace-templates`
**PR title**: `feat(skill-marketplace): canvas-template tier (L16)`
**LOC**: ~800
**Depends on**: L15 MERGED; existing marketplace (`argo-skill/marketplace.rs`).

**Files**:
- `crates/argo-skill/src/marketplace_templates.rs` — NEW.
- `crates/argo-skill/src/marketplace_templates_tests.rs` — NEW.
- `web/app/(app)/marketplace/templates/page.tsx` — NEW. Template browser.
- `web/components/skycot/marketplace/TemplateCard.tsx` — NEW.

**Scope**:
- Marketplace tier: skill templates published with full IR (not just `.md`).
- Template = ready-to-instantiate IR + parameter slots.
- Operator browses, clicks "Use template," fills slots, canvas opens with the
  filled-in graph.

**Acceptance**:
- [ ] Template browser shows ≥ 5 sample templates.
- [ ] Instantiation creates a working skill per tenant.
- [ ] 5 unit + 1 e2e.
- [ ] No file > 500 LOC.

---

### L17 — `skill-canvas-e2e-playwright` (Tier 5)

**Branch**: `claude-L17-skill-canvas-e2e`
**PR title**: `test(web/canvas): full e2e Playwright suite (L17)`
**LOC**: ~700
**Depends on**: L13 MERGED.

**Files**:
- `web/e2e/skill-canvas.spec.ts` — NEW.
- `web/e2e/skill-canvas-fixtures/` — NEW.

**Scope**:
- Full happy paths: view → edit → save → AB-promote.
- Ghostwriter → canvas → refine → save.
- CU-recording → IR → review → adopt.
- Sub-skill drill-down.
- Suggestion apply.

**Acceptance**:
- [ ] Five Playwright scenarios green.
- [ ] Runs in deploy-smoke workflow (K5).
- [ ] No file > 500 LOC.

---

## Cross-cutting acceptance for Track L

- [ ] All briefs MERGED.
- [ ] Live demo: hire a Recruiter → record one screening session → IR appears
      on canvas → operator refines one node → next run shows refined behavior.
- [ ] Ghostwriter demo: type three prompts (one per vertical) → three working
      IRs on canvas.
- [ ] Diff demo: refine a skill → diff shows changes side-by-side.
- [ ] Replay sandbox (E5-d) shows the IR-trace path through the canvas in the
      replay view.
- [ ] PRODUCT_ROADMAP.md Track L flipped DONE.

## Operator decisions — locked 2026-05-12

1. **Canvas renderer**: **React Flow (`@xyflow/react`)**. Revisit only if bundle
   becomes a problem post-ship; swap to custom SVG over the same Skill IR.
2. **Sub-skill versioning policy**: pinned by default, with an explicit
   "follow latest" toggle per call site.
3. **Ghostwriter LLM routing**: **strongest available**, regardless of the
   worker's runtime provider. Skill drafting is a one-time cost; quality
   matters more than parity with execution provider.
4. **Canonical sub-skill set (L15)**: ship these 6 in v1 —
   `auth-flow`, `document-extraction`, `escalation-pattern`,
   `approval-with-timeout`, `csv-to-records`, `retry-with-backoff`. **All six
   have existing Rust primitives in the codebase**; L15 is packaging work,
   not greenfield. See L15 brief for the substrate-to-wrapper map. Operator
   can amend the set before L15 starts; more sub-skills get added as they
   become reusable.

## Hardening implied (Track H additions)

- `mission-canvas-pii-aware-rendering` — annotations may carry PII; redact on
  display per E1-e patterns.
- `mission-canvas-multi-cursor` — multiple operators editing simultaneously.
- `mission-canvas-undo-stack` — multi-step undo (v1 has version-as-undo only).
- `mission-canvas-keyboard-nav` — full keyboard navigation for accessibility.

## Track L launch sequence

After DEPLOYMENT-PATH-v3 §5 stop conditions trip:
1. Author `SKILL-CANVAS-v1` directive that inserts L1–L17 with tiered claim
   sequencing identical to v3's pattern.
2. Conductor inserts rows over 1 tick, commit per phase.
3. Suppress mission-* replenishment until L6 MERGED (Phase 1 visible
   deliverable).
4. Stop condition: Track L cross-cutting acceptance check passes.
