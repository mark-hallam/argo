# Track M — Enterprise Parity (closing UiPath gaps)

**Authored**: 2026-05-12 against origin/main.
**Status when authored**: NOT STARTED (planned).
**Trigger to start**: Track L Phase 1 (L1–L6) MERGED.
**Directive that launches it**: `ENTERPRISE-PARITY-v1` (to be authored separately).

## Mission

Close the high-leverage gaps surfaced by the Skycot vs UiPath comparison.
**Selectively** — not every UiPath feature deserves a track. The criteria:

1. **Procurement-blocker risk**: does the gap cause Skycot to lose deals
   it would otherwise win?
2. **Architectural fit**: does closing it extend existing Skycot primitives,
   or does it require greenfield work in a different shape?
3. **Time horizon**: can a meaningful slice ship in <2 swarm-weeks?

Gaps that fail these criteria are deferred — `Process Mining`, `Task Mining`,
and `Communications Mining` are real UiPath products but represent different
businesses, not parity work. They move to a future Track N if/when warranted.

## What's in Track M (5 sub-tracks)

| Sub-track | Why it's here | UiPath equivalent |
|---|---|---|
| **M-Doc** — Document Understanding | Loses every deal where extraction is primary | UiPath Document Understanding |
| **M-Orch** — Orchestration maturity | argo-orchestrator exists; need queue/SLA/retry surface | UiPath Orchestrator + Action Center |
| **M-Eval** — Regression eval surface | argo-eval exists; need operator UI + self-healing | UiPath Test Cloud |
| **M-Code** — Pro-code skill mode | Closes the engineer-grade authoring tier | UiPath Studio (pro-code mode) |
| **M-Tmpl** — Vertical template packs | Persona-shaped templates as fast-start surface | UiPath industry templates |

## What's NOT in Track M (and why)

| Deferred | Reason |
|---|---|
| Process Mining | Different product; observes existing human workflows to surface automation candidates. Skycot's "watch to learn" is for worker training, not discovery. Build it only when a buyer asks for it. |
| Task Mining | Same as above. |
| Communications Mining (NLU on email/chat) | Vertical packs (M-Tmpl) cover this for shipped verticals; a horizontal NLU surface is a separate product. |
| 20-year UI selector library | Cannot catch up; mitigate via CU generalization + MCP-first. Not a track. |
| Compliance certifications | Procurement/legal program, not engineering. Pursue SOC2 Type II audit when first design-partner enterprise demands it. |

## Phasing

| Phase | Sub-track | Briefs | Ships | Swarm-hours (at 5/hr) |
|---|---|---|---|---|
| **1 — Doc + Orch surfaces** | M-Doc, M-Orch | M1–M8 | Doc extraction + queue UI in production | ~3.6 |
| **2 — Eval + Code** | M-Eval, M-Code | M9–M14 | Regression eval surface + TS skill mode | ~2.4 |
| **3 — Vertical templates** | M-Tmpl | M15–M20 | 5 vertical packs deployable | ~3.0 |

Total: ~20 briefs, ~14,500 LOC, ~9 swarm-hours at 5/hr (one work-week at 5/hr).

---

## Sub-track M-Doc: Document Understanding

### Why
UiPath's Document Understanding is the industry standard for OCR + structured
extraction. Skycot has `argo-executor-document` (PDF/CSV/MD/MSGraph) but no
vision-based extraction pipeline.

### Architectural fit
Extends `argo-executor-document/`. **Primary provider: Anthropic Claude vision**
— OCR + structured extraction in a single LLM call (no separate OCR step).
Anthropic is already in the Skycot provider manifest, so this adds zero new
vendor integrations. **Fallback for airgapped / sovereign deployments**:
PaddleOCR self-hosted (M-Doc-airgapped, deferred). Provider routed per tenant
via TOML config.

### Cost frame
At ~$3 per 1,000 page-images (Claude Sonnet vision), a tenant processing
10,000 invoices/month spends ~$30 — irrelevant compared to the FTE-equivalent
value. Cost only becomes material at 1M+ pages/month; at that scale
self-hosted PaddleOCR is the answer regardless.

### Briefs

#### M1 — `doc-claude-vision-extraction` (Tier 1, foundation)

