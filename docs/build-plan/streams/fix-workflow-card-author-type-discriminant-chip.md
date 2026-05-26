# fix-workflow-card-author-type-discriminant-chip

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflow-card-author-type-discriminant-chip`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-4-and-5-claim-4-and-5.md` F5 [P1].

🟠 **P1 — Workflow card "BY MARIAH" / "BY PRIYA" copy doesn't distinguish whether the author is a worker, a human, or a template-pack. Scene 5 voiceover "workflows can be edited by the AI teammate themself" has no visual anchor on the card.**

## Root cause

`web/lib/sample-workflows.ts:34-50` defines `WorkflowSummary.lastEditor: string` — just a name. `web/app/(app)/workflows/page.tsx:267` renders it as "by Mariah" / "by Priya" / etc. No icon, no type chip, no `authorType` discriminant.

Workers (Mariah / Peta / Jamal / Nori / Maya / Priya) and humans (the operator + team members) share name space — there's no way to tell from the card whether the listing was authored by an AI teammate or a person.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F5 [P1] — Workflow card author chip doesn't distinguish worker / human / template-pack

## Fix recipe

1. **Extend `WorkflowSummary` type** at `web/lib/sample-workflows.ts:34-50`:
   ```ts
   export type WorkflowSummary = {
     // ... existing fields ...
     lastEditor: string;
     authorType: "worker" | "human" | "template_pack";
     authorWorkerId?: string;  // optional, for click-through to /workers/<id>
   };
   ```

2. **Populate `authorType` per existing fixture**. For each row in `SAMPLE_WORKFLOW_SUMMARIES`, decide based on `lastEditor` name + canonical roster:
   - Names in `[Mariah, Maya, Peta, Priya, Jamal, Nori, Patrick]` → `authorType: "worker"` + populate `authorWorkerId`
   - "SKYCOT" / "system" → `authorType: "template_pack"`
   - Anything else → `authorType: "human"`

3. **Render an icon next to the name** at `web/app/(app)/workflows/page.tsx:267`:
   ```tsx
   <span className="author-chip">
     {authorType === "worker" && <Bot size={12} aria-label="AI teammate" />}
     {authorType === "human" && <User size={12} aria-label="Human teammate" />}
     {authorType === "template_pack" && <Layers size={12} aria-label="Template" />}
     by {lastEditor}
   </span>
   ```

4. **Make the chip clickable** when `authorType === "worker"` — link to `/workers/{authorWorkerId}`.

5. **Update gateway response** at `crates/argo-cli/src/handlers/gateway/routes_workflows_list.rs` to include `author_type` + `author_worker_id` in the workflow row response (so the real-data path also gets the distinguisher, not just the fixture).

6. **Add a unit test** at `web/components/skycot/workflows/WorkflowCard.test.tsx`:
   - Worker-authored card shows Bot icon
   - Human-authored card shows User icon
   - Template-pack card shows Layers icon
   - Clicking worker author navigates to `/workers/{id}`

## Acceptance criteria

- [ ] Each workflow card shows a small icon distinguishing author type
- [ ] Worker-authored cards show Bot icon and are clickable to `/workers/{id}`
- [ ] Template-pack cards show Layers icon
- [ ] Type stays accurate when gateway returns real (non-fixture) workflows
- [ ] Unit test passes

## May-touch

- `web/lib/sample-workflows.ts` (extend schema)
- `web/app/(app)/workflows/page.tsx` (render chip)
- `web/components/skycot/workflows/WorkflowCard.tsx` (extract or update component)
- `crates/argo-cli/src/handlers/gateway/routes_workflows_list.rs` (include author_type in response)
- `crates/argo-workflow/src/types.rs` (WorkflowDefinition schema if needed)
- `web/components/skycot/workflows/WorkflowCard.test.tsx` (NEW)

## Out of scope

- Author-type filtering / facet on the page (separate UX brief)
- Workflow-permission UX (separate concern)
- The lastEditor canonicalisation (covered by `fix-demo-seed-cli-for-real-roster`)

## Effort

~80-200 LOC + 1 test. ~3-4 hours. P1 because Scene 5's "workers can edit workflows" narration needs the visual anchor.
