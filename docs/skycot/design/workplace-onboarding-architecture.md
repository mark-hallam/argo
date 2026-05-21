# Workplace Knowledge + Worker Onboarding — architecture design

**Status**: design draft, ready to feed the swarm as ~22 sequenced briefs.
**Authored**: 2026-05-21 by claude. Operator directive: "this is a big ask
and spare no expense, make sure nothing is stubs or theatre."
**Scope**: every worker behaves as if they joined the company, read the
handbook, did a week of on-the-job training, and is now governed by the
same business rules every other worker is — for every customer, every
turn, every action.

---

## 1. Operator's vision in plain language

When a person joins a company, they get:

1. The **company handbook** (mission, values, brand voice, "we never say
   the word 'cheap'", customer-facing tone, things you can never promise).
2. The **department playbook** (how your team specifically operates —
   PM team uses Linear and 2-week sprints; GTM team scores leads on
   firmographics + intent).
3. **Training** — they watch a senior colleague do the job, then they do
   one supervised, then they go autonomous once their colleague signs off.
4. **Daily corrections** — six months in, the manager says "actually we
   don't reach out to enterprises on Friday afternoons" and the rule
   sticks for every future task.

SKYCOT today hires AI workers but skips ALL of this. A new worker boots
with their persona file + the vertical's hardcoded behaviors and that's
it. The customer's brand voice, business rules, "things we never do" —
none of it reaches the worker's context. This is why every customer
demo ends with "but how do I tell it WE'RE different?"

This design lands the missing onboarding layer.

---

## 2. The five pillars

```
┌──────────────────────────────────────────────────────────────────────┐
│  PILLAR 1 — Workplace Knowledge Repository (tenant-scoped)            │
│  The company handbook. Branded materials, business rules, FAQs.        │
│  EVERY worker reads it, EVERY turn, in their context window.          │
└──────────────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────────────┐
│  PILLAR 2 — Cohort Playbooks (cohort-scoped overlay)                  │
│  Department-specific knowledge layered on top of the workplace wiki.  │
│  PM Cohort gets sprint cadence + status-report templates. GTM Cohort  │
│  gets ICP + outreach voice. Worker sees: WIKI ∪ COHORT ∪ self-mem.    │
└──────────────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────────────┐
│  PILLAR 3 — Worker Onboarding Pipeline (per-hire, multi-phase)        │
│  Phase 1: read company wiki + cohort playbook (gated quiz).           │
│  Phase 2: watch 3 demo runs of a senior worker in the same role.      │
│  Phase 3: process 5 tasks under mentor supervision (mentor approves   │
│           each output before it ships).                                │
│  Phase 4: autonomy unlock — mentor + operator both sign off.          │
└──────────────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────────────┐
│  PILLAR 4 — Live OTJ (on-the-job) Correction Loop                    │
│  Operator says "actually we never X" mid-task → that becomes a       │
│  learned-skill proposal → trust engine ranks it → on promotion the    │
│  rule sticks for every future turn, persisted to the worker's memory  │
│  AND optionally promoted to cohort or workplace level (with admin     │
│  approval).                                                            │
└──────────────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────────────┐
│  PILLAR 5 — Compliance & Training Audit                              │
│  Every wiki edit, cohort playbook change, onboarding-phase            │
│  completion, OTJ correction, and skill promotion emits an audit row. │
│  "Show me what training Maya received" returns a real timeline.      │
│  "What's the diff between the brand guide today vs. 6 months ago"    │
│  returns a real diff. Procurement-grade.                              │
└──────────────────────────────────────────────────────────────────────┘
```

The five pillars are **interdependent but separately shippable**. Pillar
1 alone (workplace wiki + context injection) unblocks the biggest
demo objection. Pillars 2 + 4 add power; Pillar 3 is the polish that
turns it from "we have a wiki" into "we have a real HR process."

---

## 3. Data model — new tables and extensions

### 3.1 New tables (10)

```sql
-- ─── PILLAR 1 ───────────────────────────────────────────────────────

-- workplace_knowledge_doc — one row per current head of a wiki doc.
CREATE TABLE workplace_knowledge_doc (
    id                   TEXT      NOT NULL PRIMARY KEY,
    tenant_id            TEXT      NOT NULL,
    slug                 TEXT      NOT NULL,        -- "brand-voice", "escalation-policy"
    title                TEXT      NOT NULL,
    category             TEXT      NOT NULL,        -- "brand" | "business-rules" | "compliance" | "internal" | "faq"
    body_markdown        TEXT      NOT NULL,
    version              INTEGER   NOT NULL,         -- monotonic, increments on every edit
    parent_revision_id   TEXT,                       -- chain to prior version
    created_at_micros    INTEGER   NOT NULL,
    created_by           TEXT      NOT NULL,
    updated_at_micros    INTEGER   NOT NULL,
    updated_by           TEXT      NOT NULL,
    UNIQUE(tenant_id, slug)
);

-- workplace_knowledge_revision — every historical version
-- (additive — never destructive, mirrors workflow_definition pattern)
CREATE TABLE workplace_knowledge_revision (
    id                TEXT      NOT NULL PRIMARY KEY,
    doc_id            TEXT      NOT NULL,           -- workplace_knowledge_doc.id
    tenant_id         TEXT      NOT NULL,
    version           INTEGER   NOT NULL,
    body_markdown     TEXT      NOT NULL,
    diff_from_prior   TEXT,                          -- unified-diff for fast read in audit UI
    edited_at_micros  INTEGER   NOT NULL,
    edited_by         TEXT      NOT NULL,
    edit_reason       TEXT                            -- operator-supplied note
);

-- ─── PILLAR 2 ───────────────────────────────────────────────────────

-- cohort — represents a department/team (PM, GTM, Support, Finance, ...)
-- VERIFY existing — argo-store/cohort.rs already has a cohort table.
-- Likely extension: add a `playbook_doc_id` column linking to a wiki doc.
ALTER TABLE cohort ADD COLUMN playbook_doc_id TEXT;

-- cohort_playbook_section — fine-grained playbook content, one row per
-- editable section so admins can edit "voice" without editing "rituals".
CREATE TABLE cohort_playbook_section (
    id                  TEXT      NOT NULL PRIMARY KEY,
    tenant_id           TEXT      NOT NULL,
    cohort_id           TEXT      NOT NULL,
    slug                TEXT      NOT NULL,           -- "voice", "rituals", "tools", "metrics"
    title               TEXT      NOT NULL,
    body_markdown       TEXT      NOT NULL,
    sort_order          INTEGER   NOT NULL,
    version             INTEGER   NOT NULL,
    updated_at_micros   INTEGER   NOT NULL,
    updated_by          TEXT      NOT NULL,
    UNIQUE(tenant_id, cohort_id, slug)
);

-- ─── PILLAR 3 ───────────────────────────────────────────────────────

-- worker_onboarding_run — one row per hire's onboarding pipeline.
CREATE TABLE worker_onboarding_run (
    id                       TEXT      NOT NULL PRIMARY KEY,
    tenant_id                TEXT      NOT NULL,
    worker_id                TEXT      NOT NULL,
    status                   TEXT      NOT NULL
                                  CHECK(status IN ('pending','in_progress','completed','failed','skipped')),
    current_phase            TEXT      NOT NULL
                                  CHECK(current_phase IN ('read_wiki','watch_demos','supervised_tasks','mentor_signoff','operator_signoff','done')),
    mentor_worker_id         TEXT,                   -- senior worker assigned to mentor (null until phase 2)
    started_at_micros        INTEGER   NOT NULL,
    completed_at_micros      INTEGER,
    failed_reason            TEXT,
    UNIQUE(tenant_id, worker_id)                    -- one onboarding per worker
);

-- worker_onboarding_event — phase progress + audit
-- (every state transition writes one row here)
CREATE TABLE worker_onboarding_event (
    id                TEXT      NOT NULL PRIMARY KEY,
    run_id            TEXT      NOT NULL,           -- worker_onboarding_run.id
    tenant_id         TEXT      NOT NULL,
    phase             TEXT      NOT NULL,
    kind              TEXT      NOT NULL
                          CHECK(kind IN ('phase_started','quiz_passed','quiz_failed','demo_watched','task_submitted','task_approved','task_rejected','phase_completed')),
    payload_json      TEXT      NOT NULL,            -- shape depends on kind
    actor             TEXT      NOT NULL,            -- principal (worker, mentor, or operator)
    occurred_at_micros INTEGER  NOT NULL
);

-- worker_supervised_task — tasks the worker did during phase 3.
-- Held in a queue until the mentor approves; only approved tasks
-- get COMMITTED to the underlying system (no side effects until then).
CREATE TABLE worker_supervised_task (
    id                  TEXT      NOT NULL PRIMARY KEY,
    run_id              TEXT      NOT NULL,
    tenant_id           TEXT      NOT NULL,
    task_kind           TEXT      NOT NULL,           -- e.g. "draft_reply", "create_ticket"
    proposed_action_json TEXT     NOT NULL,            -- the action the worker WOULD take
    mentor_verdict      TEXT
                          CHECK(mentor_verdict IN ('approve','reject','rework')),
    mentor_feedback     TEXT,                          -- text feedback if reject/rework
    submitted_at_micros INTEGER  NOT NULL,
    decided_at_micros   INTEGER
);

-- ─── PILLAR 4 ───────────────────────────────────────────────────────

-- otj_correction — one row per operator correction emitted during a worker turn.
CREATE TABLE otj_correction (
    id                  TEXT      NOT NULL PRIMARY KEY,
    tenant_id           TEXT      NOT NULL,
    worker_id           TEXT      NOT NULL,
    session_id          TEXT,
    audit_row_id        TEXT,                          -- the audit row that was corrected
    correction_text     TEXT      NOT NULL,            -- operator's natural-lang correction
    proposed_rule       TEXT,                          -- AI-distilled rule from the correction
    scope               TEXT      NOT NULL
                          CHECK(scope IN ('worker','cohort','workspace')),
    status              TEXT      NOT NULL
                          CHECK(status IN ('pending_review','accepted','rejected','promoted')),
    skill_proposal_id   TEXT,                          -- link to learned_skill_proposal if promoted
    created_at_micros   INTEGER  NOT NULL,
    reviewed_by         TEXT,
    reviewed_at_micros  INTEGER
);

-- learned_skill_proposal — formalises the "from correction → skill" path.
-- (May extend an existing learning-loop table. Verify before adding.)
CREATE TABLE learned_skill_proposal (
    id                  TEXT      NOT NULL PRIMARY KEY,
    tenant_id           TEXT      NOT NULL,
    source_kind         TEXT      NOT NULL
                          CHECK(source_kind IN ('otj_correction','self_observed','operator_taught','watch_recording')),
    source_id           TEXT      NOT NULL,
    skill_name          TEXT      NOT NULL,
    skill_body_md       TEXT      NOT NULL,
    target_scope        TEXT      NOT NULL
                          CHECK(target_scope IN ('worker','cohort','workspace')),
    target_id           TEXT      NOT NULL,             -- worker_id / cohort_id / tenant_id
    status              TEXT      NOT NULL
                          CHECK(status IN ('pending','approved','rejected','expired')),
    proposed_at_micros  INTEGER   NOT NULL,
    reviewed_by         TEXT,
    reviewed_at_micros  INTEGER
);

-- ─── PILLAR 5 ───────────────────────────────────────────────────────

-- (No new table — Pillar 5 is entirely audit_record kinds; see §3.3.)
```

### 3.2 Extensions to existing tables

```sql
-- worker.trust_grade already exists. Onboarding-completion is a
-- prerequisite for trust_grade > 0; before onboarding_run.status = 'completed'
-- the worker can only run supervised tasks.
-- Enforce via the gateway dispatch layer, NOT a DB constraint
-- (additive migrations only).

-- cohort.playbook_doc_id added in §3.1. No other schema changes needed.
```

### 3.3 New AuditKinds (12)

Following the 4-site recipe from sibling brief
`web-audit-kinds-for-journey-events`:

| Variant | Label |
|---------|-------|
| `WorkplaceKnowledgeDocCreated` | `workplace_knowledge.doc.created` |
| `WorkplaceKnowledgeDocUpdated` | `workplace_knowledge.doc.updated` |
| `WorkplaceKnowledgeDocDeleted` | `workplace_knowledge.doc.deleted` |
| `CohortPlaybookSectionUpdated` | `cohort.playbook.updated` |
| `WorkerOnboardingStarted` | `worker.onboarding.started` |
| `WorkerOnboardingPhaseCompleted` | `worker.onboarding.phase_completed` |
| `WorkerOnboardingCompleted` | `worker.onboarding.completed` |
| `WorkerSupervisedTaskSubmitted` | `worker.supervised_task.submitted` |
| `WorkerSupervisedTaskDecided` | `worker.supervised_task.decided` |
| `OtjCorrectionSubmitted` | `otj.correction.submitted` |
| `OtjCorrectionReviewed` | `otj.correction.reviewed` |
| `LearnedSkillProposalPromoted` | `learned_skill.promoted` |

---

## 4. Existing SKYCOT substrate we build on

The good news: SKYCOT already has 70% of the foundations.

| Need | Existing crate / module | What we add |
|------|------------------------|--------------|
| Tenant scoping on every primitive | `argo_types::TenantId`, multi-tenant invariant | Apply to all 10 new tables. |
| Per-worker memory | `argo-memory` (MEMORY.md/USER.md/SOUL.md/AGENTS.md) | Workplace wiki sits ABOVE memory — a 5th layer. |
| Shared-team memory blob | `argo-memory::shared_team` | Replaced by `workplace_knowledge_doc` (structured). |
| Versioned docs with parent chain | `argo_store::workflow_definition` (rollback-able versioning) | Pattern copied for `workplace_knowledge_revision`. |
| Skill substrate | `argo-skill` (SkillStore, TrustGrade) | Learned-skill proposals integrate via existing skill candidate flow. |
| Workflow templates | `argo-vertical` (TOML manifests) | A new `onboarding-vertical` template encodes the 4-phase pipeline. |
| Trust-grade engine | `argo-trust-engine` (just-shipped brief) | Onboarding completion is a positive trust signal. |
| Audit chain | `argo-audit` (sink_chain, exhaustive label/from_label) | 12 new variants follow the 4-site recipe. |
| Cohort grouping | `argo-store::cohort` | Extend with `playbook_doc_id`. |
| Gateway dispatch + RBAC | `argo-cli::handlers::gateway` + `role_gate` | New endpoints follow established gates pattern. |
| Web RBAC role gating | `web-rbac-jwt-role-claim` (just-shipped brief) | Knowledge-doc edits gated to owner+admin; OTJ corrections any role. |
| Magic-link + invitations | `auth_magic_link_*` + `web-settings-members-invite-real-wire` | Onboarding triggered by `WorkspaceMemberJoined`. |

Three things we genuinely have to build new:

1. **Knowledge-doc context injection at turn-start** — modify the agent
   loop's system-message construction to read the wiki + cohort layer.
2. **Worker onboarding state machine** — the pipeline driver that walks a
   worker through the 4 phases.
3. **OTJ correction-to-skill distillation** — operator text →
   learned-skill candidate via a small LLM call.

Everything else is leveraging substrate that already shipped.

---

## 5. Brief sequence (22 briefs, dependency-ordered)

### Wave A — Workplace Wiki MVP (6 briefs, 3 weeks)

1. **`mission-workplace-knowledge-substrate`** — migration v77 + new
   crate `argo-workplace-knowledge` with CRUD ops on
   `workplace_knowledge_doc` + `workplace_knowledge_revision`.
   ~600 LOC. **Prerequisite for everything else in Wave A.**

2. **`mission-workplace-knowledge-audit-kinds`** — add 3 AuditKind
   variants (Created/Updated/Deleted) + emit-site wrappers. ~200 LOC.
   Sibling to `web-audit-kinds-for-journey-events`.

3. **`mission-workplace-knowledge-gateway`** — gateway endpoints
   for CRUD + list + diff + revert. ~800 LOC. RBAC gate: owner/admin
   write, all roles read. Coordinate with `web-rbac-jwt-role-claim-and-
   gateway-enforce`.

4. **`web-workplace-knowledge-editor`** — Next.js page
   `/settings/knowledge` with markdown editor, version history sidebar,
   diff viewer, category nav. ~1000 LOC. Uses existing markdown editor
   pattern from the design system; HashCell pattern for revision IDs.

5. **`mission-knowledge-context-injection`** — the load-bearing one.
   Modify `argo-agent-runner::turn_helpers` (the existing system-message
   construction) to read wiki docs at turn-start and inject them as a
   structured system-message section labeled `[WORKPLACE KNOWLEDGE — read-only,
   do not contradict]`. Cache keyed on `(tenant_id, max_version)` so the
   hot path is one COUNT query per turn. ~700 LOC. **This is what makes
   the wiki real instead of theater.**

6. **`mission-knowledge-startup-seed`** — every new workspace gets
   seeded with 5 starter docs (brand-voice / business-rules / escalation-
   policy / customer-tone / never-do-this) prefilled with sensible
   defaults the operator edits to taste. ~300 LOC.

### Wave B — Cohort Playbooks (3 briefs, 2 weeks)

7. **`mission-cohort-playbook-substrate`** — migration + new module
   `argo-workplace-knowledge::cohort_playbook` + the `cohort.playbook_doc_id`
   column. ~400 LOC.

8. **`mission-cohort-playbook-gateway`** — gateway endpoints for
   section CRUD. ~500 LOC.

9. **`web-cohort-playbook-editor`** — extends `/cohorts/<id>` with a
   `Playbook` tab; section-based editor (one rich-text block per section
   so admins edit "voice" without touching "rituals"). ~700 LOC.

10. **`mission-knowledge-context-cohort-overlay`** — extends Wave A's
    injection to also include the cohort layer for the worker's cohort.
    The injection becomes: `[WORKPLACE] + [COHORT: <name>] + worker memory`.
    ~250 LOC.

### Wave C — Worker Onboarding Pipeline (6 briefs, 4 weeks)

11. **`mission-worker-onboarding-substrate`** — 3 new tables
    (`worker_onboarding_run`, `worker_onboarding_event`,
    `worker_supervised_task`) + state machine in
    `argo-onboarding` crate. ~800 LOC.

12. **`mission-worker-onboarding-vertical-template`** — a vertical
    workflow template that drives the 4-phase pipeline. Stored as
    `verticals/_onboarding/workflows/standard.workflow.toml`. ~400 LOC
    (mostly TOML + audit emission glue).

13. **`mission-worker-onboarding-gateway`** — endpoints to start /
    advance / skip / complete / submit-task / decide-task. ~700 LOC.

14. **`web-worker-onboarding-dashboard`** — page
    `/workers/<id>/onboarding` showing phase progress, mentor's queue,
    quiz state, supervised-task review queue. ~1000 LOC.

15. **`mission-onboarding-mentor-selection`** — selecting a mentor:
    must be same cohort, trust_grade ≥ Tenant, not already mentoring
    > 3 workers, not paused/archived. ~400 LOC.

16. **`mission-onboarding-trust-engine-integration`** — onboarding
    completion emits a positive signal to `argo-trust-engine`, so
    workers who completed onboarding ratchet faster. Sibling to the
    just-shipped trust-engine brief — extend `scoring.rs` with a new
    `onboarding_completion_bonus` signal. ~300 LOC.

### Wave D — OTJ Correction Loop (4 briefs, 3 weeks)

17. **`mission-otj-correction-substrate`** — `otj_correction` +
    `learned_skill_proposal` tables + correction-submission API + the
    LLM-distillation helper that turns natural-lang correction into a
    candidate rule. ~600 LOC.

18. **`mission-otj-correction-trust-signal`** — extend
    `argo-trust-engine` scoring with a new `recent_correction_count`
    signal (frequent corrections lower a worker's score — they're
    making mistakes the operator has to catch). ~300 LOC.

19. **`web-otj-correction-inline-ui`** — every audit row in `/audit`
    (and every worker-turn output in `/workers/<id>/activity`) gets a
    "Correct this" button → modal where the operator types the
    correction → submission emits the audit row + creates the
    `otj_correction` row. ~600 LOC.

20. **`web-otj-correction-review-queue`** — admin page
    `/settings/learned-rules` showing pending OTJ corrections + the
    AI-distilled rule preview + scope buttons (worker / cohort /
    workspace) + approve/reject. Approved corrections become real
    learned_skill_proposal → eventually skill rows tied to the
    target scope. ~700 LOC.

### Wave E — Compliance + Training Audit (3 briefs, 2 weeks)

21. **`web-worker-training-history-page`** — per-worker page
    `/workers/<id>/training` showing the full timeline: which wiki
    versions they were running under, which onboarding phase happened
    when, which OTJ corrections they received, which skill proposals
    were promoted. All derived from `audit_record` filtering. ~700 LOC.

22. **`web-knowledge-diff-viewer`** — public per-revision diff page
    `/settings/knowledge/<slug>/diff?from=<v>&to=<v>` showing the
    semantic markdown diff side-by-side. ~500 LOC.

23. **`mission-onboarding-completion-report`** — gateway endpoint
    `GET /api/v1/workers/<id>/training-report` returning a CSV/JSON
    bundle suitable for HR-style new-hire reporting. ~400 LOC.

---

## 6. Phase plan and shipping cadence

**Total**: 22 briefs, ~13,000 LOC, 12-14 weeks at 2-3 briefs/week swarm pace.

| Week | Wave | Briefs | Demo outcome |
|------|------|--------|---------------|
| 1-3 | A | 1-6 | Operator edits "we never say 'cheap'", worker stops saying cheap NEXT turn. Wiki has 5 seeded docs. |
| 4-5 | B | 7-10 | PM Cohort has its own playbook; GTM doesn't see it. Each worker's system prompt visibly includes the right layer. |
| 6-9 | C | 11-16 | Hire a new worker → walk through 4-phase onboarding → first 5 tasks are mentor-reviewed → autonomy unlocks at 90% approval. Visible in worker-detail page. |
| 10-12 | D | 17-20 | Operator corrects Maya 3x on email tone → 3rd correction shows "promote to cohort?" → click yes → all GTM workers get the rule next turn. |
| 13-14 | E | 21-23 | "Show me Maya's training history" returns a real timeline with revision IDs + signatures (no theater). |

**Suggested swarm allocation**:
- Wave A: 3 claude + 2 codex briefs in parallel; brief 5 (context injection) MUST land before bricks 4/6.
- Wave B: serial after A. Cohort layer depends on the wiki substrate landing first.
- Wave C: 4-6 in parallel; brief 11 (substrate) is the blocker.
- Wave D: 2-3 in parallel after C.
- Wave E: can start during Wave D's later half (uses audit-row reads only).

---

## 7. "Not theater" acceptance criteria

The operator's directive: **"make sure nothing is stubs or theatre."**
Each pillar has a load-bearing test that proves the work is real:

### Pillar 1 — Workplace Wiki

- [ ] **No mock fallback in prod** — `web/lib/api/workplace-knowledge.ts`
  passes through `isStrictProduction()` gate; no sample-data short-circuit.
- [ ] **Context injection observable** — pin a debug-mode env var
  `ARGO_AGENT_DEBUG_SYSTEM_MESSAGE=1`; turn-start logs the literal
  system message the LLM saw. The wiki content MUST appear inline,
  byte-identical to the stored doc body.
- [ ] **Real LLM behavior change** — edit a doc to "Never use the word
  'cheap' — say 'cost-effective' instead." Hire a worker. Send a
  customer query mentioning pricing. The model output MUST avoid
  "cheap." Verify on real Anthropic + OpenAI providers (not just a
  mock).
- [ ] **Audit row per edit** — `SELECT kind, payload FROM audit_record
  WHERE kind = 'workplace_knowledge.doc.updated'` returns rows with
  diff in payload after every edit.

### Pillar 2 — Cohort Playbooks

- [ ] **Cohort isolation** — PM worker's system prompt shows PM
  playbook; GTM worker's shows GTM playbook. Verified by capturing the
  system message under debug mode and string-diffing.
- [ ] **No cross-cohort leak** — Same doc edit verified for PM cohort
  doesn't leak into GTM context.
- [ ] **Cohort-membership change re-injects** — Move a worker from
  PM to GTM cohort; next turn's system prompt MUST show GTM playbook.

### Pillar 3 — Worker Onboarding

- [ ] **Supervised tasks have no side effects** — During phase 3, a
  worker that "draft a reply" → the reply MUST NOT actually be sent;
  it stays in `worker_supervised_task.proposed_action_json` until
  mentor approves.
- [ ] **Mentor-rejected task → corrective audit row** — rejection
  writes a `WorkerSupervisedTaskDecided` row AND a follow-up
  `OtjCorrectionSubmitted` so the worker learns from the rejection.
- [ ] **Trust-grade lock pre-onboarding** — a worker mid-phase-3 with
  `onboarding_run.status = 'in_progress'` has gateway dispatch refusing
  any autonomous action; force-flag returns 403.
- [ ] **Mentor concept enforced** — mentor must be (same cohort) AND
  (trust_grade ≥ Tenant) AND (mentoring < 3 active workers) AND
  (not paused). Negative tests for each constraint.

### Pillar 4 — OTJ Loop

- [ ] **Operator correction round-trip in <1 hour** — operator types
  "actually, don't reach out on Friday afternoons" → trust-engine tick
  promotes the rule (or operator force-approves) → next worker turn
  on Friday afternoon respects the rule.
- [ ] **Scope respects boundaries** — a worker-scoped correction
  doesn't propagate to siblings; a cohort-scoped correction reaches
  every member of the cohort; workspace-scoped reaches every worker.
- [ ] **Distilled rule is editable** — operator can see the
  AI-distilled rule before promoting; edit/refine; cancel.
- [ ] **Bad-correction reversal** — operator-rejected corrections
  emit a `LearnedSkillProposalRejected` audit row and never appear in
  any worker's context.

### Pillar 5 — Compliance

- [ ] **Procurement-quality export** — `GET /api/v1/workers/<id>/training-report`
  returns a CSV/JSON containing every audit row, every wiki version
  the worker was running under, every OTJ correction received, every
  skill promoted. Re-runnable: same input → same output.
- [ ] **Wiki diff page is a real semantic diff** — not just line-by-line
  text — markdown headings and bullet structure preserved.
- [ ] **Hash-chain integrity preserved** — adding the 12 new AuditKind
  variants does NOT break the existing `argo-audit::verify::verify_chain`
  invariant. Pre-existing chain tests continue to pass.

---

## 8. Risks and trade-offs

### R1 — Context size

**Risk**: workplace wiki + cohort playbook + worker memory + workflow
context can blow past LLM context limits (Claude Sonnet 200k, Opus
200k). At 5 wiki docs × 2KB each = 10KB; cohort playbook 5KB; memory
3KB; per-task context 15-50KB. Comfortable margin.

**Mitigation**: relevance pruning — at turn-start, run a cheap
"relevance classifier" over wiki+cohort docs against the user's
incoming message; only inject the top-K most relevant docs. Sized as a
follow-up brief if turn-start grows too slow.

### R2 — Wiki edit storm during a turn

**Risk**: admin edits a doc mid-turn; the worker that already booted
this turn has stale context.

**Mitigation**: the injection is cached per-turn-start ONLY (no
mid-turn refresh). The NEXT turn picks up the new content. Documented
behavior; admin sees "in-flight workers will see this on their next
turn" in the editor UI.

### R3 — OTJ correction → skill promotion → infinite-loop

**Risk**: a poorly-worded correction becomes a learned skill that
contradicts the wiki, the wiki gets re-edited to match, drift
compounds.

**Mitigation**: OTJ corrections at workspace scope require explicit
operator review before propagation. Cohort-scope and worker-scope
can auto-promote at trust-engine tick. The wiki + auto-promoted
cohort rules are visible in the worker's debug system message so
drift is detectable.

### R4 — Mentor scarcity in early-stage tenants

**Risk**: a new workspace has no senior workers, so onboarding's
phase 3 (mentor-supervised tasks) can't run.

**Mitigation**: if no mentor exists in the cohort, an operator
fills the role. The mentor role accepts (worker) OR (human admin)
as actor. Document in UI: "no senior worker yet — you'll be the
mentor for the first 3 hires."

### R5 — Brief-set size

22 briefs is the largest single design ask in SKYCOT history. Risk of
mid-flight drift / scope creep.

**Mitigation**: each brief is sized 300-1000 LOC and SCOPE-FROZEN at
brief-claim time. Adjustments need a sibling brief, not a rewrite.
Wave gates: A done before B starts; B done before C; etc. Conductor
keeps a "wave-gate" status row in BUILD_STATUS.md.

---

## 9. Demo scenario (the full vision in action)

**Setup**: brand-new tenant. Operator runs `/onboarding/provision`. 5
seeded wiki docs appear.

**Step 1 — Brand the workspace.**
Operator opens `/settings/knowledge/brand-voice` and edits:
> Voice: confident, warm, never salesy. We never use the words "cheap",
> "guaranteed", "instant". We avoid emojis except 👍 for confirmations.

Edit lands. Audit row: `workplace_knowledge.doc.updated`. Version
becomes v2.

**Step 2 — Set up a cohort.**
Operator creates a `Customer Success` cohort. Opens the playbook tab.
Drafts:
> Voice (override): we're a TRUSTED ADVISOR, not a vendor. Always lead
> with empathy. Sign off with "Speak soon — Mark + the team."

Audit row: `cohort.playbook.updated`.

**Step 3 — Hire 3 workers into Customer Success.**
Each worker spawns `worker_onboarding_run`. Phase 1 starts: the worker
"reads" the workspace wiki + cohort playbook in a structured way. A
small LLM-driven quiz at end of phase 1 confirms they "understood" the
key rules. Operator sees the quiz pass/fail in the dashboard.

Phase 2: each worker watches 3 archived demo runs from `argo session
replay` of similar-role tasks. Audit rows: `phase_completed`.

Phase 3: each worker handles 5 supervised tasks. For each task, the
worker drafts the response; the mentor (operator since no senior worker
exists yet) clicks Approve / Reject / Rework in the dashboard. Rejected
tasks become OTJ corrections.

Phase 4: mentor + operator both sign off → `WorkerOnboardingCompleted`
audit row → autonomy unlocks → trust-engine sees +5 bonus → grade moves
from LearnedUnreviewed to PublishedCommunity on next tick.

**Step 4 — Real customer interaction.**
A customer emails. The on-call worker takes the turn. Her system
prompt visibly includes: `[WORKPLACE — v3]`, `[COHORT: Customer Success
— v1]`, `[WORKER MEMORY — Maya]`. She drafts a reply, lands "trusted
advisor" tone, never uses "cheap", signs off "Speak soon — Mark + the
team."

**Step 5 — A correction.**
Mid-conversation, operator notices Maya replied at 4:55 PM Friday with
a complex proposal. Operator clicks "Correct this" → types: "Don't
send complex proposals on Friday afternoons — wait until Monday
morning." → Submit. Audit row: `otj.correction.submitted`. The
AI-distilled rule:
> When the local time is Friday 14:00-18:00 and the proposed action is
> a multi-step recommendation, defer to Monday 09:00.

Operator picks scope `cohort`. Trust engine ticks. The rule is
promoted to a `cohort_playbook_section`. Next turn from ANY Customer
Success worker on a Friday afternoon respects the rule.

**Step 6 — Audit.**
3 months later, in a procurement review, a buyer asks: "Show me
every change to your brand voice + every correction Maya received."
Operator opens `/workers/maya/training` → real timeline → every
revision id is a clickable diff. Buyer is convinced. SKYCOT wins
the deal.

That is the full vision. Nothing in this design is "we'll figure it
out at runtime." Every claim in the demo above maps to a concrete
table + endpoint + UI surface + audit row.

---

## 10. What to do next

1. **Read this doc end-to-end** — confirm the vision matches yours.
2. **Approve or amend the 22-brief sequence** — particularly the
   wave gates.
3. **Approve the demo scenario** — that's the north-star for QA.
4. **Tell the conductor "fan out Wave A, ~6 briefs at once, claude
   pool"** — the briefs will be authored fresh against the latest
   `origin/main` and inserted into `BUILD_STATUS.md` at claim time.
5. **Decide the launch window** — 12-14 weeks from Wave-A kickoff
   is the conservative estimate; aggressive parallelism can shave 2-3
   weeks.

Open this doc:

`open ~/code/argo/docs/skycot/design/workplace-onboarding-architecture.md`