**Branch**: `claude-M1-doc-claude-vision-extraction`
**LOC**: ~1100
**Depends on**: nothing.

**Files**:
- `crates/argo-executor-document/src/extract/mod.rs` — NEW.
- `crates/argo-executor-document/src/extract/schema.rs` — NEW. JSON-schema-
  driven extraction targets.
- `crates/argo-executor-document/src/extract/claude_vision.rs` — NEW.
  Claude vision provider — OCR + structured extraction in one call.
- `crates/argo-executor-document/src/extract/provider.rs` — NEW.
  `ExtractionProvider` trait so airgapped fallback (M-Doc-airgapped) can
  plug in later.
- `crates/argo-executor-document/src/extract/tests.rs` — NEW.

**Scope**:
- Operator defines extraction schema as JSON Schema (e.g.
  `{invoice_number, total, line_items[]}`).
- Extractor sends the document image + schema to Claude vision, receives
  schema-conformant JSON in one call. No separate OCR step.
- Schema validation rejects non-conformant output; one retry with self-
  correction prompt.
- Per-field confidence inferred from Claude's response.
- Audit-emitting (page count + provider + model + cost logged).
- Provider trait so PaddleOCR fallback can be added later via M-Doc-
  airgapped without API change.

**Acceptance**:
- [ ] Invoice fixture: extracts invoice number, total, line items with > 95%
      field accuracy.
- [ ] Receipt fixture: extracts merchant, total, tax, items.
- [ ] Scanned-contract fixture: extracts party names + key dates.
- [ ] Schema-validation failures retry exactly once.
- [ ] Confidence scores populated per field.
- [ ] 8 unit tests + 3 fixture-based integration tests.
- [ ] No file > 500 LOC.

---

#### M2 — `doc-airgapped-fallback-paddleocr` (Tier 4, deferred)

**Branch**: `claude-M2-doc-airgapped-paddleocr`
**LOC**: ~900
**Depends on**: M1 MERGED.
**Status**: deferred — author only when first airgapped/sovereign customer
asks. Cloud customers don't need this.

**Files**:
- `crates/argo-executor-document/src/extract/paddleocr.rs` — NEW.
- `crates/argo-executor-document/src/extract/paddle_extract.rs` — NEW.
  Two-stage pipeline: PaddleOCR text → local LLM (Qwen vision or similar)
  with schema prompt.
- `deploy/paddleocr/` — NEW. Self-host deployment recipe.

**Scope**:
- PaddleOCR for OCR (CPU-runnable; GPU much faster).
- Local LLM for schema-conformant extraction.
- Provider trait from M1 makes this pluggable per-tenant via config.

**Acceptance** (when authored):
- [ ] Same 3 fixtures pass with > 90% field accuracy.
- [ ] Deploy recipe brings up the stack on a fresh VM in < 15 min.
- [ ] No outbound network required during inference.
- [ ] No file > 500 LOC.

---

#### M3 — `doc-extraction-canvas-node` (Tier 2)

**Branch**: `claude-M3-doc-extraction-canvas-node`
**LOC**: ~600
**Depends on**: M1 MERGED; L8 MERGED.

**Files**:
- `crates/argo-skill/src/ir/node_kinds/extract_doc.rs` — NEW. New node-kind
  variant `ExtractDocument`.
- `web/components/skycot/canvas/nodes/ExtractDocumentNode.tsx` — NEW.
- `web/components/skycot/canvas/nodes/ExtractDocumentInspector.tsx` — NEW.

**Scope**:
- Adds `ExtractDocument` to the IR (L1 extends).
- Canvas node: pick OCR provider + paste/upload JSON schema + bind output
  variable.
- Workers running the node call M2's extractor.

**Acceptance**:
- [ ] IR schema extended; backward-compatible (existing IRs parse unchanged).
- [ ] Canvas node renders + edits.
- [ ] Live trace pulses the node during execution.
- [ ] 4 unit tests + 1 e2e.
- [ ] No file > 500 LOC.

---

#### M4 — `doc-table-extraction` (Tier 2)

**Branch**: `claude-M4-doc-table-extraction`
**LOC**: ~800
**Depends on**: M1 MERGED.

