# Stream P2-10 — conversational "add a responsibility in plain English"

**Owner**: claude.
**Depends on**: mission-pm-workflow-persistence-schema MERGED, mission-pm-workflow-edit-api MERGED.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-conversational-responsibility`.
**Source**: skycot-pm-worker REPORT.md §7 + §11 P2 #10. Designed in `docs/skycot/ux-research/journey-delegate-ongoing-work.md`, not yet built.

## Scope

A delegator types "every Friday, look at this week's tickets, cluster into themes, draft the all-hands update, run it past head of product, post to Notion" into a chat surface on their worker's profile page. The teammate replies with a proposed workflow TOML (steps + skills + escalation gates), the operator approves (with edits), and on accept the workflow is persisted via the P1 #6 API.

This brief lands the **author surface + propose flow**. The actual LLM call that translates plain English → workflow TOML uses the existing agent-runner with a NEW skill `propose_workflow_from_plain_english` (added in this brief). The skill prompt is anchored on the workflow schema doc-comment (`crates/argo-vertical/src/workflow.rs:1-50`).

This is a meaty brief. **Splittable**: into `-skill` (the new skill + prompt + eval) + `-ui` (the chat surface). If split, `-skill` claims first.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `skills/library/propose_workflow_from_plain_english.skill.md` (NEW) — the prompt. Tagged with the workflow schema, examples drawn from the existing `verticals/*/workflows/*.workflow.toml` files. Frontmatter: source = `system:bundled`.
- `crates/argo-skill/eval/propose_workflow_from_plain_english.eval.jsonl` (NEW) — 6 eval cases covering happy path + adversarial inputs (vague request, conflicting steps, request that needs escalation gate).
- `web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx` (NEW) — the author surface. Chat-style with one input box and a streaming model response showing the proposed TOML. Approve/Edit/Cancel actions.
- `web/components/skycot/ProposeWorkflowChat.tsx` (NEW) — the chat component. ≤ 400 LOC.
- `web/components/skycot/WorkflowTomlPreview.tsx` (NEW) — pretty-print TOML with syntax highlighting (reuse existing prism setup if present, else minimal `<pre>`).
- `web/app/api/workflows/propose/route.ts` (NEW) — server route that calls the gateway with the user's bearer token, invoking a one-shot agent turn with the new skill attached. Streams the assistant response back.
- `web/components/skycot/ProposeWorkflowChat.test.tsx` (NEW).

## Files you must NOT touch

- Any backend Rust code — the gateway route surface from P1 #6 is the only Rust touchpoint and is locked.
- `crates/argo-skill/src/store.rs` — installing the bundled skill is via the existing skill-library install path; no code changes.
- Worker profile parent page — link in from there is fine, but no other edits.

## Acceptance criteria

- [ ] New skill `propose_workflow_from_plain_english` installs into the library on first run via the existing skill-bundle mechanism.
- [ ] Skill body anchors on the workflow schema (`schema_version`, `[workflow].name`, `[[step]]` with `kind` ∈ {tool_call, skill_invoke, prompt}) and produces valid TOML 80%+ of the time on the eval set.
- [ ] Eval suite has 6 cases; passes ≥ 4/6 in CI (judged via the existing eval-judge pipeline).
- [ ] `/workers/<wid>/responsibilities/new` renders the chat surface; submitting the input streams a response with the proposed TOML.
- [ ] Approve button POSTs to the P1 #6 `POST /verticals/<v>/workflows/<n>` route with the user's principal.
- [ ] Edit button drops the user into a textarea pre-filled with the proposed TOML.
- [ ] Cancel button drops back to the worker profile.
- [ ] The streamed response is rendered with syntax highlighting and a copy-to-clipboard button.
- [ ] A11y: chat input is `aria-label`-ed; streamed output is `aria-live="polite"`; approve button shows pending state during POST.
- [ ] `pnpm lint && pnpm test && pnpm build` clean in `web/`.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-conversational-responsibility`
- PR title: `web: conversational workflow proposal surface (mission-pm-conversational-responsibility)`

## Notes for the implementer

- The skill is bundled at the **library tier**, not workspace — so every tenant gets it on first run. Source value `system:bundled` lands `TrustGrade::SystemBundled`.
- The skill body should include 2 fully-worked examples (a triage workflow + a content-draft workflow) verbatim from `verticals/product-manager/` so the model has concrete anchors.
- The streaming route does NOT touch `workflow_definition` directly — it just streams a proposal. The PUT to persist happens after operator approval, through the P1 #6 route. This separation is load-bearing — we don't want a model-only POST landing in production.
- Use the existing `useTransport` / EventSource pattern in `web/lib/` (or equivalent) for the streaming. Don't introduce a new SSE library.
- Honest error states: if the proposed TOML fails `parse_workflow` on the server (P1 #6 returns 400), surface the parser error in the chat surface inline and let the user edit.

## Out of scope

- Multi-turn refinement ("can you adjust step 3 to require approval?") — separate brief; v1 is one-shot.
- Voice input — separate brief.
- Auto-test the proposed workflow via dry-run before approve — separate brief; v1 is human-judged.
- Cross-worker workflows (the chat invokes another worker's expertise) — separate brief.
