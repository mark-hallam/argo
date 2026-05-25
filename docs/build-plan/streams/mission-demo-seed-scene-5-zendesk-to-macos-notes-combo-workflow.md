# mission-demo-seed-scene-5-zendesk-to-macos-notes-combo-workflow

**Owner**: claude.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-seed-scene-5-zendesk-to-macos-notes-combo-workflow`.
**Depends on**: `mission-demo-seed-crate-scaffold` (#2272 IN_REVIEW).

🔴 **P0 DEMO-GATE — Scene 5 unrecordable without this workflow in the seed.**

`v1-mission-mvp-runbook.md` Scene 5 narrates the MCP-plus-computer-use combo: an agent that calls `zendesk:search_tickets` (MCP connector — well-known API) AND drives `macos_notes:create_note` (computer-use — desktop control). The seed currently doesn't contain a workflow with this exact combo. Scene 5 is the only scene that visually proves MISSION claim 5 ("any task a human can do on a PC" — TWO execution backends).

Per the POC/investor demo positioning doc, this IS a blocker.

## Scope

Add ONE workflow to the demo seed:

- **Name**: `customer_research_followup` (or similar — must read naturally in the talk track).
- **Step 1**: `zendesk:search_tickets` — MCP call against the existing `argo-mcp-zendesk` connector (real upstream API; sandbox account). Query example: tickets opened in the last 7 days, status open, priority high.
- **Step 2**: `macos_notes:create_note` — computer-use call on the operator's macOS via `argo-executor-computer-use-macos` (the v1-hardened backend). Creates a Notes entry with the search results from Step 1 templated in.
- **Owner / author chip**: one of the canon roster — Mariah, Peta, Priya, Jamal, or Nori (NOT "Rio" or any non-canon name). Pick whichever has the most narrative fit; default to Priya (Customer Success persona).

The seed must:
- Insert the workflow row into the `workflow` table via the same code path real workflow authoring uses (no INSERT INTO theater).
- Ensure the workflow shows up in `GET /api/v1/workflows?limit=10` so `/workflows` renders it as a card.
- Ensure the workflow's author chip on the card shows the canon worker name (Scene 5 narration: "Priya pulls the top tickets and drafts the followup notes").

## Acceptance criteria

- [ ] After `argo demo seed --tenant <demo>`, `curl http://localhost:8088/api/v1/workflows?limit=20 | jq '.workflows[] | .name'` includes `customer_research_followup` (or equivalent).
- [ ] That workflow's `steps` array has length 2, with step[0].kind `mcp_call` and step[1].kind `computer_use_call` (or whatever the executor discriminator is in the workflow schema).
- [ ] Step 1 step config references `zendesk:search_tickets` (verify via jq).
- [ ] Step 2 step config references `macos_notes:create_note` (verify via jq).
- [ ] Author chip on `/workflows` card shows a canon roster name (Mariah/Peta/Priya/Jamal/Nori) — no "Rio".
- [ ] Cross-tenant invisibility: different tenant sees 0 workflows after the same seed.
- [ ] PR body includes the full jq output of the new workflow row + a `curl` of the `/workflows` page showing the card.

## May-touch

- `crates/argo-demo-seed/src/**` — extend with a workflow-catalog branch
- `crates/argo-cli/src/handlers/demo_seed.rs` — wire the new branch
- Possibly `verticals/<vertical-id>/workflows/` if workflow definitions live in vertical packs (verify by `find verticals -name '*.workflow.toml'`)
- This brief file (status updates)

## Out of scope

- Re-implementing the macOS-Notes computer-use macro (it already exists — see `argo-executor-computer-use-macos` round 4 audit at 37/37 lib tests pass)
- Re-implementing the zendesk connector (it exists — see `eval-mcp-connector-real-vs-stub-inventory` 29/29 REAL)
- Running the workflow at seed time — just inserting the definition; runtime execution is Scene 5's recording moment

## Effort

~80-150 LOC + 1 workflow manifest TOML; ~45-60 min.
