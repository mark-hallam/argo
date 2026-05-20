# Wave-13 Audit — Step-Ordering, Branch & Sub-Workflow Behaviour

**Date**: 2026-05-21
**Auditor**: Wave-13 audit agent #4
**Scope**: `/workflows/<v>/<n>/build` canvas — Decision branching, sub-workflows, step reordering, mid-flow insertion.
**Files audited**:
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (1120 LOC, all current on `origin/main` head `cb33ab6f`)
- `crates/argo-workflow/src/dag.rs` (backend DAG primitives)
- `docs/build-plan/streams/web-workflow-builder-multi-branch-rendering.md`
- `docs/build-plan/streams/web-workflow-builder-sub-workflow-node.md`
- `docs/build-plan/streams/web-workflow-builder-node-palette-sidebar.md`
- PRs #1881, #1883 (both IN_REVIEW, CI red, not in main)

---

## 1. Decision-node rendering — is it distinct? Are branch labels visible?

**Verdict: NO — Decision steps render identically to every other linear step. There is no branch surface today.**

What exists on `origin/main`:
- `StepKind` enum at `page.tsx:71` lists `decision` as one of nine kinds.
- `STEP_KIND_META` at `page.tsx:183` gives Decision a unique colour (`#6366f1` indigo) and a `GitBranch` lucide icon. **That is the only visual distinction.**
- `SkycotStepNode` (`page.tsx:215-361`) renders **one source handle on the Bottom** (`Position.Bottom`, `page.tsx:334`) regardless of `step.kind`. No second handle for the "false" branch.
- `stepsToFlow()` (`page.tsx:370-434`) iterates `steps` linearly, emitting one solid purple edge `prev → next`. **No branching layout logic.** The function only has special-casing for `loop` (renders `step.body[]` indented at `COL_BASE + COL_INDENT = 190` and adds a dashed return edge).

Concrete seed evidence:
- `gtm-engineer/post_listing` step `s3` ("Branch on validation") and `sales/qualify_lead` step `s3` ("Branch on BANT score") have descriptions like `"Below floor → manager review. Otherwise continue."` and `"≥75 = demo. 50-74 = nurture. <50 = decline."` — but the data model has **no branch payload**. There is no `WorkflowStep.branch`, no per-edge condition field, no true/false outcome label. The narrative lives only inside the human-readable `description` string. The canvas treats decision steps as pass-through.

Wave-1 brief status:
- `web-workflow-builder-multi-branch-rendering` — **IN_REVIEW, not landed**. PR #1881 introduces `branch?: DecisionBranch` with `joinAt`, renders amber `Position.Right` handle for false path, lays branch column at `x=460` with dashed amber rejoin edge.
- All 15 CI checks on #1881 are **FAILURE** (`fmt`, `clippy`, `test (ubuntu/macos/windows)`, `loc-budget`, `prepared-statements`, `deny`, `audit-sink-tenant-guard`, `e2e-onboarding`, etc.). Likely cascade from a single root cause (probably the no-conventional-commit-prefix or a wholly Rust-fail-on-web-PR issue), but until it merges, decision branching does not exist for any buyer who clicks the seeded workflows.

---

## 2. `stepsToFlow()` and column layout

**Verdict: Single-column with loop-indent only.**

Constants at `page.tsx:363-365`:
```
const ROW_GAP = 200;
const COL_BASE = 100;
const COL_INDENT = 90;
```

Layout behaviour:
- Every top-level step emitted at `x = 100, y = 60 + i*200` — strict vertical stack.
- Loop steps re-emit `body[]` children at `x = 190` (COL_BASE + COL_INDENT). This is the **only** multi-column case.
- Edges always go `Position.Bottom → Position.Top` (handle positions hardcoded inside `SkycotStepNode`). React Flow's auto-routing draws a bent vertical line; with `animated: true` the purple stroke dashes flow.
- No horizontal fan-out, no fork-and-join, no swimlanes. The mini-map at `page.tsx:1006` reflects exactly what the canvas shows — a single column of pills.

When #1881 lands, decision steps gain `Position.Right` handle and a second column at `x = 460`. Until then, **the entire visual differentiator vs. n8n/UIpath/Replit is missing**.

---

## 3. Sub-workflow node — clickable to enter sub-workflow?

**Verdict: NO sub-workflow type exists in main today.**