**Files**:
- `crates/argo-executor-document/src/extract/tables.rs` — NEW.
- `crates/argo-executor-document/src/extract/tables_tests.rs` — NEW.

**Scope**:
- Specialized table extractor: detects table boundaries, infers headers,
  emits row-major JSON.
- Handles multi-page tables (continuation detection).
- Works on scanned + native PDF.

**Acceptance**:
- [ ] 3 fixtures: bank statement, AR aging report, multi-page invoice
      line-items.
- [ ] Each extracts with > 95% row-level accuracy.
- [ ] No file > 500 LOC.

---

## Sub-track M-Orch: Orchestration maturity

### Why
`argo-orchestrator/` has auction, routing, escalation, handoff — most of the
substrate. Missing: operator-facing surface for SLAs, retries, queues, and the
multi-channel approval composition UiPath calls Action Center.

### Architectural fit
Pure addition. New `crates/argo-orchestrator/src/sla.rs`, `queue.rs`, plus a
gateway endpoint set and frontend surface.

### Briefs

#### M5 — `orch-sla-tracking` (Tier 1)

**Branch**: `claude-M5-orch-sla-tracking`
**LOC**: ~900
**Depends on**: nothing.

**Files**:
- `crates/argo-orchestrator/src/sla.rs` — NEW.
- `crates/argo-orchestrator/src/sla_tests.rs` — NEW.
- `crates/argo-orchestrator/migrations/NNNN_sla.sql` — NEW.

**Scope**:
- `Sla` definition: target latency + escalation policy + breach action.
- Attaches to a duty / skill / worker.
- Tracker watches in-flight runs; emits breach events on threshold.
- Multi-tenant.

**Acceptance**:
- [ ] SLA definition CRUD via gateway endpoint.
- [ ] Breach event fires when synthetic run exceeds threshold.
- [ ] Escalation policy honored (notify / reroute / fail).
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.

---

#### M6 — `orch-retry-policy` (Tier 1)

**Branch**: `claude-M6-orch-retry-policy`
**LOC**: ~700
**Depends on**: M5 MERGED.

**Files**:
- `crates/argo-orchestrator/src/retry.rs` — NEW.
- `crates/argo-orchestrator/src/retry_tests.rs` — NEW.
- `crates/argo-orchestrator/migrations/NNNN_retry.sql` — NEW.