Findings:
- `StepKind` enum (`page.tsx:71-80`) has 9 variants. `sub_workflow` is NOT one of them.
- No occurrence of "sub_workflow" or "SubWorkflow" in `/web/**`.
- Backend `crates/argo-workflow/src/dag.rs` `TaskNode` (lines 49-83) has `worker`, `persona`, `inputs`, `outputs`, `depends_on`, `cache_key` — but no field for "call another workflow". The Rust DAG is **flat** — there is no concept of a workflow invoking another workflow.
- Brief copy on `web-workflow-builder-sub-workflow-node.md:5` claims `"Backend already supports sub-workflow invocation."` — **this is incorrect.** No Rust crate has a `sub_workflow`/`SubWorkflow` symbol. Either the runner has an undocumented invocation path elsewhere (worth a one-tick follow-up) or the brief is wishful.
- PR #1883 IN_REVIEW — adds `SubWorkflowNode` with stacked-card silhouette, "Expand →" opens sub-workflow `/build` route in a new tab via `<Link target="_blank">`. CI also FAILURE across the same 14 checks.

Operator question implicit in the audit: *"Sub-workflow node — clickable to enter sub-workflow?"* — On main: no node, no click target. On #1883 when it lands: yes, opens the sub-workflow's own builder in a new tab. **There is no inline expand/collapse** — the user always context-switches to a new tab, losing canvas state.

---

## 4. Step reordering — can the user drag a step to change order? Does the edge re-route?

**Verdict: Drag-to-reorder is functionally broken. User can drag, nothing useful happens.**

Critical gap in `page.tsx:993-1016`:
```tsx
<ReactFlow
  defaultNodes={initial.nodes}
  defaultEdges={initial.edges}
  ...
/>
```
- `defaultNodes` / `defaultEdges` (not `nodes` / `edges`) — React Flow runs in **uncontrolled mode**. Drag updates live inside the library's internal state.
- **No `onNodesChange` handler. No `onEdgesChange` handler. No `onNodeDragStop` handler.** Drag deltas are not propagated to the `WorkflowSpec`. The `WorkflowStep[]` ordering used by `stepsToFlow()` never updates.
- The user can drag a node freely on the canvas. The card moves visually. The edge endpoints follow the node (React Flow built-in). But: order is not changed in the underlying spec, drag position is not persisted, **a page refresh resets the layout to the seed positions**.
- There is **no way to reorder execution order** via the canvas. Even if reordering were persisted, edges in `stepsToFlow()` are computed from array order — moving a node to a different `(x,y)` doesn't move it in the array. The user has no UI affordance ("move up/down" buttons, drag-handle in the side panel, keyboard shortcut) to change the actual sequence.

Cross-reference: a buyer demo where the operator says "let me drag s3 above s2" — the operator can drag the card, the edge will visually warp into a knot, but s2 will still execute before s3 because `WorkflowStep.id` order is fixed in the seed dictionary.

---

## 5. Add step in the middle — where does the new node land? Connections preserved?

**Verdict: You cannot add a step at all. The "Add step" palette is decorative.**

Trace:
- The pink-gradient FAB at `page.tsx:1017-1024` toggles `paletteOpen`.
- `AddStepPalette` renders at `page.tsx:449-559`. Every step-kind button (`<button key={kind}>` at lines 512-553) has **mouseEnter / mouseLeave hover effects, but no `onClick` handler**. Clicking does nothing.
- Inline comment at line 450 admits it: *"Per the brief, the palette is a stub: insertion wiring lands in the node-palette-sidebar brief."*
- `web-workflow-builder-node-palette-sidebar` brief — **TODO, not yet claimed.** No PR.
- The "Add step" button on the `WorkflowBuilderEmptyState` fallback (`page.tsx:666-671`) fires a `toast.info("Add-step palette opens in the live builder")` and does not insert anything.
- AI prompt bar: `handleProposeChange` (`page.tsx:675-696`) was rewritten in Wave-6 to be an honest no-op. `toast.loading` → `toast.message("Preview build — draft not persisted")` after 1400ms. **No canvas mutation, no step insertion, no AI call.**

Net: the user has **three visible affordances to add a step** (FAB → palette, AI prompt, suggestion chips "Add a step" / "Remove gate"). **Zero of the three actually insert a step.** A buyer who clicks any of them in a demo will see toasts that explicitly say "preview build, not wired".

---

## Top 3 step-control gaps (ranked by demo-day buyer-catchability)

### Gap 1 (BLOCKER for any branching demo): Decision nodes render as linear pass-throughs

Highest-stakes gap. Branching is the differentiator between SKYCOT-as-workflow and SKYCOT-as-checklist. Currently the seeded `post_listing` and `qualify_lead` workflows — both of which exist precisely to showcase branching — render the Decision step exactly like a Skill step. A buyer says "what happens if validation fails?" and the canvas has no answer.

**Fix**: get PR #1881 green and merged. CI shows it's stuck on a cascade of red checks across 14 jobs; investigate whether they're real failures or a missing PR label / CI permissions issue. Until #1881 lands, the demo claim *"draw branches like n8n"* is unsupported by the live canvas.

### Gap 2 (UX trust killer): Add-step palette is fully decorative

The FAB is the most visually prominent piece of furniture on the page (pink-gradient, bottom-right, shadow at `0 12px 30px -8px rgba(139, 92, 246, 0.5)`). Clicking opens a beautifully-styled menu where every button is dead. This is the kind of detail a power-user catches in 8 seconds. The comment on line 450 admits it.

**Fix**: either wire one path end-to-end (e.g. "Add Skill" inserts a new `skill_invoke` step at array end, sets it as selectedNode, persists in client-side state for the session) or hide the FAB until the node-palette-sidebar brief lands. The current state — visible-but-broken — is worse than not-visible.

### Gap 3 (silent-failure trap): Drag reorder doesn't persist, doesn't reorder

Because `<ReactFlow>` runs in uncontrolled mode (`defaultNodes` not `nodes`), the canvas LOOKS interactive. Drag a node, it moves. Drag an edge, it bends. Refresh, everything snaps back to seed. There's no visible error — no toast, no console warning, no greyed-out drag cursor — just silent loss of work. For a power-user evaluating SKYCOT against n8n where reorder-via-drag is table stakes, this fails the *"does this thing actually let me build a workflow"* sniff test.

**Fix (smallest viable patch)**: switch to controlled mode (`useNodesState` / `useEdgesState` hooks from `@xyflow/react`) wired through `onNodesChange` / `onEdgesChange`. Persist positions to client-side state at minimum so refresh doesn't blow them away. The actual *reorder the WorkflowStep array based on (y) position* logic is a follow-up brief; even just persisting visual positions across refresh is a meaningful improvement.

---

## Cross-cutting observations

- **Backend gap**: The Rust workflow DAG (`crates/argo-workflow/src/dag.rs`) has `Trigger::Success | Always | Failure` per edge — this is the closest thing to a branching primitive in the runtime. Per the rustdoc comment at line 33: *"v1 ships success only; always and failure follow once the executor learns to branch on outcome."* So the backend ALSO does not yet support branching — the web brief #1881 is purely visual scaffolding ahead of runtime support. When a buyer drills in *"so when the BANT score is 50-74 what actually happens?"* the answer is: nothing different, both paths take the success edge.

- **`web-workflow-builder-sub-workflow-node.md:5`** asserts *"Backend already supports sub-workflow invocation"* — I found no evidence of this in `/crates/**` (zero hits for `sub_workflow` / `SubWorkflow`). Worth a one-tick verification before #1883 lands — if the brief is wrong, the merged sub-workflow nodes will be entirely client-side fiction.

- **No reorder UI alternative**: even if drag-to-reorder weren't broken, there's no `↑ / ↓` move buttons on the side panel, no keyboard `Cmd+Up / Cmd+Down`, no "edit YAML" escape hatch. The user has zero ways to change step order today.

- **No delete UI**: there's an Edit-step button in the side panel (`page.tsx:1077-1100`) but no delete. Once a step exists, it's permanent for the session.

- **`onNodeKeyDown` defined but not wired**: `page.tsx:633-639` declares `onNodeKeyDown` for keyboard a11y (Enter/Space opens side panel), but it's never passed to `<ReactFlow>`. The `useCallback` is dead code on main. Worth checking whether Wave-7 a11y audit flagged this (the comment claims it fixed the WCAG 2.1.1 issue, but the handler doesn't reach the component).

- **Loop node has the only "real" advanced layout**: the dashed return edge from last body step back to loop header (`page.tsx:428-429`) is genuinely interesting and uses two distinct edge styles (`linkSolid`/`linkReturn`). This is the template for how branching SHOULD look — solid for forward flow, dashed for "this is special structural meaning". If #1881's design borrows the same visual vocabulary, the canvas will feel coherent.

---

## Summary verdicts

| Question | Verdict |
| --- | --- |
| Decision node distinct? Branch labels visible? | **NO** — identical to Skill step. Only icon + color differ. |
| `stepsToFlow()` lays branches in columns? | **NO** — single column. Only loop-body indents. Wave-1 brief IN_REVIEW (#1881), CI red. |
| Sub-workflow node clickable to enter? | **NO** — no sub-workflow type exists. PR #1883 IN_REVIEW, CI red. |
| Drag to reorder steps + edges re-route? | **NO** — uncontrolled React Flow, no `onNodesChange`, no persist. Visual drag is lossy. |
| Add step in middle, connections preserved? | **NO** — Add-step palette has no click handlers. AI prompt is a no-op. |

**Step-control UX score: 1/10.** The canvas renders. It pans. It zooms. The mini-map works. Everything else that defines "a workflow builder" is either stub or not-yet-landed.