**Scope**:
- Per-skill retry policy: max attempts, backoff strategy, on-which-errors.
- Stored alongside skill IR (annotation, not separate object).
- Existing `argo-mcp-connector` shared retry-with-backoff (PR #1301) is the
  primitive; this brief wires it into the orchestrator at duty level.

**Acceptance**:
- [ ] Policy definition + storage.
- [ ] Synthetic failing skill retries per policy.
- [ ] Honors `argo-mcp-connector` shared backoff.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.

---

#### M7 — `orch-queue-management-ui` (Tier 2)

**Branch**: `claude-M7-orch-queue-management-ui`
**LOC**: ~1100
**Depends on**: M5 MERGED; M6 MERGED; E0 MERGED.

**Files**:
- `web/app/(app)/orchestration/page.tsx` — NEW. ≤ 350.
- `web/components/skycot/orch/QueueTable.tsx` — NEW. ≤ 350.
- `web/components/skycot/orch/SlaCard.tsx` — NEW. ≤ 250.
- `web/components/skycot/orch/RetryBadge.tsx` — NEW. ≤ 100.
- `web/lib/api/orchestration.ts` — NEW.
- `crates/argo-gateway/src/adapters/orchestration.rs` — NEW. Endpoints for
  queue listing + SLA mgmt.

**Scope**:
- `/orchestration` route: list of in-flight + queued runs across workers.
- Filters: worker / skill / SLA state / status.
- SLA cards: live latency vs target with breach indicator.
- Retry badges: "attempt 2/3" with last-error tooltip.
- Operator actions: cancel run, reset retry, force-escalate.

**Acceptance**:
- [ ] Route loads, shows ≥ 50 mock runs cleanly.
- [ ] All filters work.
- [ ] Operator actions hit gateway.
- [ ] 8 unit tests + 1 e2e.
- [ ] No file > 500 LOC.

---

#### M8 — `orch-multi-channel-approval-composition` (Tier 2)

**Branch**: `claude-M8-orch-multi-channel-approval-composition`
**LOC**: ~1000
**Depends on**: M7 MERGED; existing approvals infra (E1-* MERGED).

**Files**:
- `crates/argo-orchestrator/src/approval_routing.rs` — NEW.
- `crates/argo-orchestrator/src/approval_routing_tests.rs` — NEW.
- `web/components/skycot/orch/ApprovalRoutingEditor.tsx` — NEW.

**Scope**:
- Operator defines approval routing: "this skill's high-value approvals go to
  Slack #ceo-approvals, fall back to SMS to CFO if no response in 30min, fall
  back to email if no response in 4h."
- Composition language stored as part of skill IR (annotation).
- Runtime walks the routing tree per the policy.

**Acceptance**:
- [ ] Routing tree: 3-channel fallback with timeouts works end-to-end.
- [ ] Editor renders the tree visually.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.

---

## Sub-track M-Eval: Regression eval surface

### Why
`argo-eval/` has runner + compare + honesty + streaming. Missing: operator-
facing UI + regression detection across skill versions + self-healing eval
fixtures.

### Architectural fit
Pure addition. New routes + endpoint adapters + extension to existing eval
runner.

### Briefs

#### M9 — `eval-regression-detection` (Tier 1)

**Branch**: `claude-M9-eval-regression-detection`
**LOC**: ~900
**Depends on**: nothing (uses existing argo-eval).

**Files**:
- `crates/argo-eval/src/regression.rs` — NEW.
- `crates/argo-eval/src/regression_tests.rs` — NEW.
- `crates/argo-skill/src/ab_test.rs` — TOUCH. Wire regression detector into
  AB-promote gate.

**Scope**:
- Compare eval results between two skill versions on the same fixture set.
- Statistical regression detection: per-fixture pass/fail + aggregate score
  + significance test.
- AB-promote gate calls this; refuses promotion on detected regression unless
  override flag set.

**Acceptance**:
- [ ] Two fixture sets, deliberate regression detected.
- [ ] No false positives on noise-level variation.
- [ ] AB-promote gate refuses regression unless override.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.

---

#### M10 — `eval-results-dashboard` (Tier 2)

**Branch**: `claude-M10-eval-results-dashboard`
**LOC**: ~1100
**Depends on**: M9 MERGED.

**Files**:
- `web/app/(app)/workers/[id]/evals/page.tsx` — NEW. ≤ 350.
- `web/components/skycot/eval/EvalRunHistory.tsx` — NEW. ≤ 250.
- `web/components/skycot/eval/EvalFixtureTable.tsx` — NEW. ≤ 250.
- `web/components/skycot/eval/RegressionBanner.tsx` — NEW. ≤ 150.
- `web/lib/api/evals.ts` — NEW.
- `crates/argo-gateway/src/adapters/evals.rs` — NEW.

**Scope**:
- Per-worker eval surface: latest run, history, per-fixture detail.
- Regression banner when a version causes a regression.
- Drill-in: per-fixture pass/fail with diff against prior version's output.

**Acceptance**:
- [ ] Route loads, history renders.
- [ ] Drill-in shows per-fixture detail.
- [ ] Regression banner renders for synthetic regression.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.

---

#### M11 — `eval-self-healing-fixtures` (Tier 2)

**Branch**: `claude-M11-eval-self-healing-fixtures`
**LOC**: ~700
**Depends on**: M9 MERGED.

**Files**:
- `crates/argo-eval/src/self_healing.rs` — NEW.
- `crates/argo-eval/src/self_healing_tests.rs` — NEW.

**Scope**:
- When a fixture fails for a reason the LLM judges environmental (UI changed,
  selector moved, external API renamed a field), the eval runner proposes a
  fixture update.
- Operator confirms via UI (in M10's dashboard).
- Mirrors UiPath's "self-healing tests" feature.

**Acceptance**:
- [ ] Synthetic environmental failure produces a fixture-update proposal.
- [ ] Operator approval applies the update.
- [ ] Logical failures NOT proposed for self-heal.
- [ ] 5 unit tests.
- [ ] No file > 500 LOC.

---

## Sub-track M-Code: Pro-code skill mode

### Why
UiPath ships three authoring tiers (low-code / pro-code / templates). Skycot's
current authoring is hire-wizard + brief manifests; pro-code mode is missing.
Engineering-heavy customers want to write skills in code.

### Architectural fit
Skill IR (L1) already has structured node kinds. A TypeScript SDK that emits
the IR closes this gap. Rust SDK equivalent for in-process workers.

### Briefs

#### M12 — `skill-sdk-typescript` (Tier 2)

**Branch**: `claude-M12-skill-sdk-typescript`
**LOC**: ~1200
**Depends on**: L1 MERGED.

**Files**:
- `web/packages/@skycot/skill-sdk/` — NEW. Standalone npm package.
- `web/packages/@skycot/skill-sdk/src/index.ts` — NEW.
- `web/packages/@skycot/skill-sdk/src/builders.ts` — NEW.
- `web/packages/@skycot/skill-sdk/src/types.ts` — Re-exports L1 TS types.
- `web/packages/@skycot/skill-sdk/test/builders.test.ts` — NEW.

**Scope**:
- Fluent TS builder API:
  ```typescript
  const skill = defineSkill("invoice-reconciliation")
    .startWith(fetch("bank-export"))
    .then(extractDocument(InvoiceSchema))
    .branch("matches",
      onSuccess(updateRecord("gl")),
      onFailure(awaitApproval("cfo", { channel: "slack" })))
    .build();
  ```
- Compiles to valid Skill IR.
- Publishable as `@skycot/skill-sdk` on npm.

**Acceptance**:
- [ ] Five sample skills (one per role pack) author cleanly in TS.
- [ ] All compile to valid IR matching the L1 schema.
- [ ] npm package builds + has type definitions.
- [ ] 10 unit tests.
- [ ] No file > 500 LOC.

---

#### M13 — `skill-sdk-rust` (Tier 3)

**Branch**: `claude-M13-skill-sdk-rust`
**LOC**: ~900
**Depends on**: L1 MERGED.

**Files**:
- `crates/argo-skill-sdk/` — NEW crate.
- `crates/argo-skill-sdk/src/lib.rs` — NEW. Fluent Rust builder.
- `crates/argo-skill-sdk/src/builders.rs` — NEW.
- `crates/argo-skill-sdk/tests/builders.rs` — NEW.

**Scope**:
- Same fluent API as M12 but in Rust, for in-process workers / vertical packs
  shipped as crates.
- Useful for canonical sub-skill library (L15) — those can be authored via SDK.

**Acceptance**:
- [ ] Same five samples as M12 author cleanly.
- [ ] Compiles to IR matching the L1 schema.
- [ ] No file > 500 LOC.

---

#### M14 — `skill-code-mode-canvas-bridge` (Tier 3)

**Branch**: `claude-M14-skill-code-mode-canvas-bridge`
**LOC**: ~800
**Depends on**: M12 MERGED; L7 MERGED.

**Files**:
- `web/app/(app)/workers/[id]/skills/[skillId]/code/page.tsx` — NEW.
- `web/components/skycot/canvas/CodeView.tsx` — NEW. Monaco editor.
- `web/lib/api/skill-code.ts` — NEW.

**Scope**:
- Canvas → Code: emit M12 SDK source from current IR.
- Code → Canvas: parse SDK source, build IR, render.
- Round-trip with no information loss for the SDK-expressible subset.

**Acceptance**:
- [ ] Five sample skills round-trip canvas→code→canvas with identical IR hash.
- [ ] Syntax errors surface inline.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.

---

## Sub-track M-Tmpl: Vertical template packs

### Why
Harvey at $11B and 11x in sales prove vertical-first is the right product
shape. Skycot's `docs/skycot/ux-research/` already has 5 role archetypes —
the Recruiter, the AP Specialist, the Vendor Manager, the Support Specialist,
and the Deal Desk Analyst. These ship as **deployable vertical packs by role,
not by personal name**: prompt packs + skill graphs + memory schemas + tool
permissions + evaluation cases. A tenant names their worker on hire (e.g.
"call our recruiter Sarah") — the pack defaults to the role title so white-
label installations stay neutral.

**Naming convention** — packs are titled by **role**, never by a first name.
The default `display_name` on a fresh install matches the role title; the
hire wizard prompts the operator for a tenant-chosen name. This keeps packs
neutral across customers, geographies, and languages.

> Note: the existing UX research docs at `docs/skycot/ux-research/` still
> use first-name personas (Priya / Sam / Mariah / Jamal / Nori). Those docs
> need a follow-up pass to convert to role titles; track that as a separate
> docs-only brief (M-Docs-rename, ~200 LOC, no engineering impact).

### Architectural fit
Pure data + assembly work. Each pack is a bundle of:
- One worker manifest (TOML)
- 3–8 starter skills as IR documents
- Memory schema (what facts to seed)
- Tool permissions (which MCP servers are needed)
- 10+ eval fixtures per skill
- Vertical-specific prompts

### Briefs

#### M15 — `vertical-pack-loader` (Tier 1)

**Branch**: `claude-M15-vertical-pack-loader`
**LOC**: ~900
**Depends on**: L1 MERGED; L2 MERGED.

**Files**:
- `crates/argo-vertical-packs/` — NEW crate.
- `crates/argo-vertical-packs/src/lib.rs` — NEW.
- `crates/argo-vertical-packs/src/loader.rs` — NEW.
- `crates/argo-vertical-packs/src/manifest.rs` — NEW.
- `crates/argo-vertical-packs/src/fetcher.rs` — NEW. GitHub release fetch.
- `crates/argo-vertical-packs/src/dev_link.rs` — NEW. Local-path override.
- `crates/argo-cli/src/commands/packs.rs` — NEW. `argo packs install / list /
  update / dev-link / remove`.
- `crates/argo-vertical-packs/tests/loader.rs` — NEW.

**Scope**:
- Pack manifest format (TOML):
  ```toml
  [pack]
  name = "skycot-recruiter"
  version = "0.1.0"
  vertical = "people-ops"
  source = "github:skycot/skycot-packs/recruiter"

  [worker]
  default_display_name = "Recruiter"  # role title; tenant renames on hire
  manifest = "worker.toml"

  [skills]
  files = ["skills/screen-candidate.skill.json", ...]

  [memory]
  seed = "memory/seed.json"
  schema = "memory/schema.json"

  [tools]
  required_mcp = ["ats-greenhouse", "calendar-google"]

  [evals]
  fixtures = "evals/*.json"
  ```
- Fetcher: pulls from `github.com/skycot/skycot-packs` via release tarballs.
  Cached at `~/.argo/packs/<name>/<version>/`. Re-install is zero-network.
- Loader: reads pack from cache, validates, instantiates per tenant.
- **`dev-link`**: `argo packs dev-link <name> <path>` points the runtime at
  a local checkout of skycot-packs (typically `~/code/skycot-packs/`)
  instead of the cached tarball. Lets pack authors iterate without
  GitHub round-trips. Symmetrical to `cargo`'s `[patch]` directive.
- CLI: `argo packs install <name>` / `list` / `update <name>` /
  `dev-link <name> <path>` / `remove <name>`.

**Acceptance**:
- [ ] Pack format documented + validated.
- [ ] Loader installs one sample pack from GitHub release.
- [ ] Cache hit avoids re-download.
- [ ] `dev-link` overrides cache; edits to local checkout reflect in next
      worker run.
- [ ] 8 unit tests.
- [ ] No file > 500 LOC.

---

#### M16 — `vertical-pack-recruiter` (the Recruiter) (Tier 2)

**Branch**: `claude-M16-vertical-pack-recruiter`
**LOC**: ~2000 (mostly data + skills)
**Depends on**: M15 MERGED.

**Files**:
- `packs/recruiter/` — NEW directory.
  - `pack.toml`
  - `worker.toml` (role: Recruiter; default_display_name: "Recruiter")
  - `skills/screen-candidate.skill.json`
  - `skills/schedule-screen.skill.json`
  - `skills/draft-rejection.skill.json`
  - `skills/source-from-linkedin.skill.json`
  - `skills/extract-resume-fields.skill.json`
  - `memory/seed.json` (rubrics, required-fields, escalation patterns)
  - `memory/schema.json`
  - `evals/screen-candidate-fixtures.json` (≥ 15 fixtures)
  - `evals/extract-resume-fixtures.json` (≥ 10 fixtures)
- `packs/recruiter/README.md`

**Scope**:
- Worker manifest defines the Recruiter role: tone, escalation policy,
  default display name (overridable by tenant on hire).
- 5 skills covering the canonical recruiter loop.
- Memory seeded with screening rubric + must-have/nice-to-have language.
- Tools: Greenhouse / Lever / LinkedIn (MCP), Calendar, Slack approvals.
- 25+ eval fixtures for regression detection.

**Acceptance**:
- [ ] Pack installs cleanly on fresh tenant.
- [ ] The Recruiter pack runs end-to-end: receives a JD + 10 resumes,
      screens, schedules qualifying candidates, drafts rejections for the rest.
- [ ] Eval suite green on golden fixtures.
- [ ] No file > 500 LOC.

---

#### M17 — `vertical-pack-ap-specialist` (the AP Specialist) (Tier 2)

**Branch**: `claude-M17-vertical-pack-ap-specialist`
**LOC**: ~2000
**Depends on**: M15 MERGED; M2 MERGED (doc extraction).

**Files**:
- `packs/ap-specialist/` — NEW directory (same shape as M16).
- Skills: `process-invoice`, `match-po`, `flag-exception`, `route-for-
  approval`, `update-erp`.

**Scope**:
- AP Specialist role pack.
- Skills cover invoice ingest → extraction → 3-way match → exception routing
  → ERP update.
- Tools: Microsoft 365 / Google Workspace email, Document Understanding
  (M-Doc), NetSuite / SAP / Sage MCP connectors.

**Acceptance**:
- [ ] Pack installs cleanly.
- [ ] The AP Specialist pack runs end-to-end on 5 sample invoices (varied
      formats).
- [ ] 3-way match flags one deliberate mismatch.
- [ ] Eval suite green.
- [ ] No file > 500 LOC.

---

#### M18 — `vertical-pack-vendor-manager` (the Vendor Manager) (Tier 3)

**Branch**: `claude-M18-vertical-pack-vendor-manager`
**LOC**: ~1800
**Depends on**: M15 MERGED.

**Files**:
- `packs/vendor-manager/` — NEW directory.
- Skills: `draft-rfp-response`, `score-vendor-proposal`, `negotiate-renewal`,
  `track-vendor-sla`, `route-procurement-approval`.

**Acceptance criteria**: per pattern.

---

#### M19 — `vertical-pack-support-tier1` (the Support Specialist) (Tier 3)

**Branch**: `claude-M19-vertical-pack-support-tier1`
**LOC**: ~1800
**Depends on**: M15 MERGED.

**Files**:
- `packs/support-tier1/` — NEW directory.
- Skills: `triage-ticket`, `resolve-from-kb`, `escalate-to-tier2`, `draft-
  resolution-message`, `update-ticket-status`.

**Acceptance criteria**: per pattern.

---

#### M20 — `vertical-pack-deal-desk` (the Deal Desk Analyst) (Tier 3)

**Branch**: `claude-M20-vertical-pack-deal-desk`
**LOC**: ~1800
**Depends on**: M15 MERGED.

**Files**:
- `packs/deal-desk/` — NEW directory.
- Skills: `validate-quote`, `check-discount-policy`, `flag-non-standard-terms`,
  `route-deal-desk-approval`, `draft-deal-summary`.

**Acceptance criteria**: per pattern.

---

## Cross-cutting acceptance for Track M

- [ ] All 20 briefs MERGED.
- [ ] Live demo: install all 5 packs on a fresh tenant; each runs its golden
      eval suite green.
- [ ] Demo: the AP Specialist pack processes an invoice that requires
      Document Understanding (M-Doc) end-to-end.
- [ ] Demo: M7 orchestration page shows live runs across all 5 packs.
- [ ] Demo: M10 eval dashboard shows regression detection on a deliberate
      mis-fix.
- [ ] Demo: M14 code-mode round-trips a skill from one pack.
- [ ] PRODUCT_ROADMAP.md Track M flipped DONE.

## Operator decisions — locked 2026-05-12

1. **Pack distribution — separate `github.com/skycot/skycot-packs` repo from
   day one** as the ecosystem and marketplace play. Positions packs as an
   open catalog (parallel to MCP server registry) where third-party
   consultants, vertical specialists, and partners contribute packs.
   Differentiates Skycot from UiPath / StackAI proprietary template catalogs.

   **Runtime impact: zero.** Packs are pulled once at install, then run
   100% local from `~/.argo/packs/<name>/<version>/`. Same model as cargo
   crates / npm packages / VS Code extensions. No network calls at execution
   time. Pack size is ~50–200 KB; install is ~1s per pack.

   **Dev-time impact: mitigated by `dev-link`.** M15 includes
   `argo packs dev-link <path>` (CLI command, ~100 LOC addition) that points
   the runtime at a local checkout of `skycot-packs` during development.
   M16–M20 work against `dev-link`'d local repos with no GitHub round-trip;
   identical iteration UX to in-repo development. Same pattern as
   `cargo`'s `[patch]` or `pnpm link`.

   **Repo setup**: spin up `github.com/skycot/skycot-packs` before M16
   starts. Initial structure: README + `packs/` directory + `MANIFEST.toml`
   catalog file + `.github/` for pack-validation CI.
2. **OCR / extraction strategy — Anthropic Claude vision** as the canonical
   provider (already in Skycot provider manifest; OCR + structured extraction
   in one call). PaddleOCR self-hosted fallback (M2) deferred until first
   airgapped customer asks. Single-vendor simplification matters.
3. **Pack pricing model** (when it becomes a product): included with the
   platform for design-partner phase. Revisit at GA — likely a per-vertical-
   pack tier or value-tier bundling. Premature to decide now.
4. **Vertical priorities (v1)**: the Recruiter / AP Specialist / Vendor
   Manager / Support Specialist / Deal Desk Analyst. These five span
   People Ops, Finance, Procurement, Service, and Sales — the broadest
   credible enterprise footprint for the first pack release. Add legal-
   paralegal / customer-success / IT-helpdesk as a v2 pack release once v1
   feedback lands.
5. **Naming convention**: packs are titled by **role**, not by first name.
   Default `display_name` matches the role title; hire wizard prompts the
   tenant to name the worker on hire. Tenants can customize freely; Skycot
   ships role-neutral defaults.

## What this track does NOT promise

- It does not match UiPath's 20-year selector library — CU generalization is
  the long-term answer, not a track to "catch up."
- It does not deliver SOC2/ISO/HIPAA/FedRAMP certifications — those are
  compliance program work, not engineering tracks.
- It does not match UiPath's 100K-customer distribution — that's GTM, not
  engineering.

## Track M launch sequence

After Track L Phase 1 (L1–L6) MERGED:
1. **Pre-requisite (one-time, operator)**: create
   `github.com/skycot/skycot-packs` repo with initial structure (README,
   empty `packs/` dir, `MANIFEST.toml`, pack-validation CI workflow). ~30 min
   of operator work, not a brief. Must exist before M16 claims.
2. Author `ENTERPRISE-PARITY-v1` directive that inserts M1–M20 with tiered
   claim sequencing.
3. Conductor inserts rows over 3 ticks (one commit per phase).
4. Suppress mission-* replenishment until M1 MERGED (Claude vision extraction
   = visible deliverable).
5. M16–M20 development uses `argo packs dev-link` against local
   `skycot-packs` checkouts; final pushes land in the separate repo.
6. Stop condition: cross-cutting acceptance check passes.

## Strategic frame

Track M is the **"close enough to UiPath that we're a credible alternative"**
work. It does not aim to surpass UiPath on every axis — that would mistake the
goal. The goal is to remove procurement-blocker gaps so deals can be won on
Skycot's *differentiated* axes (canvas-as-fine-tuner, replay-with-verdict,
MCP-first, white-label, learn-by-watching).

Without Track M, a UiPath comparison ends with "but they don't have document
extraction / orchestration UI / regression eval / pro-code mode / industry
templates." With Track M, the comparison ends with "they match on the basics
and lead on the future."
